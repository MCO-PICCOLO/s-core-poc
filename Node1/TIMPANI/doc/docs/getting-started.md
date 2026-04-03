
<!--
* SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
* SPDX-License-Identifier: MIT
-->

# Getting Started with TIMPANI

Welcome to the TIMPANI project! This guide will help you get up and running with the main components, sample applications, and documentation structure.

---

## 1. Prerequisites

Install the following dependencies based on your OS and the components you wish to use:

### Common (Ubuntu)

```bash
sudo apt install -y libelf-dev zlib1g-dev clang linux-tools-$(uname -r)
sudo apt install -y pkg-config libsystemd-dev libyaml-dev
```

### For gRPC & Protobuf (TIMPANI-O)

```bash
sudo apt install -y libgrpc++-dev libprotobuf-dev protobuf-compiler-grpc
```

### For CentOS

See the detailed instructions in:
- timpani-n/README.CentOS.md
- timpani-o/README.md

---

## 2. Cloning the Repository

```bash
git clone --recurse-submodules https://github.com/MCO-PICCOLO/TIMPANI.git
cd TIMPANI
```

---

## 3. Building the Components


### Timpani-N

```bash
cd timpani-n
mkdir build && cd build
cmake ..
make
```

### Timpani-O

```bash
cd timpani-o
mkdir build && cd build
cmake ..
make
```

#### Cross-compilation for ARM64 (Timpani-O)
```bash
cd build
cmake -DCMAKE_TOOLCHAIN_FILE=../cmake/toolchain-aarch64-gcc.cmake ..
make
```

### Sample Applications

```bash
cd sample-apps
mkdir build && cd build
cmake ..
cmake --build .
```

---

## 4. Running the System

### Example: Running Timpani-N

1. Start the main system:
	```bash
	sudo ./timetrigger
	```
2. (Optional) Run test server and sample tasks in separate terminals:
	```bash
	./dummy_server
	sudo ./exprocs wakee1 10000 &
	sudo ./exprocs wakee2 50000 &
	sudo ./exprocs wakee3 20000 &
	```

### Example: Running Sample Apps

```bash
sudo ./sample_apps [OPTIONS] <task_name>
```
See `sample-apps/README.md` for available options and workload types.

---

## 5. Documentation

- See `doc/architecture/EN/timpani-n/README.md` for a full documentation index and architecture overview.
- Each subproject (timpani-n, timpani-o, sample-apps) has its own README with more details.

---

## 6. Contributing

Feedback and contributions are welcome! Please see the documentation and guidelines in each subproject for more information.
