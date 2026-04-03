<!--
* SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
* SPDX-License-Identifier: MIT
-->

## Prerequisites for Ubuntu 20

### Install the latest clang

> NOTE: a recent version of clang is required. The default version in Ubuntu 20 is v10

```
mkdir -p ~/.local/bin
export PATH=~/.local/bin:$PATH

sudo bash -c "$(wget -O - https://apt.llvm.org/llvm.sh)"
ln -s /usr/lib/llvm-19/bin/clang ~/.local/bin
```

### Download and compile the latest bpftool

> NOTE: a latest version of bpftool is required. The default version in Ubuntu 20 is v5.4

```
git clone --recurse-submodules -b v7.4.0 https://github.com/libbpf/bpftool.git
cd bpftool/src
make
cp bpftool ~/.local/bin
```
