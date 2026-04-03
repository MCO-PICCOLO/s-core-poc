# Pullpiri + S-CORE Lifecycle: Full Flow

## What Is This?

This example runs the **ADAS mini-adas application** (FEO scheduler with 3 agents) managed by the
**S-CORE Launch Manager (LM)** and supervised by the **Health Monitor (HM) daemon** — all together
with the **Pullpiri** middleware stack.

---

## Simple Flow (Big Picture)

```
┌─────────────────────────────────────────────────────────────┐
│  Launch Manager (LM)                                        │
│  reads: lm_demo.bin + hm_demo.bin + hmcore.bin              │
│                                                             │
│  1. Starts all components in dependency order               │
│  2. Monitors alive pings from adas_primary via HM daemon    │
│  3. On failure → restart (fallback_run_target)              │
└──────────────────┬──────────────────────────────────────────┘
                   │ starts & monitors
        ┌──────────┼──────────────────────┐
        ▼          ▼                      ▼
  Pullpiri stack  adas_primary        adas_secondary_1/2
  (apiserver,     (FEO primary,       (FEO secondaries,
   nodeagent,      HM client)          no supervision)
   statemanager,
   filtergateway…)
```

---

## How to Run

```bash
cd lifecycle/lifecycle/examples/pullpiri_LM
sudo ./run.sh
```

`run.sh` does:
1. Bazel build lifecycle (LM binary + flatbuffer configs)
2. Cargo build mini-adas with lifecycle feature
3. Install binaries + `.so` to `/opt/pullpiri/`
4. Copy flatbuffer configs to `etc/` and `/opt/pullpiri/etc/`
5. Copy `hm_config.json` to `/opt/pullpiri/bin/etc/`
6. Launch `launch_manager`

---

## Step-by-Step Flow

### Startup
1. LM reads `lm_demo.bin` (which components to run, in what order):

  **Startup Order:**

  persistency-service
  → policymanager
  → apiserver
  → monitoringserver
  → statemanager
  → filtergateway
  → actioncontroller
  → timpani-o
  → adas_primary
  → adas_secondary_1
  → adas_secondary_2

2. LM reads `hm_demo.bin` + `hmcore.bin` (HM daemon rules – which processes are supervised).
3. LM starts Pullpiri services first (persistency, apiserver, statemanager, etc.).
4. LM starts `adas_primary` with env vars `PROCESSIDENTIFIER=adas_primary`.
5. `adas_primary` starts its internal HM client → loads `./etc/hm_config.json`.
6. `adas_primary` calls `lifecycle_client_rs::report_execution_state_running()` → tells LM it is ready.
7. LM marks it **Running**. Starts `adas_secondary_1` and `adas_secondary_2`.

### Normal Operation
```
Every 400ms (FEO cycle):
  adas_primary FEO scheduler runs tasks A0-A9
  │
  ├─ on_cycle_start() → deadline.start_cycle()   ← HM deadline begins
  ├─ run all activities (Camera, Radar, NeuralNet, Braking, Steering…)
  └─ on_cycle_end()   → deadline.stop_cycle()    ← HM deadline ends

Every 200ms (supervisor_api_cycle):
  HM client sends ONE alive ping → HM daemon (inside LM)
  LM counts: expects 1–5 pings per 200ms window → OK
```

### HM Supervision Chain (inside LM/HM daemon)
```
Alive ping received
  └─► Alive Supervision check (every 50ms LM evaluation_cycle)
        └─► if pings in window [1..5]  → OK
            if pings < 1 or > 5        → FAILED
              └─► after 3 failed cycles → EXPIRED
                    └─► Local Supervision  → EXPIRED
                          └─► Global Supervision → STOPPED
                                └─► LM triggers fallback_run_target (RESTART ALL)
```

### Restart (fallback_run_target)
When `Global Supervision = STOPPED`, LM transitions to `fallback_run_target` which restarts the
full Startup set — effectively a full system restart.

---

## Health Monitor – Deadline vs Alive (Two Different Things)

| | HM Deadline | LM Alive Supervision |
|---|---|---|
| **What it checks** | Did FEO cycle finish within [0, 600ms]? | Is the process still responding? |
| **Who checks** | HM daemon (inside LM) | LM directly |
| **Triggered by** | `start_cycle()`/`stop_cycle()` calls | Alive pings from HM client |
| **Config file** | `hmproc_adas_primary.bin` (Bazel-built) | `pullpiri_lm_config.json` |
| **On failure** | Local → Global supervision chain | Same supervision chain |

---

## Config Files and Their Purpose

| File | Location | Purpose |
|---|---|---|
| `pullpiri_lm_config.json` | `examples/pullpiri_LM/config/` | LM: components, startup order, alive supervision params |
| `hm_config.json` | `examples/pullpiri_LM/config/` | adas_primary HM client: deadline window, ping rate |
| `lm_demo.bin` | built by Bazel → `etc/` | LM flatbuffer: component launch config |
| `hm_demo.bin` | built by Bazel → `etc/` | HM daemon flatbuffer: supervision rules |
| `hmcore.bin` | built by Bazel → `etc/` | HM daemon core config |
| `hmproc_adas_primary.bin` | built by Bazel → `etc/` + `/opt/pullpiri/etc/` | HM process interface (identifier, checkpoints) |

---

## Binaries Required from mini-adas

Built from `~feo/examples/rust/mini-adas/` with
`--features "signalling_relayed_tcp,lifecycle"`:

| Binary / File | Deployed to |
|---|---|
| `target/release/adas_primary` | `/opt/pullpiri/bin/adas_primary` |
| `target/release/adas_secondary` | `/opt/pullpiri/bin/adas_secondary` |
| `lib/lib*.so` | `/opt/pullpiri/lib/` |
| `config/hm_config.json` (this repo) | `/opt/pullpiri/bin/etc/hm_config.json` |

> `run.sh` does all of this automatically.

---

## Alive Supervision – The Key Numbers

```
FEO cycle time        : 400 ms   (process_arguments: ["400"])
supervisor_api_cycle  : 200 ms   → 1 alive ping every 200ms
LM reporting_cycle    : 0.2 s    → LM window = 200ms
Expected pings/window : 1–5      (min_indications: 1, max_indications: 5)
failed_cycles_tolerance: 3       → must fail 3 consecutive windows before EXPIRED
LM evaluation_cycle   : 0.05 s   (50ms) → how often LM re-evaluates
Deadline window       : [0, 600ms] → FEO cycle must complete within 600ms
```

These numbers must stay aligned. Changing `supervisor_api_cycle_ms` without updating
`reporting_cycle` (or vice versa) causes spurious alive supervision failures.

**Rule: `supervisor_api_cycle_ms` == `reporting_cycle × 1000` (i.e., same period in ms)**

---

## What Each Log Line Means

| Log | Meaning |
|---|---|
| `LCM started successfully` | LM daemon is up |
| `Reported RUNNING state to Launch Manager` | adas_primary told LM it is alive |
| `Alive Supervision … switched to OK` | Alive pings are arriving correctly |
| `Health Monitor started` | HM client background thread running in adas_primary |
| `Alive Supervision … FAILED … 8 reported (expected ≤ 1)` | **BUG**: supervisor_api_cycle too fast vs reporting_cycle |
| `Alive Supervision … EXPIRED` | Tolerance exceeded → triggers fallback |
| `Global Supervision … STOPPED` | Full restart triggered |
| `Recovery state MainPG/fallback_run_target requested` | LM begins restarting all components |
