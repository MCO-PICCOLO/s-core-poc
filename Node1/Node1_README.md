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

> **Note:** If using the **automated setup scripts** (recommended), most prerequisites will be installed automatically.
> For **manual setup**, complete all items in the checklist below.

Key checklist for manual setup:

| Item | Where | Auto-installed? |
|---|---|---|
| Bazel 8.4.2, Rust 1.90.0, Java 17 installed | SETUP.md §1 | ✅ Yes (by setup_system.sh) |
| `/opt/pullpiri/{bin,lib,etc,bin/etc}` directories created | SETUP.md §2 | ✅ Yes (by setup_system.sh) |
| All Pullpiri binaries in `/opt/pullpiri/bin/` | SETUP.md §3 | ✅ Yes (by setup_system.sh) |
| `timpani-o` in `/opt/pullpiri/bin/` | SETUP.md §3 | ✅ Yes (by setup_system.sh) |
| `adas_primary` C++ `.so` libs built and in `feo/examples/rust/mini-adas/lib/` | SETUP.md §4 | ✅ Yes (by build_adas_libs.sh) |

> **IP addresses used in this guide (replace with your real IPs):**
> - Node 1 (Master): **`<NODE1_IP>`** (e.g., `192.168.10.100`)
> - Node 2 (Worker): **`<NODE2_IP>`** (e.g., `192.168.10.101`)

---

## Node 1 — Master Setup & Launch

### Step 1 — Create Piccolo Settings File

Pullpiri reads its bind IP from `/etc/piccolo/settings.yaml`.
Create this file with **Node 1's actual IP** (or `127.0.0.1` for loopback-only mode):

```bash
# Create directory if it doesn't exist
sudo mkdir -p /etc/piccolo

# Create settings.yaml file (replace <NODE1_IP> with your actual Node 1 IP)
sudo tee /etc/piccolo/settings.yaml > /dev/null <<EOF
host:
  name: HPC
  ip: <NODE1_IP>
  type: vehicle
  role: master
dds:
  idl_path: src/vehicle/dds/idl
  domain_id: 100
EOF

# Verify the file was created correctly
cat /etc/piccolo/settings.yaml
```

**Replace `<NODE1_IP>`** with Node 1's actual IP address (e.g., `192.168.10.100`).

**Expected output:**
```yaml
host:
  name: HPC
  ip: 192.168.10.100
  type: vehicle
  role: master
dds:
  idl_path: src/vehicle/dds/idl
  domain_id: 100
```

### Step 2 — Update Reschedule Configuration

Update the reschedule policy file with **Node 2's hostname**:

```bash
# Navigate to the configuration directory
cd ~/s-core-poc/Node1/pullpiri/examples/resources

# Get Node 2's hostname (run this command on Node 2)
# hostname

# Update reschedule_sea.yaml with Node 2's hostname
# Replace <NODE2_HOSTNAME> with the actual hostname from Node 2
sed -i 's/node_name: .*/node_name: <NODE2_HOSTNAME>/g' reschedule_sea.yaml

# Verify the change
cat reschedule_sea.yaml | grep node_name
```

### Step 3 — Update Timpani Configuration Files

#### 3a. Modify timpani.rs with absolute path

```bash
# Navigate to statemanager directory
cd ~/s-core-poc/Node1/pullpiri/src/player/statemanager/src/grpc/receiver

# Edit timpani.rs and update RESCHEDULE_YAML_PATH with absolute path
# Replace the path with your actual system path
# Example: "/home/lgesdv/demo_vso/s-core-poc/Node1/pullpiri/examples/resources/reschedule_sea.yaml"
nano timpani.rs
# or
vi timpani.rs
```

**What to change:**
- Find the line with `RESCHEDULE_YAML_PATH`
- Replace with your absolute path to `reschedule_sea.yaml`

#### 3b. Update pullpiri_lm_config.json

```bash
# Navigate to config directory
cd ~/s-core-poc/Node1/lifecycle/lifecycle/examples/pullpiri_LM/config

# Edit pullpiri_lm_config.json
nano pullpiri_lm_config.json
# or
vi pullpiri_lm_config.json
```

**What to change:**
- Update the `timpani-o` node config value to reflect the absolute path
- Example: `"/home/lgesdv/demo_vso/s-core-poc/Node1/pullpiri/examples/resources/timpani/node_configurations.yaml"`

### Step 4 — Clear Database (Every Run)

> **Note:** On the first run, this directory may not exist or may be empty. This is normal.
> The database files are created when services start for the first time.

Before each demo run, wipe any stale state from a previous session:

```bash
# Navigate to Pullpiri bin directory
cd /opt/pullpiri/bin

# Remove database files (if they exist from previous runs)
sudo rm -rf kvs* 2>/dev/null || echo "No existing database files to remove (first run)"
```

### Step 5 — Build Binaries & Dependencies

You have two options: **Automated** or **Manual** setup.

#### Option A: Automated Setup (Recommended)

Use the provided scripts to automatically build all components:

```bash
# Navigate to scripts directory
cd ~/s-core-poc/Node1/lifecycle/lifecycle/examples/pullpiri_LM

# Make scripts executable
chmod +x setup_system.sh build_adas_libs.sh

# Run system setup (installs prerequisites and builds Pullpiri + Timpani binaries)
sudo ./setup_system.sh

# Build ADAS lifecycle libraries
./build_adas_libs.sh
```

**What this does:**
- Installs Java 17, Bazel 8.4.2, Rust 1.90.0 (if not present)
- Creates `/opt/pullpiri/` directory structure
- Builds all 7 Pullpiri service binaries
- Builds `timpani-o` orchestrator
- Builds C++ lifecycle client libraries

#### Option B: Manual Setup

For manual setup instructions, refer to **SETUP.md** in this directory:

```bash
cat ~/s-core-poc/Node1/lifecycle/lifecycle/examples/pullpiri_LM/SETUP.md
```

Follow sections:
- §1: System Prerequisites
- §2: One-Time Directory Setup
- §3: Install Pullpiri Service Binaries
- §4: Build adas_primary C++ Shared Libraries

### Step 6 — Start the Lifecycle Launch Manager

```bash
# Navigate to run script location
cd ~/s-core-poc/Node1/lifecycle/lifecycle/examples/pullpiri_LM

# Make script executable (if not already)
chmod +x run.sh

# Start the Launch Manager
sudo -E ./run.sh
```

> **Important:** Leave this terminal open. The Launch Manager runs in the foreground and prints
> logs for all managed processes.

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

## Verifying the Setup

After completing the setup, verify all components are in place:

```bash
# All required binaries present
ls /opt/pullpiri/bin/{persistency-service,apiserver,monitoringserver,statemanager,\
filtergateway,actioncontroller,policymanager,timpani-o,adas_primary,adas_secondary}

# Shared libraries present
ls /opt/pullpiri/lib/lib*.so

# Config files present
ls /opt/pullpiri/bin/etc/hm_config.json
ls /opt/pullpiri/etc/hmproc_adas_primary.bin
ls ~/s-core-poc/Node1/lifecycle/lifecycle/etc/{lm_demo.bin,hm_demo.bin,hmcore.bin,logging.json}
```

**If any files are missing**, refer to the manual setup guide:
```bash
cat ~/s-core-poc/Node1/lifecycle/lifecycle/examples/pullpiri_LM/SETUP.md
```

---

## Troubleshooting

| Issue | Solution |
|---|---|
| **`/etc/piccolo/settings.yaml` not found** | Run Step 1 to create the file with correct IP |
| **Database clear fails** | Normal on first run - files don't exist yet |
| **`setup_system.sh` fails** | Check you have sudo access and internet connection |
| **Bazel build fails** | Verify Bazel 8.4.2 is installed: `bazel --version` |
| **Launch Manager crashes immediately** | Check all binaries are in `/opt/pullpiri/bin/` and libs in `/opt/pullpiri/lib/` |
| **Path errors for timpani.rs** | Ensure absolute paths are used (no `~/` shortcuts) |
| **Missing `.so` libraries** | Run `build_adas_libs.sh` script from Step 5 |
