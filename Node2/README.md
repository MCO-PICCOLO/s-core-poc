Node2->
# README for run.sh (Pullpiri Lifecycle Launch Manager)

## Overview
This script (`run.sh`) automates the build and launch process for the S-CORE Lifecycle Launch Manager with Pullpiri components. It compiles the required binaries, copies configuration and logging files, syncs necessary binaries to `/opt/pullpiri/bin`, and finally starts the Launch Manager daemon.

## Steps Performed
1. **Build**: Uses Bazel to build the Pullpiri configuration and the Launch Manager daemon.
2. **Copy Flatbuffer Configs**: Copies the compiled Flatbuffer configuration binaries to the workspace's `etc` directory.
3. **Sync Binaries**: Copies `nodeagent` and `timpani-n` binaries to `/opt/pullpiri/bin` if they exist at the workspace root.
4. **Copy Logging Configs**: Copies logging configuration files to the `etc` directory.
5. **Launch**: Starts the Launch Manager daemon.

## Usage
```sh
### based on user config use root , if not root also it works
sudo ./run.sh
```
- Run the script from the `pullpiri_LM` directory.
- `sudo` is required for copying files to system directories and setting permissions.

## Prerequisites
- Bazel must be installed and available in your PATH.
- The workspace should contain the required source files and Bazel build targets.
- `nodeagent` and `timpani-n`

 ## Node 2 — Worker Setup & Launch
Step 1 — Check and Clean the Container
Before starting, make sure no stale sea-app container is running:

podman ps -a
If sea-app appears in the list:

podman rm -f sea-app
Step 2 — Configure NodeAgent with Correct IPs
Edit /etc/piccolo/nodeagent.yaml to point to Node 1 as the master and set this node's own IP:

sudo nano /etc/piccolo/nodeagent.yaml
Set these fields:

nodeagent:
  node_name: "<NODE2_HOSTNAME>"   # hostname of Node 2 (run: hostname)
  node_type: "vehicle"
  node_role: "nodeagent"
  master_ip: "<NODE1_IP>"         # ← Node 1 IP
  node_ip:   "<NODE2_IP>"         # ← Node 2 IP
  grpc_port: 47004
  log_level: "info"


#### update pullpiri_lm_config.json file in the path "/home/lge/s-core-poc/Node2/lifecycle/examples/pullpiri_LM/config/pullpiri_lm_config.json"
### update the timpani-n Ip config value to reflect the IP of Node1 for eg as below:

```bash
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
                    "10.221.40.153"
                ],
                "depends_on": []
            },
```

# Node2: Building and Running nodeagent with Lifecycle Launch manager

Node2 is responsible for building the Pullpiri `nodeagent` binary (note: the binary is around 100MB and cannot be uploaded to GitHub, so it must be built locally on Node2) and running the integration using its own run script.

### Steps to Build nodeagent

1. Go to the nodeagent source directory:
   ```sh
   cd Node2/pullpiri/src/agent/nodeagent
   ```
2. Build the nodeagent binary using Cargo:
   ```sh
   cargo build --release
   ```
   The resulting binary will be located at:
   ```
   target/release/nodeagent
   ```
3. Copy the built binary to the Node2 lifecycle repository:
   ```sh
   cp target/release/nodeagent ../../../../lifecycle/
   ```
   Adjust the destination path as needed for your setup.

### Timpani binaries

`timpani-n`  must also be present in `/opt/pullpiri/bin/`.  
Build them from the **Timpani** repo:
cd ../../TIMPANI/timpani-n
mkdir build
cd build
cmake ..
make

```bash
sudo cp -f timpani-n /opt/pullpiri/bin/
#sudo cp <timpani_build_dir>/timpani-n /opt/pullpiri/bin/
sudo chmod +x /opt/pullpiri/bin/timpani-n 
#sudo chmod +x /opt/pullpiri/bin/timpani-n

## What Happens Next?
Run the workflow in this order:
1. Build `sea_app` and its Podman image:
   ```sh
   cd ~/s-core-poc/Node2/sea_app
   cargo build --release
   podman build -t sdv.lge.com/demo/sea_app:1.0 .
   ```
2. Run the Lifecycle Launch Manager script:
   ```sh
   ### check if sea_app container is running, and remove it before running below
  sudo podman rm -f sea-app_sea-app
   cd ~/s-core-poc/Node2/lifecycle/examples/pullpiri_LM
   sudo ./run.sh
   ```
3. Move to the examples directory:
   ```sh
   cd ~/s-core-poc/Node2/examples
   ```
4. Run Timpani workflow:
   ```sh
   cd resources
     ### replace the node name in safe-exit-assist.yaml, put the Node2's hostname   

   ### replace ip of curlcommand with Node1 IP
   bash timpani.sh
### once OK comes check the current CPU affinity
   taskset -c -p $(pgrep -x sea_app)

## Triggering a Deadline Miss — CPU Stress

### Step  — Run the CPU Stress Tool on Node 2

With `sea-app` running, saturate its assigned CPUs to force deadline misses:

```bash
cd ~/s-core-poc/Node2/TIMPANI/timpani-n/tools
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

Pullpiri's `actioncontroller` detects this through the `statemanager` and
automatically generates a `reschedule` action.

---

## Automatic Schedule Update — Recovery

### Step 7 — Observe the Automatic Reschedule

Pullpiri sends an updated `Schedule` to `timpani-o` **before** restarting the
container, expanding the CPU affinity from 1 to 2 cores:

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

The `sea_app` process should now be schedulable on 2nd CPU instead of 1st cpu,
resolving the deadline misses.

On **Node 1** (Launch Manager / timpani-o terminal):

```
[actioncontroller] reschedule action triggered for sea-exit-assist
[timpani-o] Schedule updated for sea_node
[actioncontroller] container sea-app restarted with new schedule
```

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
