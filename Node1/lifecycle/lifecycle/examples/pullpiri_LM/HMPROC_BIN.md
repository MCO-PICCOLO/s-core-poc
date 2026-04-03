# Understanding `hmproc_adas_primary.bin`

## What It Is

`hmproc_adas_primary.bin` is a **FlatBuffer-compiled binary** that describes one
supervised process to the Health Monitor (HM) daemon running inside the Launch Manager.
It tells the HM daemon:

- **Which process identifier** to associate with incoming HM alive pings
  (`"adas_primary"`)
- **Which checkpoints / supervision channels** that process uses

The Launch Manager reads this file at startup alongside `hm_demo.bin` and
`hmcore.bin` to wire up the full supervision chain.

---

## Where It Comes From

```
pullpiri_lm_config.json
        │
        │  (Bazel rule: launch_manager_config)
        ▼
scripts/config_mapping/config.bzl
        │
        │  Step 1 – Python script (lifecycle_config)
        │  Reads pullpiri_lm_config.json + JSON schema
        │  Emits intermediate JSON files to bazel-bin/.../json_out/
        │
        │  Output files are named by prefix:
        │    lm_*.json   → LM schema
        │    hm_*.json   → HM schema        ← hmproc_adas_primary.json lands here
        │    hmproc_*.json → HM schema
        │    hmcore*.json → HM-core schema
        │
        │  Step 2 – flatc (FlatBuffers compiler)
        │  Compiles each JSON against its schema → .bin
        │
        ▼
bazel-bin/examples/pullpiri_LM/flatbuffer_out/
    lm_demo.bin
    hm_demo.bin
    hmcore.bin
    hmproc_adas_primary.bin          ← this file
```

**Bazel target to build it:**
```bash
bazel build --config=x86_64-linux //examples/pullpiri_LM:pullpiri_config
```

---

## What Drives Its Content

The content of `hmproc_adas_primary.bin` is derived from the `adas_primary`
entry inside `pullpiri_lm_config.json`:

```json
"adas_primary": {
    "component_properties": {
        "application_profile": {
            "application_type": "Reporting_And_Supervised",
            "alive_supervision": {
                "reporting_cycle": 0.2,
                "min_indications": 1,
                "max_indications": 5,
                "failed_cycles_tolerance": 3
            }
        }
    },
    "deployment_config": {
        "environmental_variables": {
            "PROCESSIDENTIFIER": "adas_primary",
            "CONFIG_PATH": "/opt/pullpiri/etc/hmproc_adas_primary.bin"
        }
    }
}
```

The Python config-mapping script extracts this and generates a JSON in HM-process
format (`hmproc_adas_primary.json`), which `flatc` then compiles to binary.

---

## Why Two Copies Are Deployed

| Destination | Purpose |
|---|---|
| `lifecycle/lifecycle/etc/hmproc_adas_primary.bin` | Read by the Launch Manager / HM daemon at startup to register the supervised process |
| `/opt/pullpiri/etc/hmproc_adas_primary.bin` | Read by `adas_primary` itself at runtime via `CONFIG_PATH` env var, to initialise its internal HM client checkpoints |

Both must be present and **must be generated from the same Bazel build** — using a
stale or hand-copied binary from another system can cause mismatches in process
identifiers (e.g. `"mini_adas_primary"` instead of `"adas_primary"`), which makes the
HM daemon fail to match alive pings to the registered process.

---

## Regenerating the Binary

If you change anything in `pullpiri_lm_config.json` (supervision parameters,
process identifier, etc.), regenerate the binary by re-running:

```bash
cd lifecycle/lifecycle
bazel build --config=x86_64-linux //examples/pullpiri_LM:pullpiri_config
# Then re-run the full run.sh to deploy the updated .bin files
sudo ./examples/pullpiri_LM/run.sh
```

Do **not** hand-edit or copy `.bin` files across different config versions — always
regenerate from source.

---

## HM Schema Location

The FlatBuffers schema used to compile `hmproc_*.json` → `hmproc_*.bin` is the HM
schema, referenced in `config.bzl`:

```python
"hm_schema": attr.label(
    default = Label("//src/launch_manager_daemon:..."),
    allow_single_file = [".fbs"],
)
```

The schema defines the wire format the LM/HM daemon parses at runtime.
