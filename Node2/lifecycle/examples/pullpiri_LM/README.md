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
- `nodeagent` and `timpani-n` binaries should be present at the workspace root if you want them copied to `/opt/pullpiri/bin`.

## What Happens Next?
After running this script, to continue the workflow:
1. Change directory to `/home/lg/akshay/vso_score_node2/examples`:
   ```sh
   cd /home/lg/akshay/vso_score_node2/examples
   ```
2. Run the following command:
   ```sh
   bash timpani.sh
   ```

---
**Note:** The script will stop and report errors if any step fails (due to `set -e`).
