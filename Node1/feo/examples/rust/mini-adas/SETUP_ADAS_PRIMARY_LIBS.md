# Setup — adas_primary C++ Shared Libraries

This guide rebuilds the C++ `.so` libraries required by `adas_primary` (when built
with the `lifecycle` feature) from the S-CORE Lifecycle repo and copies them into
the `lib/` directory of this example.

---

## Library List

| File | Source target |
|---|---|
| `libcommon.so` | `//src/launch_manager_daemon/common:common` |
| `libosal.so` | `//src/launch_manager_daemon/common:osal` |
| `libidentifier_hash.so` | `//src/launch_manager_daemon/common:identifier_hash` |
| `libphm_logging.so` | `//src/launch_manager_daemon/health_monitor_lib:phm_logging` |
| `libtimers.so` | `//src/launch_manager_daemon/health_monitor_lib:timers` |
| `libprocess_state_client.so` | `//src/launch_manager_daemon/process_state_client_lib:process_state_client` |
| `liblifecycle_client.so` | `//src/launch_manager_daemon/lifecycle_client_lib:lifecycle_client` |

---

## Step 1 — Build all `.so` targets from lifecycle

```bash
cd /home/acrn/new_ak/vso_score/lifecycle/lifecycle

bazel build --config=x86_64-linux \
  //src/launch_manager_daemon/common:all \
  //src/launch_manager_daemon/health_monitor_lib:phm_logging \
  //src/launch_manager_daemon/health_monitor_lib:timers \
  //src/launch_manager_daemon/health_monitor_lib:hm_shared_lib \
  //src/launch_manager_daemon/process_state_client_lib:process_state_client \
  //src/launch_manager_daemon/lifecycle_client_lib:lifecycle_client
```

---

## Step 2 — Copy `.so` files into mini-adas `lib/`

```bash
DEST=/home/acrn/new_ak/vso_score/feo/examples/rust/mini-adas/lib
BAZEL_BIN=/home/acrn/new_ak/vso_score/lifecycle/lifecycle/bazel-bin

cp $BAZEL_BIN/src/launch_manager_daemon/common/libcommon.so                              $DEST/
cp $BAZEL_BIN/src/launch_manager_daemon/common/libosal.so                                $DEST/
cp $BAZEL_BIN/src/launch_manager_daemon/common/libidentifier_hash.so                     $DEST/
cp $BAZEL_BIN/src/launch_manager_daemon/health_monitor_lib/libphm_logging.so             $DEST/
cp $BAZEL_BIN/src/launch_manager_daemon/health_monitor_lib/libtimers.so                  $DEST/
cp $BAZEL_BIN/src/launch_manager_daemon/process_state_client_lib/libprocess_state_client.so  $DEST/
cp $BAZEL_BIN/src/launch_manager_daemon/lifecycle_client_lib/liblifecycle_client.so          $DEST/
```

> **Tip:** If a copy fails, locate the exact output path first:
> ```bash
> find /home/acrn/new_ak/vso_score/lifecycle/lifecycle/bazel-bin -name "libcommon.so"
> ```

---

## Step 3 — Verify

```bash
ls -lh /home/acrn/new_ak/vso_score/feo/examples/rust/mini-adas/lib/
```

Expected output (approximate sizes):

```
libcommon.so              ~17 KB
libosal.so                ~24 KB
libidentifier_hash.so    ~344 KB
liblifecycle_client.so   ~129 KB
libprocess_state_client.so ~152 KB
libphm_logging.so          ~1.5 MB
libtimers.so               ~1.5 MB
```

---

## Step 4 — Build adas_primary with lifecycle feature

```bash
cd /home/acrn/new_ak/vso_score/feo/examples/rust/mini-adas

RUSTFLAGS="-L $(pwd)/lib" cargo build --release \
  --bin adas_primary \
  --features signalling_relayed_tcp,lifecycle
```

---

## Step 5 — Run adas_primary

```bash
cd /home/acrn/new_ak/vso_score/feo/examples/rust/mini-adas

export LD_LIBRARY_PATH=$(pwd)/lib:$LD_LIBRARY_PATH

RUSTFLAGS="-L $(pwd)/lib" RUST_LOG=info \
  cargo run --release \
  --bin adas_primary \
  --features signalling_relayed_tcp,lifecycle -- 400
```
