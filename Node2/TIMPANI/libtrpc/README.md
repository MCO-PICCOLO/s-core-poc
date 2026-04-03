#<!--
#* SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
#* SPDX-License-Identifier: MIT
#-->

# libtrpc (Timpani RPC Library)

## How to build

### Prerequisites
- Make sure to have pkg-config and libsystemd-dev installed.
```
sudo apt install -y pkg-config
sudo apt install -y libsystemd-dev
```
- Preparation for cross-compiling using dpkg multi-architecture
```
sudo dpkg --add-architecture arm64
sudo dpkg --add-architecture armhf
sudo apt update
sudo apt install -y crossbuild-essential-arm64
sudo apt install -y crossbuild-essential-armhf
sudo apt install -y libsystemd-dev:arm64
sudo apt install -y libsystemd-dev:armhf
```
> **_NOTE:_**  If 'apt update' fails, refer to https://askubuntu.com/questions/430705/how-to-use-apt-get-to-download-multi-arch-library.

### Compiling for local host
```
mkdir -p build-x86
cd build-x86
cmake ..
make
```

### Cross-compiling for ARM64
```
mkdir -p build-arm64
cd build-arm64
cmake -DCMAKE_TOOLCHAIN_FILE=../aarch64-toolchain.cmake ..
make
```

### Cross-compiling for ARM32
```
mkdir -p build-arm32
cd build-arm32
cmake -DCMAKE_TOOLCHAIN_FILE=../armhf-toolchain.cmake ..
make
```

### Manually specifying libsystemd path and version
If pkg-config does not find libsystemd, you can manually specify libsystemd path and version:
```
cmake -DLIBSYSTEMD_INCLUDE_DIRS=/path/to/include -DLIBSYSTEMD_LIBRARIES=/path/to/lib/libsystemd.so -DLIBSYSTEMD_VERSION=version ..
```

## Running test programs

- The sample server program waits for connection from clients and provides several methods.

```
./trpc_server
```

- The sample client program connects to the server and calls "Register" and "SchedInfo" methods.
```
./trpc_client
```
