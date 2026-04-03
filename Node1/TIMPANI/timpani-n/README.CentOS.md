<!--
* SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
* SPDX-License-Identifier: MIT
-->

## Prerequisites for CentOS Stream 10

- development tools such as gcc, g++, make and cmake
```
dnf group install -y "Development Tools"
dnf install -y cmake
```

- clang and bpftool required for BPF
```
dnf install -y clang bpftool
dnf install -y systemd-devel
```

- libyaml required for *dummy_server* program
```
dnf config-manager --set-enabled crb
dnf install -y libyaml-devel
```
