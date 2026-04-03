# Pullpiri and S-CORE Lifecycle Integration

This document describes the overall integration between the Pullpiri vehicle service orchestrator and the S-CORE Lifecycle system (Launch Manager & Health Monitor).

## Overview

Pullpiri is designed to orchestrate vehicle services in a cloud-native manner, deploying and managing workload scenarios. To ensure Pullpiri itself is highly available and resilient, its core components have been integrated with the S-CORE Lifecycle system.

Instead of running Pullpiri components directly via scripts or basic systemd units, the **Lifecycle Launch Manager** is used to:
1. Orchestrate the startup sequence of the 10 core Pullpiri daemons (including Timpani and NodeAgent).
2. Manage dependencies between these services.
3. Automatically recover (restart) any component that crashes.
4. Manage the components within a unified Process Group (`MainPG`).

## Directory Structure

All Pullpiri-specific lifecycle files are located in a self-contained example folder:

```
lifecycle/lifecycle/examples/pullpiri_LM/
├── BUILD                          # Bazel build rules for the Pullpiri config
├── run.sh                         # One-command script to build, configure, and start
├── config/
│   ├── BUILD                      # Bazel exports for the JSON config
│   ├── pullpiri_lm_config.json    # Launch Manager component configuration
│   ├── lm_logging.json            # Launch Manager daemon logging (appId: PULLPIRI_LM)
│   ├── hm_logging.json            # Health Monitor daemon logging  (appId: PULLPIRI_HM)
│   └── ecu_logging_config.json    # ECU-level logging (console + file output)
```

### Logging Configuration Files

| File | Purpose |
|---|---|
| `lm_logging.json` | Controls the **Launch Manager** daemon's own log verbosity. Set `logLevel` to `kDebug`, `kInfo`, `kWarn`, or `kError`. |
| `hm_logging.json` | Controls the **Health Monitor** daemon's log verbosity, using the same log levels. |
| `ecu_logging_config.json` | ECU-wide logging settings. Defines whether logs go to `kConsole`, `kFile`, or both (`kConsole|kFile`), and specifies the file output directory. |

The `run.sh` script automatically copies `lm_logging.json` → `etc/logging.json` and `ecu_logging_config.json` → `etc/ecu_logging_config.json` before starting the Launch Manager, so the daemon picks them up at runtime.

## Flow of Lifecycle Launching Pullpiri Components

The Launch Manager uses a state-machine based Process Group (`MainPG`) to launch all components. The configured states are `Off`, `Startup`, and `Recovery`.

When the Launch Manager transitions `MainPG` into the `Startup` state, it launches the Pullpiri background daemons in a carefully ordered sequence based on their inter-dependencies:

### Startup Sequence

1. **`persistency-service`**:
   - Primary key-value store relying on gRPC.
   - **Dependencies**: None. This must start first as all other components need to load or save state here.
2. **`apiserver`** & **`monitoringserver`**:
   - API server handles external REST requests and artifact uploads.
   - Monitoring server tracks running workload statuses.
   - **Dependencies**: Both depend on `persistency-service` being fully initialized. Their start is triggered simultaneously once the persistency service is up.
3. **`statemanager`**:
   - Manages the state machine logic for workloads.
   - **Dependencies**: Depends on `monitoringserver`.
4. **`filtergateway`**:
   - Listens to vehicle DDS topics and filters events based on deployed scenarios.
   - **Dependencies**: Depends on `apiserver`.
5. **`actioncontroller`**:
   - The execution engine that triggers Bluechi or NodeAgent based on scenario matches.
   - **Dependencies**: Depends on `filtergateway` and `statemanager`. Must be the last to start to ensure the engine only acts when the decision components are fully online.
6. **`policymanager`**:
   - Manages scheduling and resource policies.
   - **Dependencies**: None.
7. **`timpani-o`**:
   - Timpani orchestrator providing scheduling info via gRPC and D-Bus.
   - **Dependencies**: None. Launched with arguments: `-s 50052 localhost -p 50053 -d 7777 --node-config <path_to_node_configurations.yaml>`.
8. **`nodeagent`**:
   - Orchestrates local workload execution on the node.
   - **Dependencies**: Depends on `apiserver` for registration.
9. **`timpani-n`**:
   - Node-level Timpani agent.
   - **Dependencies**: Depends on `timpani-o`. Launched with arguments: `-n node01 -c 1 -P 85 -p 7777 -l 4 -a localhost`.

### Daemonization Flow

Under the hood, Launch Manager follows a strict POSIX `fork` and `exec` path:
- Each component is executed as a standalone background process (daemon).
- Standard variables like `LD_LIBRARY_PATH` and logging paths are injected directly into the environment of the `fork`ed processes.
- The Launch Manager collects the exit status of these daemons. If a daemon terminates unexpectedly (abnormal exit code), the Launch Manager catches it and applies the configured recovery actions (e.g., restarting the daemon up to N times, or switching to the `Recovery` mode).

## Launch Manager Configuration

The configuration defining this flow is provided in the JSON file: `examples/pullpiri_LM/config/pullpiri_lm_config.json`.

This file includes:
- A **defaults** section specifying the binary directory (`/opt/pullpiri/bin`), timeouts, recovery actions, and the `Native` application profile.
- A **components** section with an entry for each of the 10 managed processes, detailing:
  - `binary_name`: The executable name inside `/opt/pullpiri/bin`.
  - `depends_on`: Which other processes must be ready before this one can start.
  - `process_arguments`: Command-line arguments (used by `timpani-o`).
  - `environmental_variables`: Runtime environment variables (like `PROCESSIDENTIFIER`).
- A **run_targets** section defining the `Startup` and `Recovery` states.

## Prerequisites

Before starting the components, make sure the environment is properly configured:

1. **Update the Bind IP:**
   The Pullpiri components read their bind IP from `/etc/piccolo/settings.yaml`. Since you are running them locally, ensure the IP is set to `127.0.0.1` (otherwise they will fail to bind):
   ```bash
   sudo sed -i 's/192.168.2.177/127.0.0.1/g' /etc/piccolo/settings.yaml
   ```

2. **Create the Database Directory:**
   The `persistency-service` writes its Key-Value Store files to `/var/lib/pullpiri`. Create it and grant permissions:
   ```bash
   sudo mkdir -p /var/lib/pullpiri
   sudo chmod 777 /var/lib/pullpiri
   ```

## How to Run

### Quick Start (Recommended)

Use the provided `run.sh` script. It handles building, copying configs (including logging), and launching the daemon in one command:

```bash
cd /home/acrn/new_ak/vso_score/lifecycle/lifecycle/examples/pullpiri_LM
chmod +x run.sh
./run.sh
```

The script will:
1. Build the S-CORE Lifecycle project with the Pullpiri configuration.
2. Copy the compiled Flatbuffer configs (`lm_demo.bin`, `hm_demo.bin`, `hmcore.bin`) to `etc/`.
3. Copy the LM and ECU logging configs to `etc/` so the daemon picks them up.
4. Start the Launch Manager daemon in the foreground.

### Manual Steps

If you prefer to run each step manually:

1. **Ensure Pullpiri Binaries are Built**
   Compile all Pullpiri binaries and place them in the target `/opt/pullpiri/bin` directory. For example, after building the project locally with `cargo build`, you can copy the binaries over:
   ```bash
   cd /home/acrn/new_ak/vso_score/pullpiri/src/target/debug
   sudo cp filtergateway monitoringserver statemanager actioncontroller apiserver persistency-service policymanager /opt/pullpiri/bin
   cd  /home/acrn/new_ak/vso_score/pullpiri/src/agent/nodeagent/target/debug
   sudo cp nodeagent /opt/pullpiri/bin
   sudo cp /home/acrn/new_ak/vso_score/timpani-o/build /opt/pullpiri/bin
   sudo cp /home/acrn/new_ak/score/score_orc/vso/new_timpani/time-trigger/build/timpani-n /opt/pullpiri/bin/timpani-n
   sudo chown root:root /opt/pullpiri/bin/*
   ```

2. **Build the S-CORE Lifecycle Project**
   ```bash
   cd /home/acrn/new_ak/vso_score/lifecycle/lifecycle
   bazel build --config=x86_64-linux //...
   ```

3. **Copy Configs and Start the Launch Manager**
   ```bash
   cd /home/acrn/new_ak/vso_score/lifecycle/lifecycle
   # Flatbuffer binary configs
   sudo cp bazel-bin/examples/pullpiri_LM/flatbuffer_out/lm_demo.bin etc/lm_demo.bin
   sudo cp bazel-bin/examples/pullpiri_LM/flatbuffer_out/hm_demo.bin etc/hm_demo.bin
   sudo cp bazel-bin/examples/pullpiri_LM/flatbuffer_out/hmcore.bin etc/hmcore.bin
   # Logging configs
   cp examples/pullpiri_LM/config/lm_logging.json etc/logging.json
   cp examples/pullpiri_LM/config/ecu_logging_config.json etc/ecu_logging_config.json
   # Start
   bazel-bin/src/launch_manager_daemon/launch_manager
   ```
   *(This process will stay running in the foreground and print logs for all daemons. Leave it open.)*

4. **Verify the Deployment**
   Check the Launch Manager's terminal output. You should see all 10 Pullpiri components initializing:
   ```
   LCM started successfully
   Completed the request for PG MainPG to State MainPG/Startup
   Starting Pullpiri Persistency Service
   Initializing FilterGateway
   initialize action controller
   MonitoringServerManager init
   FilterGatewayManager init
   SchedInfoServer listening on port 50052
   DBusServer listening on port 7777
   ...
   ```

### Adjusting Log Levels

To reduce log verbosity, edit the logging JSONs in `examples/pullpiri_LM/config/` and change `logLevel` from `kDebug` to `kInfo`, `kWarn`, or `kError`. Then re-run `run.sh` or manually copy the updated file to `etc/logging.json`.

## Application Supervision Types

The S-CORE Launch Manager supports multiple levels of integration between itself and the applications it manages. This is controlled by the `application_type` field in the configuration.

### Current Mode: `Native`

Pullpiri currently runs with `"application_type": "Native"`. In this mode:

- The Launch Manager treats each Pullpiri component as a plain Linux process.
- **No "I am alive" messages** are sent from Pullpiri to the Launch Manager.
- The Launch Manager monitors health purely by watching the **process exit status** via the OS. If a process crashes (unexpected exit code), the Launch Manager detects it and triggers recovery (restart up to 3 times, then switch to `Recovery` run target).
- This requires **zero code changes** inside Pullpiri — the components don't need to know about S-CORE at all.

### Available Modes (Not Currently Used)

| Mode | Description |
|---|---|
| `Native` | **(Current)** No integration. Launch Manager only monitors process liveness via the OS. |
| `Reporting` | The application uses S-CORE Lifecycle APIs to report its execution state transitions (e.g., initializing → running → terminating). |
| `Reporting_And_Supervised` | The application uses Lifecycle APIs **and** actively sends periodic "I am alive" checkpoint messages. The Launch Manager expects these at a configured interval and will trigger recovery if they stop arriving. |
| `State_Manager` | Full integration: Lifecycle APIs + alive checkpoints + permission to change the active Run Target. |

### ⚠️ Warning: Do NOT Switch to `Reporting_And_Supervised` Without Code Changes

If you changed `pullpiri_lm_config.json` to use `"application_type": "Reporting_And_Supervised"` today **without modifying the Pullpiri source code**, the following would happen:

1. The Launch Manager starts the Pullpiri component normally.
2. It immediately begins waiting for the first "I am alive" checkpoint message.
3. **Pullpiri has no S-CORE library linked**, so it never sends any checkpoint.
4. After the configured `alive_supervision.reporting_cycle` timeout expires, the Launch Manager assumes the process has frozen.
5. The Launch Manager **kills the process** and attempts to restart it.
6. This cycle repeats until all restart attempts are exhausted, and the system falls back to the `Recovery` run target — effectively shutting down all Pullpiri components.

### Future: Enabling Active Health Monitoring

To enable `Reporting_And_Supervised` for Pullpiri in the future, you would need to:

1. **Link the S-CORE Lifecycle client library** into the Pullpiri Rust/C++ codebase:
   - For C++: link against `liblifecycle_client.so` and `libprocess_state_client.so` (found in `bazel-bin/src/launch_manager_daemon/`).
   - For Rust: create FFI bindings to the C++ library, or use the Rust supervised app example (`examples/rust_supervised_app/`) as a reference.

2. **Add checkpoint reporting** in each Pullpiri component's main loop:
   - Periodically send an "I am alive" message at the interval specified by `alive_supervision.reporting_cycle` in the config.

3. **Update the configuration**:
   - Change `application_type` to `Reporting_And_Supervised`.
   - Add `alive_supervision` parameters (`reporting_cycle`, `failed_cycles_tolerance`, `min_indications`, `max_indications`) to each component's `component_properties`.

Until these code changes are made, **keep `application_type` set to `Native`**. The OS-level process monitoring already provides crash detection and automatic restart, which is sufficient for the current deployment.

## Recent Code Fixes (quick summary)

Small fixes were applied to improve schedule updates and fault handling during scenario updates:

- **`actioncontroller/manager.rs`**: Added a `reschedule` action and changed the update flow to send the new `Schedule` to `timpani-o` before restarting containers. This prevents race conditions where a restarted container would register with `timpani-n` before the updated schedule is available.

Rebuild notes:

- Rebuild `actioncontroller` after changes: `cargo build -p actioncontroller`.
- Rebuild `timpani-o` (CMake) and restart the service so the fixes take effect.

These changes enable a `reschedule` workflow: send a `Schedule` + `Scenario(action: reschedule)` (and a `Package` block if using the apiserver `apply()` flow) to update timing without restarting containers.
