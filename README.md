
## Getting Started

Clone the repository using one of the following methods:

**SSH:**
```sh
git clone git@github.com:MCO-PICCOLO/s-core-poc.git
```

**HTTPS:**
```sh
git clone https://github.com/MCO-PICCOLO/s-core-poc.git
```

After cloning, your directory structure will look like:

```
s-core-poc/
├── Node1/
│   ├── feo/
│   ├── lifecycle/
│   ├── sea_app/
│   └── timpani-o/
├── Node2/
│   ├── examples/
│   ├── lifecycle/
│   ├── pullpiri/
│   ├── sea_app/
│   └── time-trigger/
├── README.md
└── ...
```
# s-core-poc

Integration PoC between S-CORE modules and Pullpiri/Timpani

## Overview
s-core-poc demonstrates integration between S-CORE modules and Pullpiri/Timpani systems across two nodes



---

## Node1: Lifecycle Launch Manager

To build and run the S-CORE Lifecycle Launch Manager with Pullpiri components, follow the instructions in the Node1 run script:

- [Node1 run.sh instructions](Node1/lifecycle/examples/pullpiri_LM/README.md)
- [Direct link to run.sh script](Node1/lifecycle/examples/pullpiri_LM/run.sh)

This script will:
- Build the required S-CORE and Pullpiri binaries using Bazel and Cargo
- Copy configuration and logging files
- Sync necessary binaries to `/opt/pullpiri/bin`
- Start the Launch Manager daemon

See the linked README for detailed prerequisites and step-by-step instructions.

---


## Node2: Building and Running nodeagent with Lifecycle Launch manager

Node2 is responsible for building the Pullpiri `nodeagent` binary (note: the binary is around 100MB and cannot be uploaded to GitHub, so it must be built locally on Node2) and running the integration using its own run script.

### Steps to Build nodeagent

1. Go to the nodeagent source directory:
   ```sh
   cd Node2/pullpiri/src/agent/nodeagent
   ```
2. Build the nodeagent binary using Cargo:
   ```sh
   cargo build --release
   ```
   The resulting binary will be located at:
   ```
   target/release/nodeagent
   ```
3. Copy the built binary to the Node2 lifecycle repository:
   ```sh
   cp target/release/nodeagent ../../../../lifecycle/
   ```
   Adjust the destination path as needed for your setup.

After building, run the Node2 integration using the provided run script:

- [Node2 run.sh instructions](Node2/lifecycle/examples/pullpiri_LM/README.md)
- [Direct link to Node2 run.sh script](Node2/lifecycle/examples/pullpiri_LM/run.sh)

For more details and system setup, see the [Node2 README](Node2/README.md).

---

## Additional Notes

- Ensure all prerequisites (Bazel, Rust, Java, sudo access) are met as described in the respective READMEs.
- For troubleshooting and advanced configuration, refer to the documentation in each module's directory.
