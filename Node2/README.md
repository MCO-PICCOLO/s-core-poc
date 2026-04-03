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


Node-2-># New System Setup Guide — Pullpiri LM Demo

This guide covers everything you need to set up a fresh machine to run the
Pullpiri + S-CORE Lifecycle Launch Manager demo.

---

## Repository Layout (Expected)

```
vso_score/
├── lifecycle/lifecycle/    ← this repo  (WORKSPACE_ROOT)
└── feo/                    ← mini-adas repo (MINI_ADAS_DIR = ../feo from lifecycle/)
```

`run.sh` derives `MINI_ADAS_DIR` as `$WORKSPACE_ROOT/../../feo`.
If your checkout differs, export `MINI_ADAS_DIR` before running the script.

---

## 1. System Prerequisites

| Tool | Version | Install |
|---|---|---|
| Bazel | `8.4.2` (see `.bazelversion`) | [bazelisk](https://github.com/bazelbuild/bazelisk) recommended |
| Rust / Cargo | `1.90.0` (see `rust-toolchain.toml`) | `rustup toolchain install 1.90.0` |
| Java | 17 | `sudo apt install openjdk-17-jdk` |
| `sudo` access | — | needed to write to `/opt/pullpiri/` |

> Bazelisk will read `.bazelversion` and download the correct Bazel automatically.
> Install bazelisk and symlink it as `bazel`:
> ```bash
> sudo wget -O /usr/local/bin/bazel \
>   https://github.com/bazelbuild/bazelisk/releases/latest/download/bazelisk-linux-amd64
> sudo chmod +x /usr/local/bin/bazel
> ```

---

## 2. One-Time Directory Setup

Create the `/opt/pullpiri` tree before running the demo for the first time:

```bash
sudo mkdir -p /opt/pullpiri/bin
sudo mkdir -p /opt/pullpiri/bin/etc   # adas_primary reads hm_config.json from here
sudo mkdir -p /opt/pullpiri/lib       # shared .so files
sudo mkdir -p /opt/pullpiri/etc       # hmproc_adas_primary.bin lives here
```

---

## 3. Install Pullpiri Service Binaries

The Launch Manager starts these binaries from `/opt/pullpiri/bin/`.
Build them from the **Pullpiri** repo and copy them:

```bash
# Inside the pullpiri repo
cargo build --release
sudo cp target/release/persistency-service /opt/pullpiri/bin/
sudo cp target/release/apiserver            /opt/pullpiri/bin/
sudo cp target/release/monitoringserver     /opt/pullpiri/bin/
sudo cp target/release/statemanager         /opt/pullpiri/bin/
sudo cp target/release/filtergateway        /opt/pullpiri/bin/
sudo cp target/release/actioncontroller     /opt/pullpiri/bin/
sudo cp target/release/policymanager        /opt/pullpiri/bin/
sudo cp target/release/nodeagent            /opt/pullpiri/bin/
sudo chmod +x /opt/pullpiri/bin/*
```

### Timpani binaries

`timpani-o` and `timpani-n` must also be present in `/opt/pullpiri/bin/`.
Build them from the **Timpani** repo:

```bash
sudo cp <timpani_build_dir>/timpani-o /opt/pullpiri/bin/
sudo cp <timpani_build_dir>/timpani-n /opt/pullpiri/bin/
sudo chmod +x /opt/pullpiri/bin/timpani-o /opt/pullpiri/bin/timpani-n
```

---

## 4. Fix the Hardcoded Timpani Path

`pullpiri_lm_config.json` contains a **hardcoded absolute path** for timpani-o's
node configuration:

```json
"process_arguments": [
    ...
    "/home/acrn/new_ak/vso_score/pullpiri/examples/resources/timpani/node_configurations.yaml"
]
```

Change this to the correct absolute path on your machine before building:

```bash
# Example: replace with your actual pullpiri checkout path
sed -i 's|/home/acrn/new_ak/vso_score/pullpiri|/path/to/your/pullpiri|g' \
    examples/pullpiri_LM/config/pullpiri_lm_config.json
```

---

## 5. Build & Run

From the lifecycle repo root (`lifecycle/lifecycle/`):

```bash
cd examples/pullpiri_LM
sudo ./run.sh
```

`run.sh` does the following automatically:

| Step | What |
|---|---|
| 1   | Bazel build: lifecycle flatbuffer configs + `launch_manager` binary |
| 1.5 | Cargo build: `adas_primary` + `adas_secondary` with `lifecycle` feature |
| 2   | Copy flatbuffer `.bin` files → `lifecycle/lifecycle/etc/` |
| 3   | Copy logging configs → `lifecycle/lifecycle/etc/` |
| 4   | Copy `hm_config.json` → `/opt/pullpiri/bin/etc/` and `hmproc_adas_primary.bin` → `/opt/pullpiri/etc/` |
| 5   | `exec launch_manager` |

---

## 6. Runtime File Locations Summary

| File | Location | Populated by |
|---|---|---|
| `launch_manager` binary | `bazel-bin/src/launch_manager_daemon/` | Bazel build |
| `lm_demo.bin`, `hm_demo.bin`, `hmcore.bin`, `hmproc_adas_primary.bin` | `lifecycle/lifecycle/etc/` | `run.sh` step 2 |
| `logging.json`, `ecu_logging_config.json` | `lifecycle/lifecycle/etc/` | `run.sh` step 3 |
| `hm_config.json` | `/opt/pullpiri/bin/etc/` | `run.sh` step 4 |
| `hmproc_adas_primary.bin` | `/opt/pullpiri/etc/` | `run.sh` step 4 |
| `adas_primary`, `adas_secondary` | `/opt/pullpiri/bin/` | `run.sh` step 1.5 |
| `lib*.so` (mini-adas) | `/opt/pullpiri/lib/` | `run.sh` step 1.5 |
| Pullpiri service binaries | `/opt/pullpiri/bin/` | Manual (section 3 above) |

---

## 7. Verifying the Setup

```bash
# All required binaries present
ls /opt/pullpiri/bin/{persistency-service,apiserver,monitoringserver,statemanager,\
filtergateway,actioncontroller,policymanager,nodeagent,timpani-o,timpani-n,\
adas_primary,adas_secondary}

# Shared libraries present
ls /opt/pullpiri/lib/lib*.so

# Config files present
ls /opt/pullpiri/bin/etc/hm_config.json
ls /opt/pullpiri/etc/hmproc_adas_primary.bin
ls lifecycle/lifecycle/etc/{lm_demo.bin,hm_demo.bin,hmcore.bin,logging.json}
```

---

## 8. Common Failure Modes

| Symptom | Likely cause |
|---|---|
| `launch_manager: not found` | Bazel build failed or wrong `BAZEL_BIN` path |
| `adas_primary` fails to start | Missing `.so` in `/opt/pullpiri/lib/` or wrong `CONFIG_PATH` |
| Alive supervision EXPIRED immediately | `supervisor_api_cycle_ms` ≠ `reporting_cycle × 1000` in config |
| `timpani-o` crash on start | Hardcoded `node_configurations.yaml` path not updated (section 4) |
| Permission denied on `/opt/pullpiri/` | `run.sh` must be run with `sudo` |

---

For a detailed explanation of `hmproc_adas_primary.bin` generation, see
[HMPROC_BIN.md](HMPROC_BIN.md).

GitHub - bazelbuild/bazelisk: A user-friendly launcher for Bazel.
A user-friendly launcher for Bazel. Contribute to bazelbuild/bazelisk development by creating an account on GitHub.
