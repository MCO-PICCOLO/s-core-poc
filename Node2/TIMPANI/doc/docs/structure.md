
<!--
* SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
* SPDX-License-Identifier: MIT
-->

# Project Structure

This document describes the current structure of the TIMPANI repository. All files and folders listed here are considered stable and will remain untouched in the future, except for the `timpani_rust` folder, which will be the sole focus of ongoing development.

---

## Current Repository Layout

```bash
TIMPANI/
├── LICENSE
├── README.md
├── .gitmodules
├── doc/
│   ├── architecture/
│   │   ├── architecture-diagrams/
│   │   ├── EN/
│   │   └── KR/
│   ├── contribution/
│   ├── docs/
│   └── images/
├── examples/
│   └── readme.md
├── libtrpc/
│   ├── src/
│   ├── test/
│   ├── CMakeLists.txt
│   └── README.md
├── sample-apps/
│   ├── src/
│   ├── README.md
│   └── README_kr.md
├── scripts/
│   └── version.txt
├── timpani-n/
│   ├── src/
│   ├── test/
│   ├── scripts/
│   ├── README.md
│   ├── README.CentOS.md
│   └── README.Ubuntu20.md
├── timpani-o/
│   ├── src/
│   ├── proto/
│   ├── cmake/
│   ├── tests/
│   └── README.md
└── timpani_rust/
	├── timpani-n/
	│   └── readme.md
	└── timpani-o/
		└── readme.md
```

---

## Future Development: `timpani_rust/`

All future work will be focused on the `timpani_rust` directory. The rest of the repository will remain as a reference and for legacy support.

### Planned Rust Structure (Example)

```bash
timpani_rust/
├── timpani-n/
│   ├── src/                # Rust source code for Timpani-N
│   ├── Cargo.toml          # Rust package manifest
│   └── readme.md           # Documentation for Timpani-N
└── timpani-o/
	├── src/                # Rust source code for Timpani-O
	├── proto/              # gRPC proto files
	├── Cargo.toml          # Rust package manifest
	└── readme.md           # Documentation for Timpani-O
```

#### Module Overview

- **timpani-n**: Rust implementation of the time-triggered node agent.
- **timpani-o**: Rust implementation of the orchestrator, including gRPC interfaces and scheduling logic.

> Additional submodules (e.g., common, utils, integration tests) may be added as the Rust codebase evolves.

---

## Notes

- All legacy C/C++ code, documentation, and sample applications will remain for reference and backward compatibility.
- Only the `timpani_rust` folder will be actively developed and maintained going forward.
