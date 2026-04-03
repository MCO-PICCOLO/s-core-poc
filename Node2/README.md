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
sudo ./run.sh
```
- Run the script from the `pullpiri_LM` directory.
- `sudo` is required for copying files to system directories and setting permissions.

## Prerequisites
- Bazel must be installed and available in your PATH.
- The workspace should contain the required source files and Bazel build targets.
- `nodeagent` and `timpani-n`

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
After running this script, to continue the workflow:
1. Change directory to `~/s-core-poc/Node2/examples`:
   ```sh
   cd ~/s-core-poc/Node2/examples/
   ```
2. Run the following command:
   ```sh
   bash timpani.sh
   ```

---
**Note:** The script will stop and report errors if any step fails (due to `set -e`).
