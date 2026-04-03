# Health Monitor Integration for Mini-ADAS

This document describes the integration of Health Monitor (HM) and Launch Manager (LM) with the mini-adas FEO application.

## Overview

The mini-adas primary agent now integrates with the SCORE Health Monitor and Lifecycle Management system to provide:

- **Deadline monitoring**: Detects when FEO cycles exceed expected duration
- **Automatic recovery**: Launch Manager restarts failed processes
- **Process group coordination**: All mini-adas processes are managed as a coordinated group

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                   Launch Manager (LM)                           │
│  • Starts/stops processes based on RunTargets                   │
│  • Monitors process lifecycle via LifecycleClient API           │
│  • Triggers recovery when HM reports failures                   │
└──────────────┬──────────────────────────┬───────────────────────┘
               │                          │
               │ IPC                      │ IPC
               │                          │
    ┌──────────▼────────────┐  ┌──────────▼──────────────────────┐
    │  Health Monitor       │  │  adas_primary                   │
    │  (HM daemon)          │  │                                  │
    │                       │  │  Embeds HM lib:                 │
    │  • Reads alive CPs    │◄─┤  • Alive reporting              │
    │  • Evaluates deadlines│  │  • Deadline monitoring          │
    │  • Triggers recovery  │  │  • Reports RUNNING to LM        │
    │  • Sends to LM        │  │                                  │
    └───────────────────────┘  └──────────┬──────────────────────┘
                                          │ FEO Signaling
                                          │ (TCP)
                    ┌─────────────────────┼─────────────────────┐
                    │                     │                     │
        ┌───────────▼───────────┐ ┌───────▼───────────┐ ┌──────▼─────────┐
        │ adas_secondary (101)  │ │ adas_secondary   │ │  (future        │
        │                       │ │     (102)         │ │   recorders)    │
        └───────────────────────┘ └───────────────────┘ └────────────────┘
```

## How Failure Detection Works

### Normal Operation
1. **adas_primary** starts and initializes Health Monitor
2. Reports **RUNNING** state to Launch Manager
3. Connects to secondary agents (101, 102)
4. Runs FEO scheduler loop with deadline monitoring
5. Health Monitor sends periodic alive notifications to LM daemon

### When Secondary Agent Fails (e.g., timeout)
1. **Secondary agent** stops responding or times out
2. **FEO Primary** detects timeout in `wait_next_ready()` (currently 10s timeout)
3. **FEO cycle** takes longer than expected (> max_deadline_ms)
4. **Health Monitor** detects deadline violation
5. **HM daemon** stops sending alive notifications to LM
6. **Launch Manager** detects missing alive notifications
7. **LM triggers recovery**: Restart entire `mini_adas_group` process group

## Configuration

### 1. Health Monitor Configuration
**File**: `feo/examples/rust/mini-adas/etc/hm_config.json`

```json
{
  "deadline_window": {
    "min_ms": 200,
    "max_ms": 600
  },
  "health_monitor": {
    "internal_processing_cycle_ms": 50,
    "supervisor_api_cycle_ms": 50
  }
}
```

**Parameters**:
- `min_ms`: Minimum expected cycle time (50% of FEO cycle)
- `max_ms`: Maximum acceptable cycle time (150% of FEO cycle + grace)
- For 400ms FEO cycle: [200ms, 600ms] allows for timeout detection

**Tuning**:
- Increase `max_ms` if you need more tolerance for temporary delays
- Decrease `max_ms` for faster failure detection
- Adjust based on your FEO cycle time (passed as CLI argument)

### 2. Launch Manager Configuration
**File**: `lifecycle/examples/config/mini_adas_lifecycle.json`

**Key Features**:
- **Process Group**: All 3 processes (primary + 2 secondaries) form one group
- **Recovery Strategy**: `restart_process_group` ensures coordinated restart
- **Startup Order**: Secondaries start first, then primary connects
- **Alive Supervision**: HM evaluates every 50ms, tolerates 3 failed cycles

**Recovery Actions**:
```json
"recovery_action": {
  "restart_process_group": {
    "process_group": "mini_adas_group"
  }
}
```

When any process fails → entire group restarts.

## Building

### Prerequisites
- Both `feo` and `lifecycle` workspaces must be available
- Rust toolchain (2024 edition)
- Bazel (for bazel builds)

### Option 1: Cargo Build (Development)

From `feo/examples/rust/mini-adas` directory:

```bash
# Build with cargo (uses path dependencies to lifecycle workspace)
cargo build --release --bin adas_primary --features signalling_relayed_tcp

# The binary will be at:
# target/release/adas_primary
```

### Option 2: Bazel Build (Production)

**Note**: Bazel integration requires lifecycle workspace to be properly registered in MODULE.bazel or WORKSPACE.

```bash
cd feo/

# Build primary agent
bazel build --config=lint-rust //examples/rust/mini-adas:adas_primary

# Build secondary agents
bazel build --config=lint-rust //examples/rust/mini-adas:adas_secondary
```

## Running

### Full System with Launch Manager

1. **Start Launch Manager daemon**:
   ```bash
   cd lifecycle/
   # Configure and start LM daemon with mini_adas_lifecycle.json
   # (exact commands depend on your LM setup)
   ```

2. **Deploy binaries**:
   ```bash
   # Copy binaries to /opt/mini_adas/
   cp target/release/adas_primary /opt/mini_adas/
   cp target/release/adas_secondary /opt/mini_adas/
   cp etc/hm_config.json /opt/mini_adas/etc/
   ```

3. **Activate RunTarget**:
   ```bash
   # Use LM control client to switch to Running target
   lmcontrol activate Running
   ```

## Testing Failure Scenarios

### Simulate Secondary Timeout

1. Modify a secondary agent to introduce artificial delay:
   ```rust
   // In activity execution
   std::thread::sleep(Duration::from_secs(20)); // Exceeds FEO timeout
   ```

2. Rebuild and run
3. **Expected behavior**:
   - Primary FEO cycle hangs waiting for secondary
   - Cycle exceeds 600ms deadline
   - HM detects violation
   - LM restarts entire process group

### Simulate Secondary Crash

1. Kill a secondary process:
   ```bash
   pkill -9 adas_secondary
   ```

2. **Expected behavior**:
   - Primary FEO detects connection loss
   - Scheduler timeout triggers
   - HM detects extended cycle time
   - LM restarts process group

## Monitoring and Logs

### Health Monitor Logs
- HM reports deadline violations
- Check HM daemon logs for alive supervision status

### Primary Agent Logs
```
[INFO] Starting primary agent 100 with cycle time 400ms
[INFO] Loaded HM config: deadline window [200, 600] ms
[INFO] Health Monitor started
[INFO] Reported RUNNING state to Launch Manager
[INFO] Monitoring primary execution (heartbeat interval: 100ms)
[INFO] Starting FEO primary agent with health monitoring
```

### Launch Manager Logs
- Process state transitions
- Recovery action triggers
- Process group restart events

## Adjusting Deadline Thresholds

Edit `feo/examples/rust/mini-adas/etc/hm_config.json`:

**For 400ms FEO cycle**:
```json
{
  "deadline_window": {
    "min_ms": 200,    // 50% of cycle
    "max_ms": 600     // 150% of cycle
  }
}
```

**For 1000ms FEO cycle**:
```json
{
  "deadline_window": {
    "min_ms": 500,    // 50% of cycle
    "max_ms": 1500    // 150% of cycle
  }
}
```

**For faster detection** (more sensitive):
```json
{
  "deadline_window": {
    "min_ms": 300,
    "max_ms": 450     // Only 12.5% grace period
  }
}
```

## Troubleshooting

### Issue: "Failed to build Health Monitor"
- **Cause**: HM library not found
- **Solution**: Ensure lifecycle workspace is accessible at `../../../lifecycle/`

### Issue: "Failed to report execution state"
- **Cause**: Launch Manager daemon not running or LM config invalid
- **Solution**: Start LM daemon with the validated mini-adas config first

### Issue: Deadline violations but no recovery
- **Cause**: Not running under Launch Manager supervision
- **Solution**: Deploy via LM with proper configuration

### Issue: Too many false alarms
- **Cause**: `max_ms` threshold too tight
- **Solution**: Increase `max_ms` in hm_config.json

### Issue: Recovery too slow
- **Cause**: `max_ms` threshold too large
- **Solution**: Decrease `max_ms` for faster detection

## Implementation Details

### Modified Files

1. **`src/bin/adas_primary.rs`**
   - Added HM and lifecycle client imports
   - Added HM configuration loading
   - Initialize HM before Primary::run()
   - Report RUNNING state to LM
   - Monitoring thread tracks execution

2. **`Cargo.toml`**
   - Added `health_monitoring_lib` dependency
   - Added `lifecycle_client_rs` dependency
   - Added `serde_json` for config parsing

3. **`etc/hm_config.json`**
   - New file for HM configuration
   - Allows runtime tuning without rebuild

4. **`BUILD.bazel`**
   - Added hm_config.json as data dependency
   - Placeholder for lifecycle dependencies

### Code Flow

```rust
main()
  ├─ set_process_name()              // Set from PROCESSIDENTIFIER env
  ├─ load_hm_config()                 // Load JSON configuration
  ├─ initialize_health_monitor()      // Build HM + deadline monitor
  ├─ hm.start()                       // Start HM background thread
  ├─ report_execution_state_running() // Tell LM we're running
  ├─ Primary::new(config)             // Initialize FEO primary
  └─ run_with_health_monitor()        // Execute with monitoring
       ├─ deadline.start()            // Begin deadline window
       ├─ primary.run()               // Run FEO (blocks)
       └─ deadline.stop()             // End deadline (automatic via RAII)
```

## Future Enhancements

1. **Per-Secondary Monitoring**: Track individual secondary agents
2. **Dynamic Threshold Adjustment**: Auto-tune based on observed behavior
3. **Metrics Export**: Expose deadline violation counts, cycle times
4. **Health Dashboard**: Real-time visualization of FEO + HM status
5. **Graceful Degradation**: Continue with reduced functionality instead of full restart

## References

- [Health Monitor Library](../../lifecycle/src/health_monitoring_lib/)
- [Lifecycle Client API](../../lifecycle/src/launch_manager_daemon/lifecycle_client_lib/)
- [FEO Framework](../../src/feo/)
- [test_hari Example](../../lifecycle/test_hari/)
