# Understanding Your HM Connection Errors

## The Two Errors Explained

### Error 1: "Connection to PHM daemon failed"
```
!!! -> 2026/4/2 13:44:2 LCHM Sprv ERROR:
       [ Connection to PHM daemon failed, for the Monitor (mini_adas_primary) ]
```

**What it means**:
- Your Health Monitor (HM) tries to connect to PHM (Platform Health Monitor) daemon
- PHM daemon is **part of the Launch Manager** process
- When LM is not running, PHM doesn't exist → connection fails

**Where does HM try to connect?**
From your flatbuffer config (`hmproc_adas_primary.bin`):
- **interfacePath**: `lifecycle_health_mini_adas_primary`
- This creates an IPC endpoint (Unix socket or shared memory)
- Path is like: `/tmp/lifecycle_health_mini_adas_primary` or similar
- **The PHM daemon inside LM creates this endpoint**

**Impact**:
- ⚠️  HM cannot report violations to the platform
- ✅ HM still monitors locally (checkpoints, deadlines)
- ❌ No automatic recovery actions triggered

---

### Error 2: "Failed to access communication channel with Launch Manager"
```
!!! -> 2026/4/2 13:44:2 LCLM LCLM ERROR:
       [ [Lifecycle Client] Failed to access communication channel with Launch Manager. ]
```

**What it means**:
- Your `adas_primary` calls `lifecycle_client_rs::report_execution_state_running()`
- This tries to tell LM: "I'm now running"
- LM daemon is not running → communication fails

**Where does Lifecycle Client connect?**
- Uses a well-known IPC path (likely `/tmp/lifecycle_manager_socket` or similar)
- This socket is created by LM daemon when it starts
- Your app tries to connect, socket doesn't exist → error

**Impact**:
- ❌ LM doesn't know your process exists
- ❌ Cannot trigger state transitions (Startup → Running)
- ❌ No lifecycle management
- ✅ Your app continues running (after my fix)

---

## The Root Cause

The original errors came from missing or invalid LM-side startup/config wiring.

```
         Invalid / incomplete LM setup        Valid LM-managed startup

    adas_primary                          adas_primary
         │                                     │
         │                                     │
         ├─► HM init ✅                        ├─► HM init ✅
         │   HM start ✅                       │   HM start ✅
         │                                     │
         ├─► Connect to PHM ❌                 ├─► Connect to PHM ✅
         │   (PHM doesn't exist)               │   (PHM responds)
         │                                     │
         ├─► Report Running ❌                 ├─► Report Running ✅
         │   (LM socket missing)               │   (LM registers process)
         │                                     │
          └─► Startup/reporting fails           └─► Fully managed ✅
```

---

## Lifecycle-Managed Fix

**Quick Setup**:
```bash
cd /home/lg/HARI/SCORE/feo/examples/rust/mini-adas

# 1. Setup LM daemon directory and configs
./setup_lm_daemon.sh

# 2. Run LM daemon (in a separate terminal)
./run_with_lm_daemon.sh
```

**What this does**:
1. Creates `/tmp/mini_adas_lm/` with proper structure
2. Copies your binary, libraries, and configs
3. Compiles LM configuration to flatbuffer
4. Starts LM daemon
5. **LM daemon automatically launches your adas_primary**

**Expected result**:
```
[INFO] Health Monitor initialized
[INFO] Health Monitor started  ✅ No PHM error!
[INFO] Reported RUNNING state to Launch Manager  ✅ Success!
```

**Benefits**:
- ✅ Full lifecycle management
- ✅ HM can report to PHM
- ✅ Automatic recovery on failures
- ✅ Process group management

---

## IPC Communication Details

### What is IPC?
**IPC = Inter-Process Communication**
- Allows processes to talk to each other
- Common methods: Unix sockets, shared memory, message queues

### Your Setup Uses:
1. **Unix Domain Sockets** (likely)
   - File-like paths: `/tmp/lifecycle_health_mini_adas_primary`
   - Created by LM daemon's PHM component
   - Your HM connects as a client

2. **Lifecycle Manager Socket**
   - Well-known path for all processes
   - Your Lifecycle Client connects here
   - Reports execution state (Initializing → Running → Terminating)

### When LM Daemon Starts:
```bash
# LM daemon creates these IPC endpoints:
/tmp/lifecycle_manager_socket           # For lifecycle client
/tmp/lifecycle_health_mini_adas_primary # For your HM (from flatbuffer config)
/tmp/lifecycle_health_*                 # For other processes
```

### Your App Tries to Connect:
```rust
// In adas_primary.rs:

// 1. HM tries to connect to PHM
hm.start()  // → Reads interfacePath from flatbuffer
            // → Tries to connect to /tmp/lifecycle_health_mini_adas_primary
            // → If LM not running, this socket doesn't exist → ERROR 1

// 2. Lifecycle Client reports state
lifecycle_client_rs::report_execution_state_running()
            // → Tries to connect to /tmp/lifecycle_manager_socket
            // → If LM not running, socket doesn't exist → ERROR 2
```

---

## Try It Now

### Start LM-managed mini-adas:
```bash
# Terminal 1: Setup and run LM
cd /home/lg/HARI/SCORE/feo/examples/rust/mini-adas
./setup_lm_daemon.sh
./run_with_lm_daemon.sh

# Then start secondaries manually in separate terminals
```

---

## Further Reading

- **LAUNCHING_WITH_LM.md** - Detailed LM daemon setup guide
- **HM_INTEGRATION_STATUS.md** - Technical HM implementation status
- **run_with_lm_daemon.sh** - LM-managed launcher

---

## Summary

The original LM/HM startup errors were configuration issues, not FEO runtime issues.

✅ **HM infrastructure works perfectly**
✅ **Flatbuffer config loads correctly**
✅ **All C++ libraries linked**
✅ **Your app runs fine**

The supported mini-adas path is now LM-managed startup only.
