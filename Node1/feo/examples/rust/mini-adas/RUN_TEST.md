# Running Mini-ADAS with Health Monitor

## ⚠️ CRITICAL: Enable Lifecycle Feature

**You MUST use `--features lifecycle` or Health Monitor won't run!**

Quick command:
```bash
cd /home/lg/HARI/SCORE/feo/examples/rust/mini-adas
export LD_LIBRARY_PATH=$(pwd)/lib:$LD_LIBRARY_PATH
RUSTFLAGS="-L $(pwd)/lib" RUST_LOG=info,health_monitoring_lib=debug \
  cargo run --release --bin adas_primary --features signalling_relayed_tcp,lifecycle -- 400
```

**CRITICAL:** `RUSTFLAGS="-L $(pwd)/lib"` is required for build-time linking of C++ libraries.

Verify HM is active by checking for log message: `Health Monitor started`

---

## Build Summary

✅ **Successfully built adas_primary with lifecycle (Health Monitor) integration!**

The build now includes all required C++ lifecycle libraries:
- libcommon.so (17KB)
- libosal.so (24KB)
- libidentifier_hash.so (344KB)
- liblifecycle_client.so (129KB)
- libprocess_state_client.so (152KB)
- libphm_logging.so (1.5MB)
- libtimers.so (1.5MB)
- libhm-lib.a (345KB)

## Quick Test (3 Terminals)

### Easy Way - Use Helper Scripts

**Terminal 1:**
```bash
cd /home/lg/HARI/SCORE/feo/examples/rust/mini-adas
./run_primary_with_hm.sh 400
```

**Terminal 2:**
```bash
cd /home/lg/HARI/SCORE/feo/examples/rust/mini-adas
./run_secondary.sh 1
```

**Terminal 3:**
```bash
cd /home/lg/HARI/SCORE/feo/examples/rust/mini-adas
./run_secondary.sh 2
```

### Manual Way - Full Commands

### Terminal 1 - Primary FEO Agent (with Health Monitor)
```bash
cd /home/lg/HARI/SCORE/feo/examples/rust/mini-adas
export LD_LIBRARY_PATH=$(pwd)/lib:$LD_LIBRARY_PATH
RUSTFLAGS="-L $(pwd)/lib" RUST_LOG=info,health_monitoring_lib=debug \
  cargo run --release --bin adas_primary --features signalling_relayed_tcp,lifecycle -- 400
```

**CRITICAL NOTES:**
- `RUSTFLAGS="-L $(pwd)/lib"` is required for build-time linking
- `LD_LIBRARY_PATH` is required for runtime linking
- `--features signalling_relayed_tcp,lifecycle` enables Health Monitor

### Terminal 2 - Secondary FEO Agent #1
```bash
cd /home/lg/HARI/SCORE/feo/examples/rust/mini-adas
cargo run --release --bin adas_secondary --features signalling_relayed_tcp -- 1
```

### Terminal 3 - Secondary FEO Agent #2
```bash
cd /home/lg/HARI/SCORE/feo/examples/rust/mini-adas
cargo run --release --bin adas_secondary --features signalling_relayed_tcp -- 2
```

## What to Expect

**Verification - Check HM is Running:**
When starting the primary, you MUST see these log messages:
```
INFO  adas_primary] Loaded HM config: deadline window [200, 600] ms
INFO  adas_primary] Health Monitor initialized
INFO  adas_primary] Health Monitor started
INFO  adas_primary] Reported RUNNING state to Launch Manager
INFO  adas_primary] Starting deadline monitoring for FEO execution
```

If the primary does not show HM initialization and LM reporting logs, verify that you built it with the `lifecycle` feature.

**Normal Operation:**
- Primary coordinates the cycle execution every 400ms
- Health Monitor in primary supervises deadline compliance
- All 3 processes communicate via TCP signaling

**Deadline Miss Scenario:**
To trigger a deadline violation and test Health Monitor:

1. **Method 1 - Kill a secondary:**
   ```bash
   pkill -9 adas_secondary
   ```
   The primary will timeout waiting for response → HM detects violation

2. **Method 2 - Simulate slow response:**
   Modify a secondary's activity to sleep 15 seconds (exceeds deadline window)

**Expected HM Behavior:**
- Deadline window configured: 200ms minimum, 600ms maximum
- If cycle completes outside this window, HM logs violation
- HM internal processing: 50ms cycles
- Note: Full LM integration (restart recovery) requires Launch Manager daemon running

## Configuration Files

- **HM Config:** `etc/hm_config.json` - deadline thresholds (tunable without rebuild)
- **LM Config:** `../../../../lifecycle/examples/config/mini_adas_lifecycle.json` - process group definition

## Binary Locations

- Primary: `/home/lg/HARI/SCORE/feo/target/release/adas_primary` (8.7MB with lifecycle)
- Secondary: `/home/lg/HARI/SCORE/feo/target/release/adas_secondary`

## Build Commands

**With Health Monitor (lifecycle feature):**
```bash
cd /home/lg/HARI/SCORE/feo/examples/rust/mini-adas
RUSTFLAGS="-L $(pwd)/lib" cargo build --release --bin adas_primary --features signalling_relayed_tcp,lifecycle
```

**Secondaries:**
```bash
cargo build --release --bin adas_secondary --features signalling_relayed_tcp
```

## Troubleshooting

**❌ HM initialization logs do not appear:**
- You likely forgot the `--features lifecycle` flag
- Health Monitor is not active in that build
- Solution: Add `--features signalling_relayed_tcp,lifecycle` to the cargo command

**✅ Correct log output should show:**
```
INFO  adas_primary] Loaded HM config: deadline window [200, 600] ms
INFO  adas_primary] Health Monitor initialized
INFO  adas_primary] Health Monitor started
INFO  adas_primary] Reported RUNNING state to Launch Manager
INFO  adas_primary] Starting deadline monitoring for FEO execution
```

**Note:** The supported run path is under Launch Manager. Use LM-managed startup when validating lifecycle behavior.

**If you see "error while loading shared libraries":**
```bash
export LD_LIBRARY_PATH=/home/lg/HARI/SCORE/feo/examples/rust/mini-adas/lib:$LD_LIBRARY_PATH
```

**If you see "unable to find library -llifecycle_client" during build:**
- You forgot `RUSTFLAGS="-L $(pwd)/lib"` in the cargo command
- This is needed for build-time linking of C++ libraries
- Both `RUSTFLAGS` (build-time) and `LD_LIBRARY_PATH` (runtime) are required

**Why both environment variables?**
- `RUSTFLAGS="-L path"` tells the linker where to find libraries during compilation
- `LD_LIBRARY_PATH=path` tells the dynamic linker where to find .so files when running the binary

**To rebuild C++ libraries (if needed):**
```bash
cd /home/lg/HARI/SCORE/lifecycle
bazel build //src/launch_manager_daemon/lifecycle_client_lib:lifecycle_client \
             //src/launch_manager_daemon/process_state_client_lib:process_state_client \
             //src/launch_manager_daemon/health_monitor_lib:hm_shared_lib
```

## Next Steps

1. ✅ Validate LM startup and LM-launched primary
2. ✅ Validate both secondaries connect to the primary
3. ⏳ Simulate deadline miss and observe HM detection
4. ⏳ Extend LM config so all three processes are LM-managed
