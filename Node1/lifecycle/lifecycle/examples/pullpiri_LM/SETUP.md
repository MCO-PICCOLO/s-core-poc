# New System Setup Guide — Pullpiri LM Demo

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
## Timpani Dependencies
sudo apt install -y libgrpc++-dev libprotobuf-dev protobuf-compiler-grpc
sudo apt install -y libsystemd-dev
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
cd s-core-poc/Node1/pullpiri
in each of below components run 

cd src/server/persistency-service/
cargo build --release
cd ../apiserver/
cargo build --release
cd ../monitoringserver/
cargo build --release
cd ../policymanager/
cargo build --release
cd ../../player/actioncontroller/
cargo build --release
cd ../filtergateway
cargo build --release
cd ../statemanager
cargo build --release

cd ../

sudo cp target/release/persistency-service /opt/pullpiri/bin/
sudo cp target/release/apiserver            /opt/pullpiri/bin/
sudo cp target/release/monitoringserver     /opt/pullpiri/bin/
sudo cp target/release/statemanager         /opt/pullpiri/bin/
sudo cp target/release/filtergateway        /opt/pullpiri/bin/
sudo cp target/release/actioncontroller     /opt/pullpiri/bin/
sudo cp target/release/policymanager        /opt/pullpiri/bin/
#sudo cp target/release/nodeagent            /opt/pullpiri/bin/
sudo chmod +x /opt/pullpiri/bin/*
```

### Timpani binaries

`timpani-o`  must also be present in `/opt/pullpiri/bin/`.  
Build them from the **Timpani** repo:
cd ../../TIMPANI/timpani-o
mkdir build
cd build
cmake ..
make

```bash
sudo cp -f timpani-o /opt/pullpiri/bin/
#sudo cp <timpani_build_dir>/timpani-n /opt/pullpiri/bin/
sudo chmod +x /opt/pullpiri/bin/timpani-o 
#sudo chmod +x /opt/pullpiri/bin/timpani-n
```

---

## 4. Build adas_primary C++ Shared Libraries

The `adas_primary` binary (built with the `lifecycle` feature) links against several
C++ shared libraries that are produced by the S-CORE Lifecycle Bazel build.  
They must be built and copied into `feo/examples/rust/mini-adas/lib/` **before**
building `adas_primary`.

> **Quick path:** run the helper script once and skip the manual steps below:
> ```bash
> cd ~/s-core-poc/Node1/feo/examples/rust/mini-adas
> mkdir lib
> cd ~/s-core-poc/Node1/lifecycle/lifecycle/examples/pullpiri_LM
> chmod +x build_adas_libs.sh
> ./build_adas_libs.sh
> ```
 
 ### or follow below 4a, 4b, 4c steps manually
### 4a. Build the `.so` targets manually

```bash
cd /home/acrn/new_ak/vso_score/lifecycle/lifecycle

bazel build --config=x86_64-linux \
  //src/launch_manager_daemon/common:all \
  //src/launch_manager_daemon/health_monitor_lib:phm_logging \
  //src/launch_manager_daemon/health_monitor_lib:timers \
  //src/launch_manager_daemon/health_monitor_lib:hm-lib \ 
  //src/launch_manager_daemon/health_monitor_lib:hm_shared_lib \
  //src/launch_manager_daemon/process_state_client_lib:process_state_client \
  //src/launch_manager_daemon/lifecycle_client_lib:lifecycle_client
```

### 4b. Copy `.so` files into mini-adas `lib/`

```bash
DEST=~/s-core-poc/Node1/feo/examples/rust/mini-adas/lib
BAZEL_BIN=~/s-core-poc/Node1/lifecycle/lifecycle/bazel-bin

cp $BAZEL_BIN/src/launch_manager_daemon/common/libcommon.so                                  $DEST/
cp $BAZEL_BIN/src/launch_manager_daemon/common/libosal.so                                    $DEST/
cp $BAZEL_BIN/src/launch_manager_daemon/common/libidentifier_hash.so                         $DEST/
cp $BAZEL_BIN/src/launch_manager_daemon/health_monitor_lib/libphm_logging.so                 $DEST/
cp $BAZEL_BIN/src/launch_manager_daemon/health_monitor_lib/libtimers.so                      $DEST/
cp $BAZEL_BIN/src/launch_manager_daemon/process_state_client_lib/libprocess_state_client.so  $DEST/
cp $BAZEL_BIN/src/launch_manager_daemon/lifecycle_client_lib/liblifecycle_client.so          $DEST/
cp $BAZEL_BIN/src/launch_manager_daemon/health_monitor_lib/libhm-lib.a                       $DEST/
```

### 4c. Verify

```bash
ls -lh ~/s-core-poc/Node1/feo/examples/rust/mini-adas/lib/
```

| Library | Approx. size |
|---|---|
| `libcommon.so` | ~17 KB |
| `libosal.so` | ~24 KB |
| `libidentifier_hash.so` | ~344 KB |
| `liblifecycle_client.so` | ~129 KB |
| `libprocess_state_client.so` | ~152 KB |
| `libphm_logging.so` | ~1.5 MB |
| `libtimers.so` | ~1.5 MB |

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

> **Note:** `run.sh` expects the `.so` libraries to already be present in
> `feo/examples/rust/mini-adas/lib/`. Run `build_adas_libs.sh` (section 4 above)
> before the first `run.sh` invocation.

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
| `lib*.so` (mini-adas) | `feo/examples/rust/mini-adas/lib/` | `build_adas_libs.sh` |
| `lib*.so` (runtime) | `/opt/pullpiri/lib/` | `run.sh` step 1.5 |
| Pullpiri service binaries | `/opt/pullpiri/bin/` | Manual (section 3 above) |

---

## 7. Verifying the Setup

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

---
