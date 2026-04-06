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

 ### replace the node name in reschedule_sea.yaml found in Node1 path ~/s-core-poc/Node1/pullpiri/examples/resources/reschedule_sea.yaml, put the Node2's hostname   
 
### Step 2 — Clear the Database (Every Run)

Before each demo run, wipe any stale state from a previous session:

```bash
cd /opt/pullpiri/bin
sudo rm -rf kvs*
```
### Setp 3 - Binaries generation
```bash
cd ~/s-core-poc/Node1/lifecycle/lifecycle/examples/pullpiri_LM
sudo ./setup_system.sh
chmod +x build_adas_libs.sh
./build_adas_libs.sh
```

 ### step - modify timpani.rs with string value "RESCHEDULE_YAML_PATH" with ABSOLUTE PATH LIKE "/home/lgesdv/demo_vso/s-core-poc/Node1/pullpiri/examples/resources/reschedule_sea.yaml"
### Also modify the reschedule_sea.yaml node value Node2(Timpani-n running) hostname
#### update pullpiri_lm_config.json file in the path "/home/lgesdv/demo_vso/s-core-poc/Node1/lifecycle/lifecycle/examples/pullpiri_LM/config/pullpiri_lm_config.json"
### update the timpani-o node config value to reflect the absolute path of the Node1 path for eg as below:
"/home/lgesdv/demo_vso/s-core-poc/Node1/pullpiri/examples/resources/timpani/node_configurations.yaml"
 
### Step 4 — Start the Lifecycle Launch Manager (Node 1)

```bash
cd ~/s-core-poc/Node1/lifecycle/lifecycle/examples/pullpiri_LM
./run.sh --> if any permissions issue faced then run below
sudo -E ./run.sh
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

 Verifying the Setup

```bash
# All required binaries present
ls /opt/pullpiri/bin/{persistency-service,apiserver,monitoringserver,statemanager,\
filtergateway,actioncontroller,policymanager,nodeagent,timpani-o,timpani-n,\
adas_primary,adas_secondary

# Shared libraries present
ls /opt/pullpiri/lib/lib*.so

# Config files present
ls /opt/pullpiri/bin/etc/hm_config.json
ls /opt/pullpiri/etc/hmproc_adas_primary.bin
ls lifecycle/lifecycle/etc/{lm_demo.bin,hm_demo.bin,hmcore.bin,logging.json}
```

For any issue please try mannual way https://github.com/MCO-PICCOLO/s-core-poc/blob/main/Node1/lifecycle/lifecycle/examples/pullpiri_LM/SETUP.md
