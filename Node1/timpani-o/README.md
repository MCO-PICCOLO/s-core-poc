<!--
* SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
* SPDX-License-Identifier: MIT
-->

# TIMPANI-O

**Timpani-O** is the orchestrator component of the TIMPANI project, responsible for distributing task schedules to Timpani-N instances via gRPC.

| | |
|---|---|
| **Version** | 2026.03.0 ([CalVer](https://calver.org/)) |
| **Changelog** | [CHANGELOG.md](CHANGELOG.md) |
| **License** | MIT |

## Getting started

Refer to [TIMPANI-N's README.md](https://github.com/MCO-PICCOLO/TIMPANI/blob/main/timpani-n/README.md) for the full description of the project.

## Prerequisites

- On Ubuntu
  - gRPC & protobuf
    ```
    sudo apt install -y libgrpc++-dev libprotobuf-dev protobuf-compiler-grpc
    ```
  - Prerequisites for libtrpc(D-Bus)
    ```
    sudo apt install -y libsystemd-dev
    ```

- On CentOS
  - gRPC & protobuf
    ```
    sudo dnf install -y protobuf-devel protobuf-compiler
    # Enable EPEL(Extra Packages for Enterprise Linux) repository for gRPC
    sudo dnf install -y epel-release
    sudo dnf install -y grpc-devel
    ```
  - Prerequisites for libtrpc(D-Bus)
    ```
    sudo dnf install -y systemd-devel
    ```

## How to build

```
git clone --recurse-submodules https://github.com/MCO-PICCOLO/TIMPANI.git
cd timpani-o
mkdir build
cd build
cmake ..
make
```

### Cross-compilation for ARM64

```
cd build
cmake -DCMAKE_TOOLCHAIN_FILE=../cmake/toolchain-aarch64-gcc.cmake ..
make
```

### Packaging

```
cd build
cpack -G DEB
or
cpack -G RPM
or
cpack -G TGZ
```

## Coding style

- TIMPANI-O follows the [Google C++ Style Guide](https://google.github.io/styleguide/cppguide.html) with some modifications:
  - Use 4 spaces for indentation.
  - Place a line break before the opening brace after function and class definitions
- Use `clang-format` to format your code with .clang-format file provided in the project root.
  ```
  clang-format -i <file>
  ```
- `.clang-format` and `.editorconfig` are provided to help maintain consistent coding styles.

## How to run

- To run Timpani-O with default options:
  ```
  timpani-o
  ```
- To run Timpani-O with specific options, refer to the help message:
  ```
  timpani-o -h
  ```

## Testing

### Dependencies

GoogleTest framework is required for testing.

- On Ubuntu:
  ```
  sudo apt install -y libgtest-dev
  ```
- On CentOS:
  ```
  sudo dnf install -y gtest-devel
  ```

### Enable and run tests

- To enable testing, configure the build with the following CMake option:
  ```
  cmake -DBUILD_TESTS=ON ..
  ```

- To run all tests:
  ```
  make test
  ```

- To run a specific unit test:
  ```
  ./tests/test_schedinfo_service
  ./tests/test_dbus_server
  ./tests/test_fault_client
  ./tests/test_global_scheduler
  ./tests/test_node_config
  ```

## Container Deployment

TIMPANI-O can be built and deployed as a container image for Docker or Podman.

### Ports

| Port | Protocol | Service |
|------|----------|---------|
| 50052 | gRPC | SchedInfoServer |
| 7777 | TCP | D-Bus Server (libtrpc) |

### Prerequisites

- Docker 20.10+ or Podman 4.0+
- For multi-arch builds: Docker Buildx or Podman with manifest support

### Build Image

#### Using Docker

```bash
# Initialize submodule (required for container build)
git submodule update --init --recursive

# Build for current architecture
./scripts/build-image.sh v0.1.0

# Build for amd64 and arm64
./scripts/build-multiarch.sh v0.1.0
```

#### Using Podman

```bash
# Initialize submodule
git submodule update --init --recursive

# Build for current architecture
podman build -t sdv.lge.com/timpani/timpani-o:v0.1.0 .

# Build for specific architecture
podman build --platform linux/amd64 -t sdv.lge.com/timpani/timpani-o:v0.1.0-amd64 .
podman build --platform linux/arm64 -t sdv.lge.com/timpani/timpani-o:v0.1.0-arm64 .

# Create multi-arch manifest
podman manifest create sdv.lge.com/timpani/timpani-o:v0.1.0
podman manifest add sdv.lge.com/timpani/timpani-o:v0.1.0 sdv.lge.com/timpani/timpani-o:v0.1.0-amd64
podman manifest add sdv.lge.com/timpani/timpani-o:v0.1.0 sdv.lge.com/timpani/timpani-o:v0.1.0-arm64
```

### Run Container

The container image includes a default configuration file (`/timpani-o/examples/node_configurations.yaml`).

#### Method 1: Use the default configuration file included in the image

If no configuration changes are needed, you can use the example configuration file included in the image directly.

**Docker:**
```bash
docker run -d \
  --name timpani-o \
  -p 50052:50052 \
  -p 7777:7777 \
  sdv.lge.com/timpani/timpani-o:latest \
  -s 50052 -d 7777 -c /timpani-o/examples/node_configurations.yaml
```

**Podman:**
```bash
podman run -d \
  --name timpani-o \
  -p 50052:50052 \
  -p 7777:7777 \
  sdv.lge.com/timpani/timpani-o:latest \
  -s 50052 -d 7777 -c /timpani-o/examples/node_configurations.yaml
```

#### Method 2: Volume mount a custom configuration file

To use a configuration file tailored to your environment, pass it to the container via volume mount.

**Docker:**
```bash
docker run -d \
  --name timpani-o \
  -p 50052:50052 \
  -p 7777:7777 \
  -v /path/to/your/node_configurations.yaml:/config/node_configurations.yaml:ro \
  sdv.lge.com/timpani/timpani-o:latest \
  -s 50052 -d 7777 -c /config/node_configurations.yaml
```

**Podman:**
```bash
podman run -d \
  --name timpani-o \
  -p 50052:50052 \
  -p 7777:7777 \
  -v /path/to/your/node_configurations.yaml:/config/node_configurations.yaml:ro \
  sdv.lge.com/timpani/timpani-o:latest \
  -s 50052 -d 7777 -c /config/node_configurations.yaml
```

#### Method 3: Run without a configuration file

You can also run in default mode without a configuration file.

```bash
# Docker
docker run -d --name timpani-o -p 50052:50052 -p 7777:7777 \
  sdv.lge.com/timpani/timpani-o:latest

# Podman
podman run -d --name timpani-o -p 50052:50052 -p 7777:7777 \
  sdv.lge.com/timpani/timpani-o:latest
```

#### Using docker-compose / podman-compose

```bash
# Docker
docker-compose up -d

# Podman
podman-compose up -d
```

> **Note:** `docker-compose.yml` uses Method 2 (volume mount).
> Modify the `command` section as needed.

### Push to Registry

#### Using Docker

```bash
# Login to internal registry
docker login sdv.lge.com

# Push image
./scripts/push-image.sh v0.1.0

# Or build and push multi-arch in one step
./scripts/build-multiarch.sh v0.1.0 --push
```

#### Using Podman

```bash
# Login to internal registry
podman login sdv.lge.com

# Push single image
podman push sdv.lge.com/timpani/timpani-o:v0.1.0

# Push multi-arch manifest
podman manifest push sdv.lge.com/timpani/timpani-o:v0.1.0
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `SINFO_PORT` | 50052 | gRPC SchedInfoServer port |
| `DBUS_PORT` | 7777 | D-Bus Server port |
| `FAULT_HOST` | localhost | Piccolo FaultService host |
| `FAULT_PORT` | 50053 | Piccolo FaultService port |

### Connecting to Piccolo

To connect timpani-o to Piccolo's FaultService running on the host:

#### Docker
```bash
docker run -d \
  --name timpani-o \
  -p 50052:50052 \
  -p 7777:7777 \
  --add-host=host.docker.internal:host-gateway \
  sdv.lge.com/timpani/timpani-o:latest \
  -f host.docker.internal -p 50053
```

#### Podman
```bash
podman run -d \
  --name timpani-o \
  -p 50052:50052 \
  -p 7777:7777 \
  --network=host \
  sdv.lge.com/timpani/timpani-o:latest \
  -f localhost -p 50053
```

