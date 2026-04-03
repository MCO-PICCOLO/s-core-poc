#!/bin/bash
# Setup Launch Manager daemon deployment for mini-adas v2

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LM_DIR="/tmp/mini_adas_lm"
FLATC="/home/lg/HARI/SCORE/lifecycle/bazel-bin/external/flatbuffers+/flatc"
LM_SCHEMA="/home/lg/HARI/SCORE/lifecycle/src/launch_manager_daemon/config/lm_flatcfg.fbs"
HM_SCHEMA="/home/lg/HARI/SCORE/lifecycle/src/launch_manager_daemon/health_monitor_lib/config/hm_flatcfg.fbs"
HMCORE_BIN="/home/lg/HARI/SCORE/lifecycle/bazel-bin/examples/flatbuffer_out/hmcore.bin"
LM_LOGGING_JSON="/home/lg/HARI/SCORE/lifecycle/examples/config/lm_logging.json"

SECONDARY_DELAY_EVERY_CYCLES="${SECONDARY_DELAY_EVERY_CYCLES:-20}"
SECONDARY_DELAY_MS="${SECONDARY_DELAY_MS:-0}"

echo "=============================================="
echo "Mini-ADAS Launch Manager Setup"
echo "=============================================="
echo ""

# Clean previous setup
if [ -d "$LM_DIR" ]; then
    echo "🧹 Cleaning previous setup..."
    rm -rf "$LM_DIR"
fi

echo "📁 Creating directory structure..."
mkdir -p "$LM_DIR"/{etc,mini_adas,lib}

echo "📦 Copying binaries..."
# LM daemon
cp /home/lg/HARI/SCORE/lifecycle/bazel-bin/src/launch_manager_daemon/launch_manager "$LM_DIR/"
echo "  ✅ Launch Manager daemon"

# Application binaries
cp /home/lg/HARI/SCORE/feo/target/release/adas_primary "$LM_DIR/mini_adas/"
echo "  ✅ adas_primary binary"
cp /home/lg/HARI/SCORE/feo/target/release/adas_secondary "$LM_DIR/mini_adas/"
echo "  ✅ adas_secondary binary"

# Libraries
cp "$SCRIPT_DIR"/lib/*.so "$LM_DIR/lib/" 2>/dev/null || echo "  ⚠️  No .so files in lib/"
echo "  ✅ Shared libraries"

echo ""
echo "📋 Copying configurations..."
# Your HM config
cp "$SCRIPT_DIR/etc/hmproc_adas_primary.bin" "$LM_DIR/etc/"
echo "  ✅ hmproc_adas_primary.bin"
cp "$SCRIPT_DIR/etc/hm_config.json" "$LM_DIR/etc/"
mkdir -p "$LM_DIR/mini_adas/etc"
cp "$SCRIPT_DIR/etc/hm_config.json" "$LM_DIR/mini_adas/etc/"
echo "  ✅ hm_config.json"

cp "$HMCORE_BIN" "$LM_DIR/etc/hmcore.bin"
cp "$LM_LOGGING_JSON" "$LM_DIR/etc/logging.json"
echo "  ✅ hmcore.bin"
echo "  ✅ logging.json"

echo ""
echo "⚙️  Creating LM configuration..."

cat > "$LM_DIR/etc/lm_demo.json" << EOF
{
    "versionMajor": 7,
    "versionMinor": 0,
    "ModeGroup": [
        {
            "identifier": "MainPG",
            "initialMode_name": "not-used",
            "recoveryMode_name": "MainPG/Running",
            "modeDeclaration": [
                { "identifier": "MainPG/Startup" },
                { "identifier": "MainPG/Running" },
                { "identifier": "MainPG/fallback_run_target" }
            ]
        }
    ],
    "Process": [
        {
            "identifier": "mini_adas_secondary_1",
            "path": "/tmp/mini_adas_lm/mini_adas/adas_secondary",
            "uid": 1000,
            "gid": 1000,
            "numberOfRestartAttempts": 3,
            "executable_reportingBehavior": "DoesNotReportExecutionState",
            "startupConfig": [
                {
                    "identifier": "mini_adas_secondary_1_startup",
                    "enterTimeoutValue": 10000,
                    "exitTimeoutValue": 10000,
                    "schedulingPolicy": "SCHED_OTHER",
                    "schedulingPriority": "0",
                    "terminationBehavior": "ProcessIsNotSelfTerminating",
                    "executionError": "1",
                    "processGroupStateDependency": [
                        { "stateMachine_name": "MainPG", "stateName": "MainPG/Startup" }
                    ],
                    "environmentVariable": [
                        { "key": "LD_LIBRARY_PATH", "value": "/tmp/mini_adas_lm/lib" },
                        { "key": "IDENTIFIER", "value": "mini_adas_secondary_1" },
                        { "key": "PROCESSIDENTIFIER", "value": "adas_secondary_1" },
                        { "key": "MINI_ADAS_DELAY_EVERY_CYCLES", "value": "$SECONDARY_DELAY_EVERY_CYCLES" },
                        { "key": "MINI_ADAS_DELAY_MS", "value": "$SECONDARY_DELAY_MS" },
                        { "key": "MINI_ADAS_DELAY_ACTIVITY_IDS", "value": "2" }
                    ],
                    "processArgument": [
                        { "argument": "1" }
                    ],
                    "executionDependency": [
                        { "stateName": "Running", "targetProcess_identifier": "mini_adas_primary" }
                    ]
                }
            ]
        },
        {
            "identifier": "mini_adas_secondary_2",
            "path": "/tmp/mini_adas_lm/mini_adas/adas_secondary",
            "uid": 1000,
            "gid": 1000,
            "numberOfRestartAttempts": 3,
            "executable_reportingBehavior": "DoesNotReportExecutionState",
            "startupConfig": [
                {
                    "identifier": "mini_adas_secondary_2_startup",
                    "enterTimeoutValue": 10000,
                    "exitTimeoutValue": 10000,
                    "schedulingPolicy": "SCHED_OTHER",
                    "schedulingPriority": "0",
                    "terminationBehavior": "ProcessIsNotSelfTerminating",
                    "executionError": "1",
                    "processGroupStateDependency": [
                        { "stateMachine_name": "MainPG", "stateName": "MainPG/Startup" }
                    ],
                    "environmentVariable": [
                        { "key": "LD_LIBRARY_PATH", "value": "/tmp/mini_adas_lm/lib" },
                        { "key": "IDENTIFIER", "value": "mini_adas_secondary_2" },
                        { "key": "PROCESSIDENTIFIER", "value": "adas_secondary_2" }
                    ],
                    "processArgument": [
                        { "argument": "2" }
                    ],
                    "executionDependency": [
                        { "stateName": "Running", "targetProcess_identifier": "mini_adas_primary" }
                    ]
                }
            ]
        },
        {
            "identifier": "mini_adas_primary",
            "path": "/tmp/mini_adas_lm/mini_adas/adas_primary",
            "uid": 1000,
            "gid": 1000,
            "numberOfRestartAttempts": 3,
            "executable_reportingBehavior": "ReportsExecutionState",
            "startupConfig": [
                {
                    "identifier": "mini_adas_primary_startup",
                    "enterTimeoutValue": 10000,
                    "exitTimeoutValue": 10000,
                    "schedulingPolicy": "SCHED_OTHER",
                    "schedulingPriority": "0",
                    "terminationBehavior": "ProcessIsNotSelfTerminating",
                    "executionError": "1",
                    "processGroupStateDependency": [
                        { "stateMachine_name": "MainPG", "stateName": "MainPG/Startup" },
                    ],
                    "environmentVariable": [
                        { "key": "LD_LIBRARY_PATH", "value": "/tmp/mini_adas_lm/lib" },
                        { "key": "IDENTIFIER", "value": "mini_adas_primary" },
                        { "key": "PROCESSIDENTIFIER", "value": "mini_adas_primary" },
                        { "key": "CONFIG_PATH", "value": "/tmp/mini_adas_lm/etc/hmproc_adas_primary.bin" },
                        { "key": "RUST_LOG", "value": "info" }
                    ],
                    "processArgument": [
                        { "argument": "400" }
                    ],
                    "executionDependency": []
                }
            ]
        }
    ]
}
EOF

cat > "$LM_DIR/etc/hm_demo.json" << 'EOF'
{
    "versionMajor": 8,
    "versionMinor": 0,
    "process": [],
    "hmMonitorInterface": [],
    "hmSupervisionCheckpoint": [],
    "hmAliveSupervision": [],
    "hmLocalSupervision": [],
    "hmGlobalSupervision": [],
    "hmRecoveryNotification": []
}
EOF

echo "  ✅ lm_demo.json created"
echo "  ✅ hm_demo.json created"

if [ ! -f "$FLATC" ]; then
    echo ""
    echo "⚠️  WARNING: flatc compiler not found!"
    echo "   Build it with: cd /home/lg/HARI/SCORE/lifecycle && bazel build @flatbuffers//:flatc"
    echo ""
    echo "❌ Cannot compile LM config to flatbuffer without flatc"
    echo ""
    echo "Setup incomplete. Follow LAUNCHING_WITH_LM.md for manual steps."
    exit 1
fi

if [ ! -f "$LM_SCHEMA" ] || [ ! -f "$HM_SCHEMA" ]; then
    echo ""
    echo "⚠️  WARNING: flatbuffer schema not found"
    echo ""
    echo "❌ Cannot compile config"
    echo ""
    echo "Setup incomplete. Follow LAUNCHING_WITH_LM.md for manual steps."
    exit 1
fi

echo ""
echo "🔨 Compiling LM config to flatbuffer..."
cd "$LM_DIR/etc"

if $FLATC -b -o . "$LM_SCHEMA" lm_demo.json 2>&1 | grep -q "error:"; then
    echo "❌ Failed to compile LM config"
    echo "   Check the JSON syntax and schema compatibility"
    exit 1
fi

if $FLATC -b -o . "$HM_SCHEMA" hm_demo.json 2>&1 | grep -q "error:"; then
    echo "❌ Failed to compile HM config"
    exit 1
fi

if [ -f "lm_demo.bin" ]; then
    echo "  ✅ lm_demo.bin created ($(stat -c%s lm_demo.bin) bytes)"
else
    echo "❌ Flatbuffer binary not created"
    exit 1
fi

echo ""
echo "=============================================="
echo "✅ Setup Complete!"
echo "=============================================="
echo ""
echo "Directory structure:"
echo "  $LM_DIR/"
echo "    ├── launch_manager          (3.0M)"
echo "    ├── etc/"
echo "    │   ├── lm_demo.bin"
echo "    │   ├── hm_demo.bin"
echo "    │   ├── hmcore.bin"
echo "    │   ├── hmproc_adas_primary.bin"
echo "    │   └── hm_config.json"
echo "    ├── mini_adas/"
echo "    │   ├── adas_primary"
echo "    │   ├── adas_secondary"
echo "    │   └── etc/hm_config.json"
echo "    └── lib/"
echo "        └── *.so (C++ libraries)"
echo ""
echo "To run Launch Manager daemon:"
echo ""
echo "  cd $LM_DIR"
echo "  export LD_LIBRARY_PATH=\$PWD/lib"
echo "  ./launch_manager"
echo ""
echo "Or use the convenience script:"
echo ""
echo "  $SCRIPT_DIR/run_with_lm_daemon.sh"
echo ""
echo "See LAUNCHING_WITH_LM.md for detailed documentation."
echo ""
