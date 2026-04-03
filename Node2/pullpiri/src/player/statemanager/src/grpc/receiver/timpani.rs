/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use crate::grpc::sender;
use common::external::timpani::fault_service_server::FaultService;
use common::external::timpani::{FaultInfo, FaultType, Response as TimpaniResponse};
use common::spec::artifact::{Artifact, Model, Network, Node, Package, Scenario, Schedule, Volume};
use std::collections::HashSet;
use std::sync::OnceLock;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

/// Path to the reschedule YAML file containing the updated Schedule, Scenario,
/// and Package artifacts to apply when a deadline-miss fault is received.
/// Edit this path to point to wherever the reschedule spec is stored.
const RESCHEDULE_YAML_PATH: &str =
    "/home/acrn/new_ak/vso_score/pullpiri/examples/resources/reschdule.yaml";

/// Per-workload reschedule state.
/// Once a workload is inserted here, all further deadline-miss faults for it
/// are silently ignored — the reschedule has already been applied.
static RESCHEDULED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn rescheduled() -> &'static Mutex<HashSet<String>> {
    RESCHEDULED.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Read `reschdule.yaml`, parse every `---`-separated document, write each one
/// into the DB under `{Kind}/{name}`, and return the Scenario name found.
///
/// This mirrors what `apiserver::artifact::apply` does, so the DB ends up with
/// the exact artifacts defined in the yaml — including the full Schedule spec.
async fn apply_yaml_to_db() -> Result<String, String> {
    let content = tokio::fs::read_to_string(RESCHEDULE_YAML_PATH)
        .await
        .map_err(|e| format!("Failed to read '{}': {}", RESCHEDULE_YAML_PATH, e))?;

    let mut scenario_name: Option<String> = None;

    for doc in content.split("---") {
        let doc = doc.trim();
        if doc.is_empty() {
            continue;
        }

        let value: serde_yaml::Value = serde_yaml::from_str(doc)
            .map_err(|e| format!("Failed to parse YAML document: {}", e))?;

        let kind = match value.get("kind").and_then(|v| v.as_str()) {
            Some(k) => k.to_string(),
            None => continue,
        };

        let (key, name) = match kind.as_str() {
            "Schedule" => {
                let s: Schedule = serde_yaml::from_value(value.clone())
                    .map_err(|e| format!("Failed to parse Schedule: {}", e))?;
                let n = s.get_name();
                (format!("Schedule/{}", n), n)
            }
            "Scenario" => {
                let s: Scenario = serde_yaml::from_value(value.clone())
                    .map_err(|e| format!("Failed to parse Scenario: {}", e))?;
                let n = s.get_name();
                scenario_name = Some(n.clone());
                (format!("Scenario/{}", n), n)
            }
            "Package" => {
                let p: Package = serde_yaml::from_value(value.clone())
                    .map_err(|e| format!("Failed to parse Package: {}", e))?;
                let n = p.get_name();
                (format!("Package/{}", n), n)
            }
            "Volume" => {
                let v: Volume = serde_yaml::from_value(value.clone())
                    .map_err(|e| format!("Failed to parse Volume: {}", e))?;
                let n = v.get_name();
                (format!("Volume/{}", n), n)
            }
            "Network" => {
                let v: Network = serde_yaml::from_value(value.clone())
                    .map_err(|e| format!("Failed to parse Network: {}", e))?;
                let n = v.get_name();
                (format!("Network/{}", n), n)
            }
            "Node" => {
                let v: Node = serde_yaml::from_value(value.clone())
                    .map_err(|e| format!("Failed to parse Node: {}", e))?;
                let n = v.get_name();
                (format!("Node/{}", n), n)
            }
            "Model" => {
                let v: Model = serde_yaml::from_value(value.clone())
                    .map_err(|e| format!("Failed to parse Model: {}", e))?;
                let n = v.get_name();
                (format!("Model/{}", n), n)
            }
            other => {
                println!("Skipping unknown kind '{}' in reschedule yaml", other);
                continue;
            }
        };

        let artifact_str = serde_yaml::to_string(&value)
            .map_err(|e| format!("Failed to serialize '{}': {}", key, e))?;

        common::persistency::put(&key, &artifact_str)
            .await
            .map_err(|e| format!("Failed to write '{}' to DB: {}", key, e))?;

        println!("Applied {}/{} to DB", kind, name);
    }

    scenario_name.ok_or_else(|| {
        format!(
            "No Scenario document found in '{}'",
            RESCHEDULE_YAML_PATH
        )
    })
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
        println!("{}{}{:?}", 4, "Received fault notification: ", info);

        if info.r#type == FaultType::Dmiss as i32 {
            let workload_id = info.workload_id.clone();

            // Check if reschedule was already done for this workload
            {
                let done = rescheduled().lock().await;
                if done.contains(&workload_id) {
                    println!(
                        "Deadline miss for '{}' ignored — reschedule already applied",
                        workload_id
                    );
                    return Ok(Response::new(TimpaniResponse { status: 0 }));
                }
            }

            tokio::spawn(async move {
                // Mark as handled immediately to block concurrent fault notifications
                {
                    let mut done = rescheduled().lock().await;
                    if !done.insert(workload_id.clone()) {
                        println!(
                            "Deadline miss for '{}' ignored — reschedule already in progress",
                            workload_id
                        );
                        return;
                    }
                }

                // Step 1: read reschdule.yaml and apply all artifacts (Schedule, Scenario,
                // Package) to DB — this picks up any edits made to the yaml file.
                let scenario_name = match apply_yaml_to_db().await {
                    Ok(name) => {
                        println!(
                            "Applied reschedule yaml to DB, scenario='{}'",
                            name
                        );
                        name
                    }
                    Err(e) => {
                        println!("Failed to apply reschedule yaml: {}", e);
                        rescheduled().lock().await.remove(&workload_id);
                        return;
                    }
                };

                // Step 2: trigger actioncontroller — reads Scenario/Package/Schedule from DB
                // and forwards the updated SchedInfo to timpani-o (no container restart).
                match sender::trigger_action(scenario_name.clone()).await {
                    Ok(resp) => println!(
                        "Reschedule triggered for scenario '{}': status={}",
                        scenario_name,
                        resp.into_inner().status
                    ),
                    Err(e) => {
                        println!(
                            "Failed to trigger reschedule for scenario '{}': {:?}",
                            scenario_name, e
                        );
                        // Remove so the fault can be retried on next notification
                        rescheduled().lock().await.remove(&workload_id);
                    }
                }
            });
        }

        Ok(Response::new(TimpaniResponse { status: 0 }))
    }
}
