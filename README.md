
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

## Node2: Building and Providing nodeagent

Node2 is responsible for building the Pullpiri `nodeagent` binary and making it available to Node1's lifecycle system.

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
3. Copy the built binary to the Node2 lifecycle repository (or to the location expected by Node1's run.sh):
	```sh
	cp target/release/nodeagent ../../../../lifecycle/
	```
	Adjust the destination path as needed for your setup.

For more details and system setup, see the [Node2 README](Node2/README.md).

---

## Additional Notes

- Ensure all prerequisites (Bazel, Rust, Java, sudo access) are met as described in the respective READMEs.
- For troubleshooting and advanced configuration, refer to the documentation in each module's directory.
