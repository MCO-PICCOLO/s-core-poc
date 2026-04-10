#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<EOF
Usage: $0 [options]

Options:
  --master-ip IP               IP of Node1 (master) to inject into configs
  --hostname NAME              Hostname to write into manifests (defaults to 
                               output of 
                               "/usr/bin/hostname")
  --node-ip IP                 Node's own IP to write into /etc/piccolo/nodeagent.yaml
  --pullpiri-config PATH       Path to pullpiri_lm_config.json (default: lifecycle/examples/pullpiri_LM/config/pullpiri_lm_config.json)
  --safe-resource PATH         Path to safe-exit-assist.yaml (default: examples/resources/safe-exit-assist.yaml)
  --timpani-sh PATH            Path to timpani.sh (default: examples/timpani.sh)
  -h, --help                   Show this help

Example:
  $0 --master-ip 10.221.40.153 --hostname lg-NUC11TNHi5
EOF
}

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
NODEAGENT_CONF="/etc/piccolo/nodeagent.yaml"
PULLPIRI_CONFIG="$ROOT_DIR/../lifecycle/examples/pullpiri_LM/config/pullpiri_lm_config.json"
SAFE_RESOURCE="$ROOT_DIR/../examples/resources/safe-exit-assist.yaml"
TIMPANI_SH="$ROOT_DIR/../examples/timpani.sh"

# Parse args
while [[ $# -gt 0 ]]; do
  case "$1" in
    --master-ip|--ip) MASTER_IP="$2"; shift 2;;
    --hostname) HOSTNAME="$2"; shift 2;;
    --node-ip) NODE_IP="$2"; shift 2;;
    --pullpiri-config) PULLPIRI_CONFIG="$2"; shift 2;;
    --safe-resource) SAFE_RESOURCE="$2"; shift 2;;
    --timpani-sh) TIMPANI_SH="$2"; shift 2;;
    -h|--help) usage; exit 0;;
    *) echo "Unknown arg: $1"; usage; exit 1;;
  esac
done

if [[ -z "${MASTER_IP:-}" && -z "${HOSTNAME:-}" && -z "${NODE_IP:-}" ]]; then
  echo "Nothing to do: pass --master-ip and/or --hostname and/or --node-ip" >&2
  usage
  exit 1
fi

# Defaults
HOSTNAME="${HOSTNAME:-$(hostname)}"
NODE_IP="${NODE_IP:-$(hostname -I 2>/dev/null | awk '{print $1}' || echo 127.0.0.1)}"

echo "Using ROOT_DIR=$ROOT_DIR"

# 1) Write /etc/piccolo/nodeagent.yaml
if [[ -n "${MASTER_IP:-}" || -n "${HOSTNAME:-}" || -n "${NODE_IP:-}" ]]; then
  echo "Creating/updating $NODEAGENT_CONF (may prompt for sudo)"
  TMPNODE="$(mktemp)"
  cat > "$TMPNODE" <<EOF
nodeagent:
  node_name: "$HOSTNAME"
  node_type: "vehicle"
  node_role: "nodeagent"
  master_ip: "${MASTER_IP:-}"
  node_ip: "$NODE_IP"
  grpc_port: 47004
  log_level: "info"
  metrics:
    collection_interval: 5
    batch_size: 50
  system:
    hostname: "$HOSTNAME"
    platform: "Linux"
    architecture: "x86_64"
EOF

  DIR="$(dirname "$NODEAGENT_CONF")"
  if [[ ! -d "$DIR" ]]; then
    echo "Creating directory $DIR (sudo)"
    sudo mkdir -p "$DIR"
  fi
  echo "Writing $NODEAGENT_CONF (sudo)"
  sudo cp "$TMPNODE" "$NODEAGENT_CONF"
  rm -f "$TMPNODE"
fi

# Helper to escape sed replacement strings
escape_for_sed() {
  printf '%s' "$1" | sed 's/[&/]/\\&/g'
}

# 2) Update node name in safe-exit-assist.yaml (first occurrence of node: or node_name:)
if [[ -n "${HOSTNAME:-}" && -f "$SAFE_RESOURCE" ]]; then
  echo "Patching node name in $SAFE_RESOURCE -> $HOSTNAME"
  esc_host=$(escape_for_sed "$HOSTNAME")
  # replace first occurrence of 'node:' or 'node_name:'
  sed -i "0,/node:/s/node:.*/node: $esc_host/" "$SAFE_RESOURCE" || true
  sed -i "0,/node_name:/s/node_name:.*/node_name: $esc_host/" "$SAFE_RESOURCE" || true
fi

# 3) Update timpani.sh to point to master IP (replace any http://...:47099)
if [[ -n "${MASTER_IP:-}" && -f "$TIMPANI_SH" ]]; then
  echo "Patching $TIMPANI_SH to use master IP $MASTER_IP"
  esc_master=$(escape_for_sed "$MASTER_IP")
  sed -i -E "s|http://[^:]+:47099|http://$esc_master:47099|g" "$TIMPANI_SH"
fi

# 4) Update pullpiri_lm_config.json timpani-n IP (use python for robust JSON edit)
if [[ -n "${MASTER_IP:-}" && -f "$PULLPIRI_CONFIG" ]]; then
  echo "Patching $PULLPIRI_CONFIG (timpani-n ipc arg) -> $MASTER_IP"
  python3 - <<PY
import json,sys
path = "$PULLPIRI_CONFIG"
with open(path,'r') as f:
    data = json.load(f)
try:
    args = data['components']['timpani-n']['component_properties']['process_arguments']
    # Replace last argument with provided master ip
    if isinstance(args, list) and len(args) >= 1:
        args[-1] = "$MASTER_IP"
        data['components']['timpani-n']['component_properties']['process_arguments'] = args
        with open(path,'w') as out:
            json.dump(data,out,indent=4)
except Exception as e:
    print('Failed to patch JSON:', e, file=sys.stderr)
    sys.exit(1)
PY
fi

echo "Done."

exit 0
