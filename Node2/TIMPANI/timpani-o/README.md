<!--
* SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
* SPDX-License-Identifier: MIT
-->

# TIMPANI-O

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
