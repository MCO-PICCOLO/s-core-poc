/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use crate::grpc::sender;
use common::external::timpani::fault_service_server::FaultService;
use common::external::timpani::{FaultInfo, FaultType, Response as TimpaniResponse};
// artifact imports removed — patching is done via serde_yaml::Value directly
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

/// Per-workload: the `cpu_affinity` bitmask of the last deadline-miss we acted on.
///
/// Decision rules (evaluated once per incoming fault under the lock):
///   • `fault_cpu == last_acted_cpu`  → **ignore** — the workload is still on the
///     CPU we already rescheduled away from; the fault is stale or the workload
///     has not been moved yet. No repeated action.
///   • `fault_cpu != last_acted_cpu`  → **act** — pick a new CPU randomly from
///     the pool reported by Timpani-O (`FaultInfo.num_cpus`), excluding the
///     faulting CPU; build the reschedule YAML in memory; apply it to the DB;
///     trigger the action controller.  Record `fault_cpu` as `last_acted_cpu`
///     so the next arrival from the same CPU is suppressed.
static WORKLOAD_LAST_CPU: OnceLock<Mutex<HashMap<String, u64>>> = OnceLock::new();

fn workload_last_cpu() -> &'static Mutex<HashMap<String, u64>> {
    WORKLOAD_LAST_CPU.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Return the list of CPU indices that are set in `mask`.
fn cpu_bits(mask: u64) -> Vec<u32> {
    (0..64).filter(|&i| mask & (1u64 << i) != 0).collect()
}

/// Return all k-combinations of elements from `pool`.
fn combinations(pool: &[u32], k: usize) -> Vec<Vec<u32>> {
    if k == 0 {
        return vec![vec![]];
    }
    if pool.len() < k {
        return vec![];
    }
    let mut result = Vec::new();
    for i in 0..=(pool.len() - k) {
        for mut combo in combinations(&pool[i + 1..], k - 1) {
            combo.insert(0, pool[i]);
            result.push(combo);
        }
    }
    result
}

/// Pick a new CPU-affinity bitmask with the **same number of CPUs** as
/// `fault_cpu_mask` but a different combination, chosen from `available_cpu_mask`.
///
/// `available_cpu_mask` encodes the exact CPU indices available on the node
/// (e.g. CPUs {0,3,4,6} → 0x59).  The pool is built from the set bits of this
/// mask so the result is always a valid index on the real hardware.
///
/// When `fault_cpu_mask == 0` picks 1 CPU from the available pool.
/// Falls back to `fault_cpu_mask` when no alternative combination exists.
fn pick_new_cpu_set(available_cpu_mask: u64, fault_cpu_mask: u64) -> u64 {
    // Build pool from actual available CPU indices.
    let pool: Vec<u32> = cpu_bits(available_cpu_mask);
    if pool.is_empty() {
        return fault_cpu_mask;
    }
    // Preserve the same count of CPUs as the faulting affinity (min 1).
    let n_assigned = if fault_cpu_mask == 0 {
        1
    } else {
        fault_cpu_mask.count_ones() as usize
    };
    let candidates: Vec<u64> = combinations(&pool, n_assigned)
        .into_iter()
        .map(|combo| combo.iter().fold(0u64, |acc, &c| acc | (1u64 << c)))
        .filter(|&mask| mask != fault_cpu_mask)
        .collect();
    if candidates.is_empty() {
        return fault_cpu_mask;
    }
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    candidates[seed as usize % candidates.len()]
}

/// Path to the reschedule template YAML file.
const RESCHEDULE_YAML_PATH: &str =
    "/home/acrn/new_ak/vso_score/s-core-poc/Node1/pullpiri/examples/resources/reschedule_sea.yaml";

/// Read `reschedule_sea.yaml`, patch the four dynamic fields in memory, write
/// every document to the DB, and return the Scenario name.
///
/// Fields patched per document kind:
///   Schedule : metadata.name        → `schedule_name`  (= original workload_id)
///              spec[0].name         → `task_name`
///              spec[0].cpu_affinity → `new_cpu_affinity`
///              spec[0].node_id      → `node_id`
///   Scenario : metadata.name        → `base_id`
///              spec.target          → `base_id`
///   Package  : metadata.name        → `base_id`
///              spec.schedule        → `schedule_name`
///              spec.models[0].name  → `task_name`
///
/// `schedule_name` equals the original workload_id received from Timpani-O so
/// that each reschedule replaces the SAME key in Timpani-O's sched_info_map_
/// instead of accumulating new "-schedule"-suffixed entries.
async fn apply_reschedule_yaml_patched(
    schedule_name: &str,
    base_id: &str,
    task_name: &str,
    new_cpu_affinity: u64,
    node_id: &str,
) -> Result<String, String> {
    let content = tokio::fs::read_to_string(RESCHEDULE_YAML_PATH)
        .await
        .map_err(|e| format!("Failed to read '{}': {}", RESCHEDULE_YAML_PATH, e))?;

    let mut scenario_name: Option<String> = None;

    for raw_doc in content.split("---") {
        let raw_doc = raw_doc.trim();
        if raw_doc.is_empty() {
            continue;
        }

        let mut value: serde_yaml::Value = serde_yaml::from_str(raw_doc)
            .map_err(|e| format!("Failed to parse YAML document from '{}': {}", RESCHEDULE_YAML_PATH, e))?;

        let kind = match value.get("kind").and_then(|v| v.as_str()) {
            Some(k) => k.to_string(),
            None => continue,
        };

        // ── Patch dynamic fields ───────────────────────────────────────────
        match kind.as_str() {
            "Schedule" => {
                value["metadata"]["name"] = serde_yaml::Value::String(schedule_name.to_string());
                if let Some(spec) = value.get_mut("spec").and_then(|v| v.as_sequence_mut()) {
                    if let Some(first) = spec.first_mut() {
                        first["name"] = serde_yaml::Value::String(task_name.to_string());
                        first["cpu_affinity"] =
                            serde_yaml::Value::Number(new_cpu_affinity.into());
                        if !node_id.is_empty() {
                            first["node_id"] = serde_yaml::Value::String(node_id.to_string());
                        }
                    }
                }
            }
            "Scenario" => {
                value["metadata"]["name"] = serde_yaml::Value::String(base_id.to_string());
                if let Some(spec) = value.get_mut("spec") {
                    spec["target"] = serde_yaml::Value::String(base_id.to_string());
                }
                scenario_name = Some(base_id.to_string());
            }
            "Package" => {
                value["metadata"]["name"] = serde_yaml::Value::String(base_id.to_string());
                if let Some(spec) = value.get_mut("spec") {
                    spec["schedule"] = serde_yaml::Value::String(schedule_name.to_string());
                    if let Some(models) = spec
                        .get_mut("models")
                        .and_then(|v| v.as_sequence_mut())
                    {
                        if let Some(first) = models.first_mut() {
                            first["name"] = serde_yaml::Value::String(task_name.to_string());
                        }
                    }
                }
            }
            other => {
                println!("Skipping unknown kind '{}' in reschedule YAML", other);
                continue;
            }
        }

        // ── Write patched document directly to DB ──────────────────────────
        let db_key = match kind.as_str() {
            "Schedule" => format!("Schedule/{}", schedule_name),
            "Scenario" => format!("Scenario/{}", base_id),
            "Package"  => format!("Package/{}", base_id),
            _          => continue,
        };

        let artifact_str = serde_yaml::to_string(&value)
            .map_err(|e| format!("Failed to serialize '{}': {}", db_key, e))?;

        common::persistency::put(&db_key, &artifact_str)
            .await
            .map_err(|e| format!("Failed to write '{}' to DB: {}", db_key, e))?;

        println!(
            "Patched {} \u{2192} cpu_affinity={}, task='{}', workload='{}'",
            db_key, new_cpu_affinity, task_name, base_id
        );
    }

    scenario_name
        .ok_or_else(|| format!("No Scenario document found in '{}'", RESCHEDULE_YAML_PATH))
}

#[derive(Default)]
pub struct TimpaniReceiver {}

#[tonic::async_trait]
impl FaultService for TimpaniReceiver {
    async fn notify_fault(
        &self,
        info: Request<FaultInfo>,
    ) -> Result<Response<TimpaniResponse>, Status> {
        let info = info.into_inner();
        println!("Received fault notification: {:?}", info);

        if info.r#type == FaultType::Dmiss as i32 {
            let workload_id = info.workload_id.clone();
            let task_name   = info.task_name.clone();
            let node_id     = info.node_id.clone();

            // Strip any trailing "-schedule" chain — the action-controller echoes
            // the Schedule name back as workload_id, which would otherwise keep
            // accumulating "-schedule" suffixes on every reschedule.
            let base_id: String = workload_id.trim_end_matches("-schedule").to_string();

            // cpu_affinity bitmask reported by Timpani-O (e.g. cpu 2 → 0b100 = 4).
            let fault_cpu = info.cpu_affinity;

            // Number of CPUs available on the system.
            // Timpani-O populates num_cpus; default to 4 if the field is absent/zero.
            let num_cpus = if info.num_cpus == 0 { 4 } else { info.num_cpus };

            // Bitmask of available CPU indices on the node (e.g. {0,3,4,6} → 0x59).
            // Fall back to a contiguous mask of num_cpus when not provided.
            let available_cpu_mask = if info.available_cpu_mask == 0 {
                (1u64 << num_cpus) - 1
            } else {
                info.available_cpu_mask
            };

            // Use a per-node key so identical workload names on different nodes
            // are tracked independently. If node_id is empty, fall back to base_id.
            let state_key = if node_id.is_empty() {
                base_id.clone()
            } else {
                format!("{}:{}", base_id, node_id)
            };

            // ── Guard: act only when the faulting CPU has changed ────────────────
            let should_act = {
                let mut state = workload_last_cpu().lock().await;
                let last = state.entry(state_key.clone()).or_insert(u64::MAX);
                if *last == fault_cpu {
                    println!(
                        "[{}] DMISS on cpu_affinity=0x{:x} — same as last acted CPU, ignoring",
                        state_key, fault_cpu
                    );
                    false
                } else {
                    *last = fault_cpu;
                    true
                }
            }; // lock released

            if !should_act {
                return Ok(Response::new(TimpaniResponse { status: 0 }));
            }

            // ── Choose a new CPU set (same count, different combination) ─────
            let new_cpu_affinity = pick_new_cpu_set(available_cpu_mask, fault_cpu);

            println!(
                "[{}] DMISS cpu_affinity=0x{:x} (CPUs {:?}, count={}), \
                 rescheduling to 0x{:x} (CPUs {:?}), available=0x{:x}",
                state_key, fault_cpu, cpu_bits(fault_cpu), fault_cpu.count_ones(),
                new_cpu_affinity, cpu_bits(new_cpu_affinity), available_cpu_mask
            );

            // ── Spawn async task ────────────────────────────────────────────────
            let schedule_name = workload_id.clone(); // original key in Timpani-O's map
            let wid = base_id.clone();               // stable name for Scenario/Package
            let task_name_owned = task_name.clone();
            let node_id_owned = node_id.clone();
            let state_key_owned = state_key.clone();
            tokio::spawn(async move {
                let scenario_name = match apply_reschedule_yaml_patched(
                    &schedule_name, &wid, &task_name_owned, new_cpu_affinity, &node_id_owned
                ).await {
                    Ok(name) => {
                        println!(
                            "[{}] Reschedule YAML applied to DB, scenario='{}'",
                            wid, name
                        );
                        name
                    }
                    Err(e) => {
                        println!("[{}] Failed to apply reschedule YAML to DB: {}", wid, e);
                        workload_last_cpu().lock().await.remove(&state_key_owned);
                        return;
                    }
                };

                // Trigger the action controller to forward the updated SchedInfo
                // to Timpani-O.
                match sender::trigger_action(scenario_name.clone()).await {
                    Ok(resp) => println!(
                        "[{}] Reschedule triggered for scenario '{}': status={}",
                        wid, scenario_name, resp.into_inner().status
                    ),
                    Err(e) => {
                        println!(
                            "[{}] Failed to trigger reschedule for scenario '{}': {:?}",
                            wid, scenario_name, e
                        );
                        // Clear so the next fault can retry.
                        workload_last_cpu().lock().await.remove(&state_key_owned);
                    }
                }
            });
        }

        Ok(Response::new(TimpaniResponse { status: 0 }))
    }
}
