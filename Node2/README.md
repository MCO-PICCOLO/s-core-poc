# Node 2 Setup Guide: Worker Node with NodeAgent + Timpani-n

This guide provides setup instructions for **Node 2 (Worker Node)** in the S-CORE Lifecycle + Pullpiri + Timpani demo.

- **Node 2 (Worker)** — Runs `nodeagent` + `timpani-n`, receives workloads (`sea-app` container), and participates in deadline-miss recovery demonstration.

> **For Node 1 (Master) setup**, refer to [Node1/Node1_README.md](../Node1/Node1_README.md)

---

## Overview

The `run.sh` script automates the build and launch process for the S-CORE Lifecycle Launch Manager with Pullpiri components on Node 2. It performs the following:

1. **Build**: Uses Bazel to build the Pullpiri configuration and the Launch Manager daemon.
2. **Copy Flatbuffer Configs**: Copies the compiled Flatbuffer configuration binaries to the workspace's `etc` directory.
3. **Sync Binaries**: Copies `nodeagent` and `timpani-n` binaries to `/opt/pullpiri/bin` if they exist at the workspace root.
4. **Copy Logging Configs**: Copies logging configuration files to the `etc` directory.
5. **Launch**: Starts the Launch Manager daemon.

---

## Prerequisites

Before starting, ensure you have:

- **Ubuntu 22.04+ LTS**
- **sudo access**
- **Network connectivity to Node 1** (Master)
- **Git repository cloned:** `s-core-poc/`
- **Node 1 must be running** before starting Node 2
- **Bazel** must be installed and available in your PATH
- **Rust and Cargo** installed

### Required Software

Install these prerequisites if not already present:

```bash
# Update system
sudo apt update

# Install Rust (required for nodeagent and sea_app)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustup toolchain install 1.90.0

# Install Bazel (required for lifecycle)
sudo wget -O /usr/local/bin/bazel \
  https://github.com/bazelbuild/bazelisk/releases/latest/download/bazelisk-linux-amd64
sudo chmod +x /usr/local/bin/bazel

# Install Java 17 (required for Bazel)
sudo apt install -y openjdk-17-jdk

# Install build tools
sudo apt install -y cmake build-essential git

# Install Podman (required for containers)
sudo apt install -y podman

# Install Timpani dependencies
sudo apt install -y libsystemd-dev
```

---
### Node 2 - Setup & Launch 
--
### Step 1 — Configure NodeAgent

Create the NodeAgent configuration file with your Node 1 (Master) and Node 2 (Worker) IP addresses:

```bash
# Create directory if it doesn't exist
sudo mkdir -p /etc/piccolo

# Create nodeagent.yaml (replace <NODE1_IP> and <NODE2_IP> with actual IPs)
sudo tee /etc/piccolo/nodeagent.yaml > /dev/null <<EOF
nodeagent:
  node_name: "$(hostname)"
  node_type: "vehicle"
  node_role: "nodeagent"
  master_ip: "<Node1 Ip>"
  node_ip: "<Node 2 IP - present system IP>"
  grpc_port: 47004
  log_level: "info"
  metrics:
    collection_interval: 5
    batch_size: 50
  system:
    hostname: "$(hostname)"
    platform: "Linux"
    architecture: "x86_64"
EOF

# Verify configuration
cat /etc/piccolo/nodeagent.yaml
```

**Replace placeholders:**
- `<NODE1_IP>` — IP address of Node 1 (Master), e.g., `192.168.10.100`
- `<NODE2_IP>` — IP address of this Node 2 (Worker), e.g., `192.168.10.101`

### Step 2 — Update pullpiri_lm_config.json

Update the Timpani-n IP configuration to point to Node 1:

```bash
# Navigate to config directory
cd ~/s-core-poc/Node2/lifecycle/examples/pullpiri_LM/config

# Edit pullpiri_lm_config.json
nano pullpiri_lm_config.json
# or
vi pullpiri_lm_config.json
```

**Update the `timpani-n` IP config value** to reflect the IP of Node 1:

```json
"timpani-n": {
    "component_properties": {
        "binary_name": "timpani-n",
        "process_arguments": [
            "-n",
            "perf_node",
            "-c",
            "1",
            "-P",
            "85",
            "-p",
            "7777",
            "-l",
            "4",
            "<NODE1_IP>"
        ],
        "depends_on": []
    },
}
```

Replace `<NODE1_IP>` with Node 1's actual IP address (e.g., `10.221.40.153`).

---

## Setup Steps

You have two options: **Automated** or **Manual** setup.

### Option A: Automated Setup (Recommended)

Use the provided script to automatically build all components:

```bash
# Navigate to scripts directory
cd ~/s-core-poc/Node2/lifecycle/examples/pullpiri_LM

# Make script executable
chmod +x setup_node2.sh

# Run system setup (installs prerequisites and builds all binaries)
sudo ./setup_node2.sh
```

**What this does:**
- Creates `/opt/pullpiri/` directory structure
- Builds NodeAgent binary (~100MB)
- Builds Timpani-n binary
- Builds sea-app container image
- Creates WORKSPACE file

**Duration:** ~10-15 minutes (depending on system and whether packages are cached)

After running the automated script, proceed to:
- **Step 1**: Configure `/etc/piccolo/nodeagent.yaml`
- **Step 2**: Update `pullpiri_lm_config.json`
- **Step 7**: Clean existing containers (for subsequent runs)

### Option B: Manual Setup

Follow Steps 1-4 below for manual installation and build process.

---



### Step 1 — Build NodeAgent Binary

> **Important:** The `nodeagent` binary is around 100MB and cannot be uploaded to GitHub, so it **must be built locally** on Node 2.

```bash
# Navigate to nodeagent source directory
cd ~/s-core-poc/Node2/pullpiri/src/agent/nodeagent

# Build with Cargo (Release mode)
cargo build --release

# Create directory for binaries
sudo mkdir -p /opt/pullpiri/bin

# Copy nodeagent binary to system location
sudo cp target/release/nodeagent /opt/pullpiri/bin/

# Set ownership and permissions
sudo chown root:root /opt/pullpiri/bin/* || true
sudo chmod +x /opt/pullpiri/bin/* || true

# Verify binary exists (should be ~100MB)
ls -lh /opt/pullpiri/bin/nodeagent
```

**Expected output:**
```
-rwxr-xr-x 1 root root 100M ... /opt/pullpiri/bin/nodeagent
```

### Step 2 — Build Timpani-n

```bash
# Navigate to TIMPANI directory
cd ~/s-core-poc/Node2/TIMPANI

# Clone libbpf library (if not already present)
if [ ! -d "libbpf/src" ]; then
    git clone https://github.com/libbpf/libbpf.git
fi

# Build timpani-n
cd timpani-n
mkdir -p build
cd build
cmake ..
make

# Copy binary to /opt/pullpiri/bin
sudo cp timpani-n /opt/pullpiri/bin/

# Set ownership and permissions
sudo chown root:root /opt/pullpiri/bin/* || true
sudo chmod +x /opt/pullpiri/bin/* || true

# Verify
ls -lh /opt/pullpiri/bin/timpani-n
```

### Step 3 — Build sea-app Container

The `sea-app` is a Safe Exit Assist application that will be deployed as a container workload:

```bash
# Navigate to sea_app directory
cd ~/s-core-poc/Node2/sea_app

# Build the Rust binary
cargo build --release

# Build Podman container image
sudo podman build -t sdv.lge.com/demo/sea_app:1.0 .

# Verify image was created
sudo podman images | grep sea_app
```

**Expected output:**
```
sdv.lge.com/demo/sea_app  1.0  <image-id>  <size>  <time>
```


### Step 4 — Clean Existing Containers (Before Each Run)

> **Note:** On the first run, there won't be any existing containers. This step is needed for subsequent runs.

Before starting the Launch Manager, ensure no stale `sea-app` container is running:

```bash
# Check for existing containers
sudo podman ps -a

# If sea-app appears in the list, stop and remove it
sudo podman stop sea-app 2>/dev/null || true
sudo podman rm -f sea-app 2>/dev/null || echo "No existing container to remove (first run)"
```

---

## Running the Launch Manager

After completing all setup steps, start the Launch Manager:

```bash
# Navigate to run script location
cd ~/s-core-poc/Node2/lifecycle/examples/pullpiri_LM

# Make script executable
chmod +x run.sh

# Start Launch Manager (runs nodeagent and timpani-n)
sudo ./run.sh
```

> **Note:** Run the script from the `pullpiri_LM` directory. `sudo` is required for copying files to system directories and setting permissions.

**Expected output:**
```
=============================================
  Pullpiri Lifecycle - Build & Run
=============================================
Building Pullpiri configuration...
Starting Launch Manager daemon...
[nodeagent] Connecting to master at <NODE1_IP>...
[timpani-n] Monitoring started
```

**⚠️ Leave this terminal running.** The Launch Manager will print logs for all managed processes.

---

## Demo Workflow — Deadline Miss Recovery

Once Node 2 is running, follow these steps to demonstrate automated deadline-miss recovery:

### Step 1 — Update Deployment Configuration

Update the workload manifest with Node 2's hostname:

```bash
cd ~/s-core-poc/Node2/examples/resources

# Update safe-exit-assist.yaml with this node's hostname
HOSTNAME=$(hostname)
sed -i "s/node_name: .*/node_name: $HOSTNAME/g" safe-exit-assist.yaml

# Verify the change
cat safe-exit-assist.yaml | grep node_name
```

### Step 2 — Deploy sea-app Container

Deploy the container workload from Node 2 to Node 1's API server:

```bash
cd ~/s-core-poc/Node2/examples

# Update timpani.sh with Node 1's IP address
sed -i "s|http://.*:47099|http://<NODE1_IP>:47099|g" timpani.sh

# Deploy the workload
bash timpani.sh
```

**Expected response:** `OK` from API server

### Step 3 — Verify Container Deployment

Check that the container is running with initial CPU allocation:

```bash
# Check container status
sudo podman ps | grep sea_app

# Check initial CPU affinity (should be 1 CPU)
taskset -c -p $(pgrep -x sea_app)
```

**Expected output:**
```
pid <pid>'s current affinity list: 1
```

This means the container is running on **CPU core 1 only**.

### Step 4 — Trigger Deadline Miss with CPU Stress

With `sea-app` running, saturate its assigned CPUs to force deadline misses:

```bash
cd ~/s-core-poc/Node2/TIMPANI/timpani-n/tools

# Run stress tool
# Arguments: <app_name> <duration_seconds> <cpu_load_percentage>
sudo chrt -f 51 ./stress_app_cpus.sh sea_app 60 98
```

| Argument | Value | Meaning |
|---|---|---|
| `sea_app` | app name | process to stress (matches `/proc/<pid>/comm`) |
| `60` | duration | stress for 60 seconds |
| `98` | load % | 98% CPU load on the assigned core(s) |

This saturates the CPU core that `sea_app` is pinned to, causing it to miss its real-time deadline.

**What happens:**

On **Node 1** (Launch Manager terminal), `timpani-n` reports deadline misses back to `timpani-o`:

```
[timpani-n] DEADLINE MISS detected for task sea_app (node: sea_node)
[timpani-o] Received deadline miss report from sea_node
```

Pullpiri's `actioncontroller` detects this through the `statemanager` and automatically generates a `reschedule` action.

### Step 5 — Observe Automatic Reschedule Recovery

Pullpiri sends an updated `Schedule` to `timpani-o` **before** restarting the container, expanding the CPU affinity from 1 to 2 cores:

**Before (initial schedule):**

```yaml
- name: sea_app
  cpu_affinity: 2   # first CPU is assigned
  period: 200000
  runtime: 160000
```

**After (auto-rescheduled):**

```yaml
- name: sea_app
  cpu_affinity: 4   # 2 CPU is assigned
  period: 200000
  runtime: 160000
```

On **Node 2**, verify the container restarted with the new CPU assignment:

```bash
# Check the container restarted
sudo podman ps -a

# Check CPU affinity of the sea_app process
PID=$(pidof sea_app)
taskset -cp $PID
```

**Expected output:**
```
pid <new_pid>'s current affinity list: 1,2
```

The `sea_app` process should now be schedulable on 2nd CPU instead of 1st cpu, resolving the deadline misses.

On **Node 1** (Launch Manager / timpani-o terminal):

```
[actioncontroller] reschedule action triggered for sea-exit-assist
[timpani-o] Schedule updated for sea_node
[actioncontroller] container sea-app restarted with new schedule
```

**✅ Success!** The system detected the deadline miss and automatically increased CPU allocation from 1 → 2 cores, resolving the performance issue.

---

## Quick Reference Commands

### Automated Setup (Recommended)

```bash
# Navigate to scripts directory
cd ~/s-core-poc/Node2/lifecycle/examples/pullpiri_LM

# Run automated setup
sudo ./setup_node2.sh

# Configure nodeagent (replace IPs)
sudo mkdir -p /etc/piccolo
sudo nano /etc/piccolo/nodeagent.yaml

# Update pullpiri_lm_config.json
cd ~/s-core-poc/Node2/lifecycle/examples/pullpiri_LM/config
nano pullpiri_lm_config.json

# Start Launch Manager
cd ~/s-core-poc/Node2/lifecycle/examples/pullpiri_LM
sudo ./run.sh
```

### Manual First-Time Setup

```bash
# Step 1: Configure nodeagent (replace IPs)
sudo mkdir -p /etc/piccolo
sudo nano /etc/piccolo/nodeagent.yaml

# Step 2: Update pullpiri_lm_config.json
cd ~/s-core-poc/Node2/lifecycle/examples/pullpiri_LM/config
nano pullpiri_lm_config.json

# Step 3: Build nodeagent
cd ~/s-core-poc/Node2/pullpiri/src/agent/nodeagent
cargo build --release
sudo mkdir -p /opt/pullpiri/bin
sudo cp target/release/nodeagent /opt/pullpiri/bin/
sudo chown root:root /opt/pullpiri/bin/* || true
sudo chmod +x /opt/pullpiri/bin/* || true

# Step 4: Build timpani-n
cd ~/s-core-poc/Node2/TIMPANI
[ ! -d "libbpf/src" ] && git clone https://github.com/libbpf/libbpf.git
cd timpani-n && mkdir -p build && cd build
cmake .. && make
sudo cp timpani-n /opt/pullpiri/bin/
sudo chown root:root /opt/pullpiri/bin/* || true
sudo chmod +x /opt/pullpiri/bin/* || true

# Step 5: Build sea-app container
cd ~/s-core-poc/Node2/sea_app
cargo build --release
sudo podman build -t sdv.lge.com/demo/sea_app:1.0 .

# Step 6: Create WORKSPACE
cd ~/s-core-poc/Node2/lifecycle
touch WORKSPACE

# Step 7: Start Launch Manager
cd ~/s-core-poc/Node2/lifecycle/examples/pullpiri_LM
sudo ./run.sh
```

### Subsequent Runs

```bash
# Clean and restart
sudo podman stop sea-app 2>/dev/null || true
sudo podman rm -f sea-app 2>/dev/null || true
cd ~/s-core-poc/Node2/lifecycle/examples/pullpiri_LM
sudo ./run.sh
```

### Complete Cleanup

To remove all installed files and build artifacts:

```bash
cd ~/s-core-poc/Node2/lifecycle/examples/pullpiri_LM
sudo ./clean_node2.sh
```

**What it removes:**
- `/opt/pullpiri/` (nodeagent, timpani-n, configs)
- Build artifacts in `pullpiri/src/target/`
- Build artifacts in `TIMPANI/timpani-n/build/`
- sea-app container image (`sdv.lge.com/demo/sea_app:1.0`)
- sea-app build artifacts in `sea_app/target/`
- Lifecycle `WORKSPACE` file and Bazel symlinks

**Preserved:**
- `/etc/piccolo/settings.yaml` and `/etc/piccolo/nodeagent.yaml` (custom configs)
- `TIMPANI/libbpf/` (git clone - remove manually if needed)

**After cleanup, to rebuild:**
```bash
cd ~/s-core-poc/Node2/lifecycle/examples/pullpiri_LM
sudo ./setup_node2.sh
```

### Demo Workflow

```bash
# 1. Deploy sea-app (update <NODE1_IP> first)
cd ~/s-core-poc/Node2/examples
bash timpani.sh

# 2. Check initial CPU affinity
taskset -c -p $(pgrep -x sea_app)

# 3. Trigger deadline miss
cd ~/s-core-poc/Node2/TIMPANI/timpani-n/tools
sudo chrt -f 51 ./stress_app_cpus.sh sea_app 60 98

# 4. Verify reschedule (after ~10-20 seconds)
taskset -c -p $(pgrep -x sea_app)
```
---

| Symptom | Likely Cause | Fix |
|---|---|---|
| `sea-app` not deployed on Node 2 | `nodeagent` not connected to master | Check `master_ip` in `/etc/piccolo/nodeagent.yaml` |
| `timpani-n` not connecting | Wrong IP passed to `-a` flag | Use Node 1 IP: `-a <NODE1_IP>` |
| API curl returns connection refused | Pullpiri not fully started | Wait for `actioncontroller` init log on Node 1, then retry |
| No deadline miss detected | Stress tool running on wrong PID | Confirm `sea_app` is running: `pidof sea_app` |
| Container not rescheduled to 2 CPUs | `actioncontroller` reschedule fix not built | Rebuild: `cargo build -p actioncontroller` and restart Node 1 |
| LM kills `adas_primary` immediately | `.so` libs missing from `/opt/pullpiri/lib/` | Run `build_adas_libs.sh` and copy `.so` files to `/opt/pullpiri/lib/` |

---

## Multi-Workload Support and Testing

Timpani-n supports running **multiple independent workloads simultaneously**, each with its own container, real-time schedule, and deadline monitoring. The `setup_node2.sh` script builds both `sea_app` and `sea_app2` container images so this scenario is ready to test out of the box.

### Overview

| Workload | Container | Process | Schedule | CPU (initial) |
|---|---|---|---|---|
| `sea-schedule` | `sea-app` | `sea_app` | `safe-exit-assist.yaml` | CPU 1 |
| `sea-schedule2` | `sea-app2` | `sea_app2` | `safe-exit-assist2.yaml` | CPU 3 |

Each workload is tracked independently by timpani-n — deadline misses for one workload do not affect the other.

### Step 1 — Build sea_app2 Container Image

The automated setup script builds both images. If you need to build `sea_app2` manually:

```bash
cd ~/s-core-poc/Node2/sea_app2
cargo build --release
sudo podman build -t sdv.lge.com/demo/sea_app2:1.0 .

# Verify
sudo podman images | grep sea_app2
```

**Expected output:**
```
sdv.lge.com/demo/sea_app2  1.0  <image-id>  <size>  <time>
```

### Step 2 — Deploy the First Workload (sea-app)

Update the node name in `safe-exit-assist.yaml` and deploy as described in the single-workload demo:

```bash
cd ~/s-core-poc/Node2/examples/resources

# Set this node's hostname in the manifest
HOSTNAME=$(hostname)
sed -i "s/node: .*/node: $HOSTNAME/" safe-exit-assist.yaml

cd ~/s-core-poc/Node2/examples

# Ensure timpani.sh points to safe-exit-assist.yaml (it does by default)
# BODY=$(< ./resources/safe-exit-assist.yaml)   ← active line in timpani.sh

bash timpani.sh
```

Verify `sea-app` is running:

```bash
sudo podman ps | grep sea-app
taskset -c -p $(pgrep -x sea_app)
# Expected: pid <pid>'s current affinity list: 1
```

### Step 3 — Deploy the Second Workload (sea-app2)

#### 3a — Update the node name in safe-exit-assist2.yaml

```bash
cd ~/s-core-poc/Node2/examples/resources

HOSTNAME=$(hostname)
sed -i "s/node: .*/node: $HOSTNAME/" safe-exit-assist2.yaml

# Verify
grep "node:" safe-exit-assist2.yaml
```

#### 3b — Switch timpani.sh to use the second manifest

Edit `timpani.sh` and swap the active `BODY` line:

```bash
cd ~/s-core-poc/Node2/examples
nano timpani.sh   # or vi timpani.sh
```

Change:
```bash
BODY=$(< ./resources/safe-exit-assist.yaml)
#BODY=$(< ./resources/safe-exit-assist2.yaml)
```

To:
```bash
#BODY=$(< ./resources/safe-exit-assist.yaml)
BODY=$(< ./resources/safe-exit-assist2.yaml)
```

#### 3c — Deploy the second workload

```bash
cd ~/s-core-poc/Node2/examples
bash timpani.sh
```

**Expected response:** `OK` from API server

### Step 4 — Verify Both Containers Are Running

```bash
# Both containers should appear
sudo podman ps | grep sea-app

# Check CPU affinity of each process independently
taskset -c -p $(pgrep -x sea_app)
# Expected: pid <pid>'s current affinity list: 1

taskset -c -p $(pgrep -x sea_app2)
# Expected: pid <pid>'s current affinity list: 3
```

Each process runs on its own dedicated CPU core as assigned by its schedule.

### Step 5 — Trigger Deadline Miss on Each Workload Independently

You can stress each workload independently to verify isolation:

```bash
cd ~/s-core-poc/Node2/TIMPANI/timpani-n/tools

# Stress sea_app (workload 1) only
sudo chrt -f 51 ./stress_app_cpus.sh sea_app 30 98

# Stress sea_app2 (workload 2) only
sudo chrt -f 51 ./stress_app_cpus.sh sea_app2 30 98
```

Timpani-n logs will show deadline misses attributed to the correct workload and task — misses from `sea_app2` will not appear under `sea-schedule`, and vice versa:

```
[ERROR] !!! DEADLINE MISS sea_app(PID): deadline miss reported for workload 'sea-schedule'
[ERROR] !!! DEADLINE MISS sea_app2(PID): deadline miss reported for workload 'sea-schedule2'
```

### Step 6 — Verify Reschedule for Each Workload

After the stress run ends, check that each process recovered to its rescheduled affinity:

```bash
# Verify sea_app recovered
taskset -c -p $(pgrep -x sea_app)
# Expected (after reschedule): pid <pid>'s current affinity list: 1,2

# Verify sea_app2 recovered
taskset -c -p $(pgrep -x sea_app2)
# Expected (after reschedule): pid <pid>'s current affinity list: 2,3
```

### Cleanup for Multi-Workload Run

Before restarting, remove both containers:

```bash
sudo podman stop sea-app sea-app2 2>/dev/null || true
sudo podman rm -f sea-app sea-app2 2>/dev/null || true
```

---
