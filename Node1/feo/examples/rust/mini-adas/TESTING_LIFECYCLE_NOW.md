# FULL LIFECYCLE INTEGRATION - TEST PLAN

## IMMEDIATE TESTING (Ready NOW)

### Option 1: Test with test_hari Example

test_hari already demonstrates Health Monitor + Launch Manager integration.
This is the FASTEST way to see the full system working.

#### Step 1: Build test_hari

cd /home/lg/HARI/SCORE/lifecycle
bazel build //test_hari:deadline_miss_app

#### Step 2: Start Launch Manager

Terminal 1 - Launch Manager Daemon:
cd /home/lg/HARI/SCORE/lifecycle/examples/tmp/launch_manager
./launch_manager -c etc/lifecycle_demo_with_hari.json

#### Step 3: Run test_hari with Deadline Miss

Terminal 2 - test_hari app:
cd /home/lg/HARI/SCORE/lifecycle
export LD_LIBRARY_PATH=/home/lg/HARI/SCORE/lifecycle/examples/tmp/lib:$LD_LIBRARY_PATH
export PROCESS entity=/home/lg/HARI/SCORE/lifecycle/test_hari/
./bazel-bin/test_hari/deadline_miss_app -p 200 -r 10 -n 50 -x 150 -m 5 -e 300

**What you'll see:**
- App starts and reports RUNNING to Launch Manager
- Runs normally for 5 cycles
- Cycle 6: Deliberately sleeps 300ms extra → exceeds max deadline (150ms)
- Health Monitor detects deadline violation
- HM stops sending alive notifications to LM
- Launch Manager detects missing alive
- **LM triggers recovery action** (restarts process group)

#### Parameters Explained:
- `-p 200`: Period is 200ms
- `-r 10`: Runtime 10ms per cycle
- `-n 50`: Min deadline window 50ms
- `-x 150`: Max deadline window 150ms
- `-m 5`: Miss deadline after 5 cycles
- `-e 300`: Extra sleep 300ms (exceeds max)

---

## Option 2: Build mini-adas with Bazel

For production deployment, build mini-adas the same way as test_hari.

### Step 1: Create Bazel Build for mini-adas with Lifecycle

You need to create a BUILD file that links the lifecycle libraries properly.

#### Create: feo/examples/rust/mini-adas/BUILD_lifecycle

load("@rules_rust//rust:defs.bzl", "rust_binary")

rust_binary(
    name = "adas_primary_lifecycle",
    srcs = ["src/bin/adas_primary.rs"],
    crate_features = [
        "signalling_relayed_tcp",
        "lifecycle",
    ],
    data = [
        "etc/hm_config.json",
    ],
    deps = [
        ":libmini_adas_rust",
        "//src/feo:libfeo_rust",
        "//src/feo-time:libfeo_time_rust",
        # These need to reference lifecycle workspace:
        "@lifecycle//src/health_monitoring_lib:health_monitoring_lib",
        "@lifecycle//src/launch_manager_daemon/lifecycle_client_lib/rust_bindings:lifecycle_client_rs",
    ],
)

### Step 2: Build with Bazel

cd /home/lg/HARI/SCORE/feo
bazel build //examples/rust/mini-adas:adas_primary_lifecycle

### Step 3: Deploy

mkdir -p /home/lg/HARI/SCORE/lifecycle/examples/tmp/mini_adas
cp bazel-bin/examples/rust/mini-adas/adas_primary_lifecycle /home/lg/HARI/SCORE/lifecycle/examples/tmp/mini_adas/adas_primary
cp bazel-bin/examples/rust/mini-adas/adas_secondary /home/lg/HARI/SCORE/lifecycle/examples/tmp/mini_adas/
cp examples/rust/mini-adas/etc/hm_config.json /home/lg/HARI/SCORE/lifecycle/examples/tmp/mini_adas/etc/

### Step 4: Create Launch Manager Config

Copy the mini_adas_lifecycle.json to launch manager config:
cp /home/lg/HARI/SCORE/lifecycle/examples/config/mini_adas_lifecycle.json /home/lg/HARI/SCORE/lifecycle/examples/tmp/launch_manager/etc/

### Step 5: Run with Launch Manager

Terminal 1 - Launch Manager:
cd /home/lg/HARI/SCORE/lifecycle/examples/tmp/launch_manager
./launch_manager -c etc/mini_adas_lifecycle.json

Terminal 2 - Activate Running target:
Use lmcontrol or the control API to activate "Running" run target

---

## QUICK TEST NOW: Use test_hari

Since test_hari is already built and working, let's use that IMMEDIATELY:

### Quick Command Sequence:

Terminal 1:
cd /home/lg/HARI/SCORE/lifecycle/examples/tmp/launch_manager
export LD_LIBRARY_PATH=../lib:$LD_LIBRARY_PATH
./launch_manager -c etc/lifecycle_demo_with_hari.json

Terminal 2 (wait 5 seconds after LM starts):
cd /home/lg/HARI/SCORE/lifecycle
export LD_LIBRARY_PATH=/home/lg/HARI/SCORE/lifecycle/examples/tmp/lib:$LD_LIBRARY_PATH
export PROCESSIDENTIFIER=test_hari_app
./bazel-bin/test_hari/deadline_miss_app -p 200 -r 10 -n 50 -x 150 -m 5 -e 300

---

## Expected Behavior

### Normal Cycles (1-5):
[INFO] cycle=1 ...normal execution...
[INFO] cycle=2 ...normal execution...
[INFO] cycle=3 ...normal execution...
[INFO] cycle=4 ...normal execution...
[INFO] cycle=5 ...normal execution...

### Cycle 6 - Deadline Miss:
[INFO] cycle=6 *** DELIBERATELY MISSING DEADLINE (sleeping 450ms max=150ms) ***
[HM] Deadline violation detected: TooLate
[HM] Stopping alive notifications to Launch Manager
[LM] Missing alive notifications from test_hari_app
[LM] Triggering recovery action: restart_process_group

### After Recovery:
[INFO] cycle=1 ...restarted, back to normal...

---

## For mini-adas: Simulate Deadline Miss

Once mini-adas is deployed, simulate secondary failure:

### Method 1: Kill a Secondary
pkill -9 adas_secondary

Result:
- Primary waits for secondary (timeout)
- FEO cycle exceeds deadline
- HM detects → LM restarts all 3 processes

### Method 2: Artificial Delay in Secondary
Add sleep in a secondary activity:
std::thread::sleep(Duration::from_secs(15)); // Exceeds timeout

Result:
- Primary waits > 10s timeout
- Cycle exceeds deadline
- HM detects → LM triggers recovery

---

## Summary

**NOW**: Use test_hari to see full lifecycle working
**NEXT**: Build mini-adas with Bazel for proper integration
**LATER**: Fine-tune deadline thresholds based on observed behavior
