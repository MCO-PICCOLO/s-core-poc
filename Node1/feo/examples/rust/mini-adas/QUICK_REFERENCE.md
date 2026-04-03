# Quick Reference: Mini-ADAS Health Monitor Integration

## File Locations

| Component | Location |
|-----------|----------|
| **Primary Binary Source** | `feo/examples/rust/mini-adas/src/bin/adas_primary.rs` |
| **HM Configuration** | `feo/examples/rust/mini-adas/etc/hm_config.json` |
| **LM Configuration** | `lifecycle/examples/config/mini_adas_lifecycle.json` |
| **Build Script** | `feo/examples/rust/mini-adas/build_and_test.sh` |
| **Documentation** | `feo/examples/rust/mini-adas/HEALTH_MONITOR_INTEGRATION.md` |
| **Implementation Summary** | `feo/examples/rust/mini-adas/IMPLEMENTATION_SUMMARY.md` |

## Quick Build

```bash
cd /home/lg/HARI/SCORE/feo/examples/rust/mini-adas
./build_and_test.sh
```

## Quick Run (Lifecycle-Managed)

```bash
# Terminal 1: Launch Manager
cd /tmp/mini_adas_lm
export LD_LIBRARY_PATH=/tmp/mini_adas_lm/lib
./launch_manager

# Terminal 2: Secondary 1
cd /home/lg/HARI/SCORE/feo/examples/rust/mini-adas
./run_secondary.sh 1

# Terminal 3: Secondary 2
cd /home/lg/HARI/SCORE/feo/examples/rust/mini-adas
./run_secondary.sh 2
```

## Key Configuration Values

### Health Monitor (etc/hm_config.json)
- **Deadline Min**: 200ms (50% of cycle)
- **Deadline Max**: 600ms (150% of cycle)
- **HM Processing**: 50ms
- **Supervisor API**: 50ms

### FEO Cycle Time (CLI argument)
- Default: 5000ms
- Recommended: 400ms
- Usage: `adas_primary <cycle_time_ms>`

### Launch Manager (mini_adas_lifecycle.json)
- **Process Group**: mini_adas_group
- **Startup Order**: secondary_1 → secondary_2 → primary
- **Recovery**: Restart entire group
- **Alive Evaluation**: 50ms

## Architecture at a Glance

```
Launch Manager
    ↓
Health Monitor Daemon ← (alive notifications)
    ↑
adas_primary (HM embedded) ← (deadline monitoring)
    ↓ TCP
adas_secondary_1 & adas_secondary_2
```

## When Failures Occur

**Secondary Timeout** → **FEO Cycle Exceeds 600ms** → **HM Detects Deadline** → **LM Restarts Group**

## Tuning Thresholds

Edit `etc/hm_config.json`:

**More Sensitive** (faster detection):
```json
{"deadline_window": {"min_ms": 300, "max_ms": 450}}
```

**More Tolerant** (reduce false alarms):
```json
{"deadline_window": {"min_ms": 200, "max_ms": 800}}
```

## Dependencies Added

- `health_monitoring_lib` → Health Monitor Rust library
- `lifecycle_client_rs` → Lifecycle Client API
- `serde_json` → JSON config parsing
- `signal-hook` → Graceful shutdown
- `libc` → Process name setting

## Environment Variables (for LM deployment)

- `PROCESSIDENTIFIER`: Process name for LM tracking
- `CONFIG_PATH`: HM configuration path (for supervised mode)
- `IDENTIFIER`: Process identifier

## Process States

1. **Not Started** → Launch Manager hasn't started process
2. **Starting** → Process launching
3. **Running** → Process reported RUNNING to LM
4. **Terminating** → Graceful shutdown in progress
5. **Failed** → HM detected violation, recovery triggered

## Recovery Flow

```
1. Secondary fails/times out
2. FEO primary cycle hangs (waiting for secondary)
3. Cycle exceeds 600ms (deadline max)
4. HM detects violation
5. HM daemon stops alive notifications
6. LM detects missing alive (after ~600ms)
7. LM executes recovery: restart_process_group
8. All 3 processes stopped in order
9. All 3 processes restarted in order
10. System returns to normal operation
```

## Common Commands

**Build**:
```bash
cargo build --release --bin adas_primary --features signalling_relayed_tcp
```

**Run with custom cycle**:
```bash
./target/release/adas_primary 1000  # 1 second FEO cycle
```

**Check if HM is working** (look for these logs):
```
[INFO] Loaded HM config: deadline window [200, 600] ms
[INFO] Health Monitor started
[INFO] Reported RUNNING state to Launch Manager
```

## Troubleshooting Quick Fixes

| Problem | Solution |
|---------|----------|
| Build fails (HM lib not found) | Check lifecycle workspace path: `../../../lifecycle/` |
| No recovery on failure | LM daemon not running or config not loaded |
| Too many false alarms | Increase `max_ms` in hm_config.json |
| Recovery too slow | Decrease `max_ms` in hm_config.json |
| Process name not set | Check `PROCESSIDENTIFIER` environment variable |

## Testing Checklist

- [ ] Build completes without errors
- [ ] Primary starts and loads HM config
- [ ] Primary reports RUNNING to LM
- [ ] All 3 processes communicate via TCP
- [ ] Kill secondary → observe primary timeout
- [ ] Deadline violation logged by HM
- [ ] LM triggers process group restart
- [ ] System recovers automatically

## Files Changed Summary

**Created**: 5 files (configs, docs, scripts)
**Modified**: 4 files (source, build, readme)
**Total Lines Added**: ~700+ lines

## Next Action Items

1. ✅ Review [IMPLEMENTATION_SUMMARY.md](IMPLEMENTATION_SUMMARY.md)
2. ✅ Build using `./build_and_test.sh`
3. ✅ Start under Launch Manager
4. ⬜ Add both secondaries to LM-managed process group
5. ⬜ Tune deadline thresholds based on observed behavior
6. ⬜ Implement HM-triggered LM recovery path
