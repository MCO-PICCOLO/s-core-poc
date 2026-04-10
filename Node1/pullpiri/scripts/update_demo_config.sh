#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<EOF
Usage: $0 [options]

Options:
  --ip IP                      IP to write into settings.yaml (host.ip)
  --hostname-node2 NAME        Node2 hostname to set as node in reschedule YAML
  --settings-path PATH         Path to settings.yaml (default: /etc/piccolo/settings.yaml)
  --reschedule-path PATH       Path to reschedule_sea.yaml (default: examples/resources/reschedule_sea.yaml)
  --timpani-rs PATH            Path to timpani.rs (default: src/player/statemanager/src/grpc/receiver/timpani.rs)
  --node-config-path PATH      Path to node_configurations.yaml to inject into pullpiri_lm_config.json
  --pullpiri-config PATH       Path to pullpiri_lm_config.json (default: lifecycle/examples/pullpiri_LM/config/pullpiri_lm_config.json)
  -h, --help                   Show this help

Example:
  $0 --ip 192.168.10.100 --hostname acrn-NUC11TNHi5 \
    --reschedule-path /home/user/pullpiri/examples/resources/reschedule_sea.yaml \
    --node-config-path /home/user/pullpiri/examples/resources/timpani/node_configurations.yaml
EOF
}

# Defaults (workspace-relative)
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SETTINGS_PATH="/etc/piccolo/settings.yaml"
RESCHEDULE_PATH="$ROOT_DIR/examples/resources/reschedule_sea.yaml"
TIMPANI_RS="$ROOT_DIR/src/player/statemanager/src/grpc/receiver/timpani.rs"
NODE_CONFIG_PATH="$ROOT_DIR/examples/resources/timpani/node_configurations.yaml"
PULLPIRI_CONFIG="$ROOT_DIR/../lifecycle/lifecycle/examples/pullpiri_LM/config/pullpiri_lm_config.json"

# Parse args
while [[ $# -gt 0 ]]; do
  case "$1" in
    --ip) IP="$2"; shift 2;;
    --hostname-node2) HOSTNAME="$2"; shift 2;;
    --settings-path) SETTINGS_PATH="$2"; shift 2;;
    --reschedule-path) RESCHEDULE_PATH="$2"; shift 2;;
    --timpani-rs) TIMPANI_RS="$2"; shift 2;;
    --node-config-path) NODE_CONFIG_PATH="$2"; shift 2;;
    --pullpiri-config) PULLPIRI_CONFIG="$2"; shift 2;;
    -h|--help) usage; exit 0;;
    *) echo "Unknown arg: $1"; usage; exit 1;;
  esac
done

if [[ -z "${IP:-}" && -z "${HOSTNAME:-}" && -z "${NODE_CONFIG_PATH:-}" ]]; then
  echo "Nothing to do: pass --ip and/or --hostname and/or --node-config-path" >&2
  usage
  exit 1
fi

# 1) Create/update settings.yaml
if [[ -n "${IP:-}" ]]; then
  echo "Updating settings YAML at $SETTINGS_PATH with ip=$IP"
  TMPSET="$(mktemp)"
  cat > "$TMPSET" <<EOF
host:
  name: HPC
  ip: $IP
  type: vehicle
  role: master
dds:
  idl_path: src/vehicle/dds/idl
  domain_id: 100
EOF
  # Ensure directory exists
  DIR="$(dirname "$SETTINGS_PATH")"
  if [[ ! -d "$DIR" ]]; then
    echo "Creating directory $DIR (sudo)"
    sudo mkdir -p "$DIR"
  fi
  echo "Writing $SETTINGS_PATH (may prompt for sudo)"
  sudo cp "$TMPSET" "$SETTINGS_PATH"
  rm -f "$TMPSET"
fi

# 2) Update 'node:' field in the Package models section of reschedule YAML
#    (does NOT touch 'node_id:' in the Schedule spec)
if [[ -n "${HOSTNAME:-}" ]]; then
  if [[ -f "$RESCHEDULE_PATH" ]]; then
    echo "Setting node in $RESCHEDULE_PATH -> $HOSTNAME"
    esc_host=$(printf '%s' "$HOSTNAME" | sed 's/[&/]/\\&/g')
    # Match lines that have 'node:' but NOT 'node_id:' or 'node_name:'
    sed -i -E "s/^([[:space:]]+)node:[[:space:]].*/\1node: $esc_host/" "$RESCHEDULE_PATH"
  else
    echo "Reschedule file not found: $RESCHEDULE_PATH" >&2
  fi
fi

# 3) Update RESCHEDULE_YAML_PATH constant in timpani.rs
#    The constant may be declared across two lines:
#      const RESCHEDULE_YAML_PATH: &str =
#          "...path...";
#    Use Python to handle this reliably.
if [[ -n "${RESCHEDULE_PATH:-}" && -f "$TIMPANI_RS" ]]; then
  echo "Patching RESCHEDULE_YAML_PATH in $TIMPANI_RS -> $RESCHEDULE_PATH"
  python3 - "$TIMPANI_RS" "$RESCHEDULE_PATH" <<'PY'
import sys, re
rs_file, new_path = sys.argv[1], sys.argv[2]
with open(rs_file, 'r') as f:
    src = f.read()
# Match single-line or two-line form of the const declaration
pattern = r'(const RESCHEDULE_YAML_PATH:\s*&str\s*=\s*\n?\s*)"[^"]*"(\s*;)'
replacement = r'\g<1>"' + new_path + r'"\g<2>'
new_src, count = re.subn(pattern, replacement, src)
if count == 0:
    print(f"WARNING: RESCHEDULE_YAML_PATH not found in {rs_file}", file=sys.stderr)
    sys.exit(1)
with open(rs_file, 'w') as f:
    f.write(new_src)
print(f"Patched {count} occurrence(s)")
PY
fi

# 4) Update pullpiri_lm_config.json's node-config path
if [[ -n "${NODE_CONFIG_PATH:-}" && -f "$PULLPIRI_CONFIG" ]]; then
  echo "Patching pullpiri LM config $PULLPIRI_CONFIG -> node_config: $NODE_CONFIG_PATH"
  # Replace any quoted string that ends with node_configurations.yaml
  esc_node_config=$(printf '%s' "$NODE_CONFIG_PATH" | sed 's/[&/]/\\&/g')
  sed -i -E "s|\"[^\"]*node_configurations.yaml\"|\"$esc_node_config\"|g" "$PULLPIRI_CONFIG"
fi

echo "Done."

exit 0
