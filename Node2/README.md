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

5. run the stress app from the s-core-poc/Node2/TIMPANI/timpani-n/tool
sudo chrt -f 51 ./stress_app_cpus.sh sea_app 60 99 [60 is timer secs, 99 is percentage of cpu load]

6.once gain check cpu rescheduling
taskset -c -p $(pgrep -x sea_app) check cpu affinity is changed

---
**Note:** The script will stop and report errors if any step fails (due to `set -e`).
