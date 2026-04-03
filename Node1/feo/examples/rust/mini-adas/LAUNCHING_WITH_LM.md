# Running Mini-ADAS with Launch Manager Daemon

## Understanding the Architecture

### IPC Communication Flow
```
                    ┌─────────────────────────┐
                    │  Launch Manager (LM)    │
                    │  Daemon Process         │
                    ├─────────────────────────┤
                    │ • Process Management    │
                    │ • Recovery Actions      │
                    │ • PHM Integration       │
                    └──────────┬──────────────┘
                               │ IPC Sockets
                               │
        ┌──────────────────────┼──────────────────────┐
        │                      │                      │
        ▼                      ▼                      ▼
┌─────────────┐      ┌─────────────────┐    ┌─────────────┐
│ Lifecycle   │      │ Health Monitor  │    │ Process     │
│ Client      │      │ (HM/PHM)        │    │ State       │
│             │      │                 │    │ Client      │
│ Reports     │      │ Supervises      │    │             │
│ exec state  │      │ deadlines       │    │ Reports     │
└──────┬──────┘      └────────┬────────┘    │ state       │
       │                      │              └──────┬──────┘
       └──────────────┬───────┴─────────────────────┘
                      │
              ┌───────▼────────┐
              │  adas_primary  │
              │  (Your App)    │
              └────────────────┘
```

### Current Integration State

**Error 1**: `Connection to PHM daemon failed`
- **Cause**: PHM endpoint wiring is not fully aligned yet between LM-side HM machine config and the process HM config
- **Impact**: The primary starts and reports RUNNING to LM, but HM is not yet reporting violations through PHM
- **Current Status**: LM-managed startup works; HM-to-PHM reporting still needs completion

**Error 2**: `Failed to access communication channel with Launch Manager`
- **Cause**: LM daemon was not running or config was not valid
- **Impact**: Lifecycle reporting failed and the process was not LM-managed
- **Current Status**: Resolved for the current LM-managed startup flow

### IPC Paths (from your flatbuffer config)

Your `hmproc_adas_primary.bin` defines:
```
interfacePath: "lifecycle_health_mini_adas_primary"
```

This tells HM to connect via IPC socket at path like:
- `/tmp/lifecycle_health_mini_adas_primary` (Unix domain socket)
- Or shared memory segment with this identifier

**The LM daemon must be running to create these IPC endpoints!**

## Option 1: Running with Full LM Stack (Complete Integration)

### Prerequisites
```bash
cd /home/lg/HARI/SCORE/lifecycle

# 1. Verify LM daemon binary exists
ls -lh bazel-bin/src/launch_manager_daemon/launch_manager
# Should show: 3.0M launch_manager binary

# 2. Check flatbuffer configs
ls -lh bazel-bin/examples/flatbuffer_out/*.bin
# Should show: lm_demo.bin, hm_demo.bin, hmcore.bin, etc.
```

### Setup Directory Structure

LM daemon expects configs in `etc/` relative to binary location:
```bash
# Create mini-adas LM deployment directory
mkdir -p /tmp/mini_adas_lm/{etc,mini_adas,lib}

# Copy LM daemon
cp /home/lg/HARI/SCORE/lifecycle/bazel-bin/src/launch_manager_daemon/launch_manager \
   /tmp/mini_adas_lm/

# Copy libraries
cp /home/lg/HARI/SCORE/feo/examples/rust/mini-adas/lib/*.so \
   /tmp/mini_adas_lm/lib/

# Copy your binary
cp /home/lg/HARI/SCORE/feo/target/release/adas_primary \
   /tmp/mini_adas_lm/mini_adas/

# Copy all configs to etc/ (LM expects them here)
cp /home/lg/HARI/SCORE/feo/examples/rust/mini-adas/etc/hmproc_adas_primary.bin \
   /tmp/mini_adas_lm/etc/

# Copy HM core configs (required by LM daemon's PHM)
cp /home/lg/HARI/SCORE/lifecycle/bazel-bin/examples/flatbuffer_out/hm_demo.bin \
   /tmp/mini_adas_lm/etc/
cp /home/lg/HARI/SCORE/lifecycle/bazel-bin/examples/flatbuffer_out/hmcore.bin \
   /tmp/mini_adas_lm/etc/
```

### Create LM Configuration

Create `/tmp/mini_adas_lm/etc/lm_mini_adas.json`:
```json
{
    "LM_version_major": 8,
    "LM_version_minor": 0,
    "processes": [
        {
            "identifier": "mini_adas_primary",
            "path": "mini_adas/adas_primary",
            "args": ["40"],
            "env": {
                "LD_LIBRARY_PATH": "/tmp/mini_adas_lm/lib",
                "IDENTIFIER": "mini_adas_primary",
                "CONFIG_PATH": "/tmp/mini_adas_lm/etc/hmproc_adas_primary.bin"
            },
            "uid": 1000,
            "gid": 1000
        }
    ],
    "process_groups": [
        {
            "identifier": "mini_adas_group",
            "processes": ["mini_adas_primary"]
        }
    ]
}
```

### Compile LM Config to Flatbuffer

```bash
cd /tmp/mini_adas_lm/etc

# Use the flatc compiler we built earlier
/home/lg/HARI/SCORE/lifecycle/bazel-bin/external/flatbuffers+/flatc \
    -b \
    -o . \
    /home/lg/HARI/SCORE/lifecycle/src/launch_manager_daemon/config/lm_flatcfg.fbs \
    lm_mini_adas.json

# Verify binary was created
ls -lh lm_mini_adas.bin
```

### Run Launch Manager Daemon

**Terminal 1: Launch Manager**
```bash
cd /tmp/mini_adas_lm
export LD_LIBRARY_PATH=/tmp/mini_adas_lm/lib

# Run LM daemon (it will start your adas_primary)
    ./launch_manager
```

Expected logs:
```
[LM] Launch Manager starting...
[LM] Loading configuration from etc/lm_mini_adas.bin
[LM] Starting process: mini_adas_primary
[LM] PHM daemon initialized
[LM] All processes started
```

### Expected Behavior with LM Running

Your adas_primary will now show:
```
[INFO] Starting primary agent Agt-100 with cycle time Duration(400ms)
[INFO] Loaded HM config: deadline window [200, 600] ms
[INFO] Health Monitor initialized
[INFO] Health Monitor started  ✅ (No PHM connection error!)
[INFO] Reported RUNNING state to Launch Manager  ✅ (Success!)
```

**Benefits**:
- ✅ HM can report violations to LM
- ✅ LM can trigger recovery actions (restart process/group)
- ✅ Full lifecycle management
- ✅ Integrated deadline supervision

## Current Test Flow

Use the LM-managed startup path only:

1. Start `launch_manager` from `/tmp/mini_adas_lm`
2. Let LM launch `adas_primary`
3. Start secondary 1 with `./run_secondary.sh 1`
4. Start secondary 2 with `./run_secondary.sh 2`
5. Trigger a failure on one secondary and observe the primary-side timeout / monitoring behavior

## Troubleshooting

### Issue: "Connection to PHM daemon failed"
**Diagnosis**:
```bash
# Check if LM daemon is running
ps aux | grep launch_manager
```

**Fix**: Start LM daemon with the generated `lm_demo.bin` and `hm_demo.bin`

### Issue: "Failed to access communication channel with Launch Manager"
**Diagnosis**:
```bash
# Check IPC endpoints
ls -la /tmp/lifecycle_* 2>/dev/null
ls -la /run/lifecycle_* 2>/dev/null
```

**Fix**: LM daemon creates these when it starts

### Issue: "Failed to read flatbuffer file"
**Diagnosis**:
```bash
# Verify CONFIG_PATH is set
echo $CONFIG_PATH

# Verify file exists and is readable
ls -lh $CONFIG_PATH
file $CONFIG_PATH  # Should show "data"
```

**Fix**:
```bash
export CONFIG_PATH="/full/path/to/hmproc_adas_primary.bin"
```

## Summary

### Your Current Status ✅
- LM startup is working and launches `adas_primary`
- Flatbuffer config is created and loading
- Primary and secondaries connect and execute task chains under the LM-managed setup

### To Get Full LM Integration
1. Create LM deployment directory structure
2. Create LM JSON config with your process definition
3. Compile to flatbuffer binary
4. Run LM daemon (Terminal 1)
5. LM daemon launches your adas_primary automatically

### Recommended Next Step

**For HM deadline detection testing**: Fix per-cycle checkpoint reporting first, then extend the LM process group to include both secondaries.

**For full lifecycle management**: Keep the LM-managed startup path as the only supported execution path for mini-adas.
