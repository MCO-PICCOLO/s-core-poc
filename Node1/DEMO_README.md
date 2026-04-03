# End-to-End Demo: Pullpiri + S-CORE Lifecycle + Timpani Deadline-Miss Recovery

This document walks through the full two-node demo:

- **Node 1 (Master)** — Runs the full Pullpiri stack (all 8 daemons) + Timpani
  Orchestrator (`timpani-o`) + ADAS processes (`adas_primary`,
  `adas_secondary`), all managed by the S-CORE Lifecycle Launch Manager.
- **Node 2 (Worker)** — Runs `nodeagent` + `timpani-n`, receives a workload
  (`sea-app` container), and participates in the deadline-miss recovery
  demonstration.

---

## Prerequisites — Both Nodes

Complete **SETUP.md** on both nodes before running this demo.  
Key checklist:

| Item | Where |
|---|---|
| Bazel 8.4.2, Rust 1.90.0, Java 17 installed | SETUP.md §1 |
| `/opt/pullpiri/{bin,lib,etc,bin/etc}` directories created | SETUP.md §2 |
| All Pullpiri binaries in `/opt/pullpiri/bin/` | SETUP.md §3 |
| `timpani-o` and `timpani-n` in `/opt/pullpiri/bin/` | SETUP.md §3 |
| `adas_primary` C++ `.so` libs built and in `feo/examples/rust/mini-adas/lib/` | SETUP.md §4 |

> **IP addresses used in this guide (replace with your real IPs):**
> - Node 1 (Master): **`<NODE1_IP>`** (e.g., `192.168.2.30`)
> - Node 2 (Worker): **`<NODE2_IP>`** (e.g., `192.168.2.31`)

---

## Node 1 — Master Setup & Launch

### Step 1 — Update the Bind IP

Pullpiri reads its bind IP from `/etc/piccolo/settings.yaml`.  
Set it to **Node 1's actual IP** (or `127.0.0.1` for loopback-only mode):

```bash
sudo sed -i 's/192.168.2.177/<NODE1_IP>/g' /etc/piccolo/settings.yaml
# Verify
cat /etc/piccolo/settings.yaml
```

The resulting file should look like:

```yaml
host:
  name: HPC
  ip: <NODE1_IP>
  type: vehicle
  role: master
dds:
  idl_path: src/vehicle/dds/idl
  domain_id: 100
```

### Step 2 — Create the Persistency Database Directory

```bash
sudo mkdir -p /var/lib/pullpiri
sudo chmod 777 /var/lib/pullpiri
```

### Step 3 — Clear the Database (Every Run)

Before each demo run, wipe any stale state from a previous session:

```bash
cd /var/lib/pullpiri
sudo rm -rf *
```

### Step 4 — Update `timpani-o` Node Configuration

`timpani-o` reads node hardware specs from:

```
/home/acrn/new_ak/vso_score/pullpiri/examples/resources/timpani/node_configurations.yaml
```

Ensure `sea_node` (Node 2) is listed alongside `perf_node`:

```yaml
nodes:
  perf_node:
    available_cpus: [0, 1, 2, 3]
    max_memory_mb: 2048
    architecture: "x86_64"
    location: "front_sensor_unit"
    description: "Autonomous driving perception and sensor fusion node"

  sea_node:
    available_cpus: [0, 1]          # adjust to Node 2's actual CPU count
    max_memory_mb: 2048
    architecture: "x86_64"
    location: "worker_node"
    description: "Safe Exit Assist worker node"
```

> The `node_id: sea_node` field in `safe-exit-assist.yaml` must match the name above.

### Step 5 — Start the Lifecycle Launch Manager (Node 1)

```bash
cd /home/acrn/new_ak/vso_score/lifecycle/lifecycle/examples/pullpiri_LM
sudo ./run.sh
```

Leave this terminal open. The Launch Manager runs in the foreground and prints
logs for all managed processes.

**Expected log output (startup complete):**

```
LCM started successfully
Completed the request for PG MainPG to State MainPG/Startup
Starting Pullpiri Persistency Service
Initializing FilterGateway
initialize action controller
MonitoringServerManager init
FilterGatewayManager init
SchedInfoServer listening on port 50052
DBusServer listening on port 7777
Health Monitor started
```

All 10+ processes are now running under Lifecycle supervision.

---

## Node 2 — Worker Setup & Launch

### Step 6 — Check and Clean the Container

Before starting, make sure no stale `sea-app` container is running:

```bash
podman ps -a
```

If `sea-app` appears in the list:

```bash
podman rm -f sea-app
```

### Step 7 — Configure NodeAgent with Correct IPs

Edit `/etc/piccolo/nodeagent.yaml` to point to Node 1 as the master
and set this node's own IP:

```bash
sudo nano /etc/piccolo/nodeagent.yaml
```

Set these fields:

```yaml
nodeagent:
  node_name: "<NODE2_HOSTNAME>"   # hostname of Node 2 (run: hostname)
  node_type: "vehicle"
  node_role: "nodeagent"
  master_ip: "<NODE1_IP>"         # ← Node 1 IP
  node_ip:   "<NODE2_IP>"         # ← Node 2 IP
  grpc_port: 47004
  log_level: "info"
```

To regenerate the file from scratch using the `install-agent.sh` helper:

```bash
cd /home/acrn/new_ak/vso_score/pullpiri/containers
sudo ./install-agent.sh <NODE1_IP> <NODE2_IP>
```

### Step 8 — Start Lifecycle on Node 2 (nodeagent + timpani-n)

```bash
cd /home/acrn/new_ak/vso_score/lifecycle/lifecycle/examples/pullpiri_LM
./run.sh
```

> On Node 2 this starts only `nodeagent` and `timpani-n` (configure the
> `pullpiri_lm_config.json` on Node 2 to include only those two components,
> or start them manually — see note below).

**Manual alternative** (if Node 2 does not have its own LM config):

```bash
# Terminal A — NodeAgent
/opt/pullpiri/bin/nodeagent

# Terminal B — timpani-n (connects to timpani-o on Node 1)
/opt/pullpiri/bin/timpani-n \
  -n node01 -c 1 -P 85 -p 7777 -l 4 -a <NODE1_IP>
```

---

## Applying the Workload — From Node 1

### Step 9 — Modify and Send the Workload Manifest

The workload manifest is at:

```
/home/acrn/new_ak/vso_score/pullpiri/examples/timpani.sh
```

Open it and verify the API server URL points to **Node 1**:

```bash
cat /home/acrn/new_ak/vso_score/pullpiri/examples/timpani.sh
```

It should read:

```bash
curl -X POST 'http://<NODE1_IP>:47099/api/artifact' \
--header 'Content-Type: text/plain' \
--data "${BODY}"
```

Update the IP if needed:

```bash
sed -i 's|http://[0-9.]*:47099|http://<NODE1_IP>:47099|g' \
  /home/acrn/new_ak/vso_score/pullpiri/examples/timpani.sh
```

Then apply the workload:

```bash
cd /home/acrn/new_ak/vso_score/pullpiri/examples
bash timpani.sh
```

This sends the `Schedule` + `Scenario` + `Package` + `Model` blocks from
`resources/timpani-test.yaml` to the Pullpiri API server on Node 1.

Alternatively, apply the `sea-app` manifest directly:

```bash
curl -X POST 'http://<NODE1_IP>:47099/api/artifact' \
  --header 'Content-Type: text/plain' \
  --data-binary @/home/acrn/new_ak/vso_score/new_timpani/sea_app/safe-exit-assist.yaml
```

### Step 10 — Verify the Container is Running on Node 2

On **Node 2**, confirm `sea-app` has been deployed:

```bash
podman ps -a
```

Expected output:

```
CONTAINER ID  IMAGE                          COMMAND   CREATED   STATUS    NAMES
<id>          sdv.lge.com/demo/sea_app:1.0  ...       ...       Up ...    sea-app
```

The `sea-app` container is now running, pinned to **1 CPU** (as defined by the
initial `cpu_affinity` in the schedule).

---

## Triggering a Deadline Miss — CPU Stress

### Step 11 — Run the CPU Stress Tool on Node 2

With `sea-app` running, saturate its assigned CPUs to force deadline misses:

```bash
cd /home/acrn/new_ak/vso_score/TIMPANI/timpani-n/tools
sudo chrt -f 51 ./stress_app_cpus.sh sea_app 60 98
```

| Argument | Value | Meaning |
|---|---|---|
| `sea_app` | app name | process to stress (matches `/proc/<pid>/comm`) |
| `60` | duration | stress for 60 seconds |
| `98` | load % | 98% CPU load on the assigned core(s) |

This saturates the CPU core that `sea_app` is pinned to, causing it to miss
its real-time deadline.

### What to Expect

On **Node 1** (Launch Manager terminal), `timpani-n` reports deadline misses
back to `timpani-o`:

```
[timpani-n] DEADLINE MISS detected for task sea_app (node: sea_node)
[timpani-o] Received deadline miss report from sea_node
```

Pullpiri's `actioncontroller` detects this through the `policymanager` and
automatically generates a `reschedule` action.

---

## Automatic Schedule Update — Recovery

### Step 12 — Observe the Automatic Reschedule

Pullpiri sends an updated `Schedule` to `timpani-o` **before** restarting the
container, expanding the CPU affinity from 1 to 2 cores:

**Before (initial schedule):**

```yaml
- name: sea_app
  cpu_affinity: 4026531840   # single CPU bitmask
  period: 200000
  runtime: 160000
```

**After (auto-rescheduled):**

```yaml
- name: sea_app
  cpu_affinity: <updated-2-cpu-bitmask>   # 2 CPUs assigned
  period: 200000
  runtime: 160000
```

On **Node 2**, verify the container restarted with the new CPU assignment:

```bash
# Check the container restarted
podman ps -a

# Check CPU affinity of the sea_app process
PID=$(pidof sea_app)
taskset -cp $PID
```

The `sea_app` process should now be schedulable on 2 CPUs instead of 1,
resolving the deadline misses.

On **Node 1** (Launch Manager / timpani-o terminal):

```
[actioncontroller] reschedule action triggered for sea-exit-assist
[timpani-o] Schedule updated for sea_node
[actioncontroller] container sea-app restarted with new schedule
```

---

## Full Command Summary

### Node 1

```bash
# 1. Set bind IP
sudo sed -i 's/192.168.2.177/<NODE1_IP>/g' /etc/piccolo/settings.yaml

# 2. Create DB dir
sudo mkdir -p /var/lib/pullpiri && sudo chmod 777 /var/lib/pullpiri

# 3. Clear DB (every run)
cd /var/lib/pullpiri && sudo rm -rf *

# 4. Start Lifecycle + Pullpiri + ADAS
cd /home/acrn/new_ak/vso_score/lifecycle/lifecycle/examples/pullpiri_LM
sudo ./run.sh

# 5. Apply workload (new terminal)
cd /home/acrn/new_ak/vso_score/pullpiri/examples
bash timpani.sh
```

### Node 2

```bash
# 1. Remove stale container
podman rm -f sea-app 2>/dev/null || true

# 2. Configure nodeagent
sudo ./install-agent.sh <NODE1_IP> <NODE2_IP>
# or edit /etc/piccolo/nodeagent.yaml directly

# 3. Start Lifecycle (nodeagent + timpani-n)
cd /home/acrn/new_ak/vso_score/lifecycle/lifecycle/examples/pullpiri_LM
./run.sh

# 4. After sea-app is running, trigger deadline miss
cd /home/acrn/new_ak/vso_score/TIMPANI/timpani-n/tools
sudo chrt -f 51 ./stress_app_cpus.sh sea_app 60 98

# 5. Observe recovery
podman ps -a
PID=$(pidof sea_app) && taskset -cp $PID
```

---

## Troubleshooting

| Symptom | Likely Cause | Fix |
|---|---|---|
| `sea-app` not deployed on Node 2 | `nodeagent` not connected to master | Check `master_ip` in `/etc/piccolo/nodeagent.yaml` |
| `timpani-n` not connecting | Wrong IP passed to `-a` flag | Use Node 1 IP: `-a <NODE1_IP>` |
| API curl returns connection refused | Pullpiri not fully started | Wait for `actioncontroller` init log on Node 1, then retry |
| No deadline miss detected | Stress tool running on wrong PID | Confirm `sea_app` is running: `pidof sea_app` |
| Container not rescheduled to 2 CPUs | `actioncontroller` reschedule fix not built | Rebuild: `cargo build -p actioncontroller` and restart Node 1 |
| LM kills `adas_primary` immediately | `.so` libs missing from `/opt/pullpiri/lib/` | Run `build_adas_libs.sh` and copy `.so` files to `/opt/pullpiri/lib/` |

---

## Reference: Key File Locations

| File / Binary | Path |
|---|---|
| Launch Manager run script | `lifecycle/lifecycle/examples/pullpiri_LM/run.sh` |
| LM config | `lifecycle/lifecycle/examples/pullpiri_LM/config/pullpiri_lm_config.json` |
| Workload apply script | `pullpiri/examples/timpani.sh` |
| Workload manifest (timpani) | `pullpiri/examples/resources/timpani-test.yaml` |
| sea-app manifest | `new_timpani/sea_app/safe-exit-assist.yaml` |
| Node configuration (timpani-o) | `pullpiri/examples/resources/timpani/node_configurations.yaml` |
| NodeAgent config | `/etc/piccolo/nodeagent.yaml` |
| Piccolo settings | `/etc/piccolo/settings.yaml` |
| Stress tool | `TIMPANI/timpani-n/tools/stress_app_cpus.sh` |
| adas_primary `.so` libs | `feo/examples/rust/mini-adas/lib/` |
| All Pullpiri binaries | `/opt/pullpiri/bin/` |
| Shared `.so` (runtime) | `/opt/pullpiri/lib/` |
