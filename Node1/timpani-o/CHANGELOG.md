# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Calendar Versioning](https://calver.org/) (YYYY.MM.PATCH).

## [Unreleased]

## [2026.03.0] - 2026-03-17

First versioned release with container deployment support.

### Added
- **Container Deployment**: Docker and Podman support
  - Multi-stage Dockerfile (Ubuntu 22.04 build → Alpine 3.21 runtime)
  - docker-compose.yml for orchestration
  - Build scripts: `build-image.sh`, `build-multiarch.sh`, `push-image.sh`
  - Multi-architecture support (amd64/arm64)
- **Version Management**: CalVer versioning system
  - VERSION file for centralized version control
  - CMake integration to read version from file
  - CHANGELOG.md for tracking changes

### Changed
- **CMakeLists.txt**: Support both external/libtrpc and ../libtrpc paths
- **README.md**: Added comprehensive Container Deployment guide

---

## Pre-release History

The following changes were made before formal version management was introduced.
These are documented for historical reference.

### Container/Packaging Preparation
- **build**: Update CMake configuration for packaging
  - Exclude libtrpc submodule from default package
  - Add packaging configuration (DEB, RPM, TGZ)
  - Update aarch64 cross-compilation cmake files

### Apex.OS Integration
- **feat**: Support for Apex.OS SchedInfo
  - Added special handling for Apex.OS workload
  - Updated cpu_affinity field from int to uint64_t
- **refactor**: Improve scheduling info buffer management
  - FreeSchedInfoBuf method for buffer cleanup
  - GetSchedInfoMap to indicate if scheduling info changed

### Core Features
- **feat**: Implement HyperperiodManager for workload hyperperiod calculations
- **feat**: Enhance workload management and task reporting
  - Added workload_id to Task structure
  - Workload-specific task details logging
  - Workload replacement and validation in SchedInfoService
- **feat**: Introduce NONE log level for Logger
- **fix**: gRPC detection with pkg-config fallback on Ubuntu 22.04
- **fix**: Improve node utilization logging
- **fix**: Update CPU utilization threshold handling
- **fix**: Correct short option for fault address in GetOptions
- **fix**: Correct serialization order for scheduling info

### Infrastructure
- **docs**: Update README and add toolchain files
- **tests**: Add unit tests using Copilot
- **tests**: Fix compile error due to recent changes

---

## Version Format

This project uses [Calendar Versioning](https://calver.org/):

```
YYYY.MM.PATCH
```

- **YYYY**: Full year (e.g., 2026)
- **MM**: Month (1-12, without leading zero)
- **PATCH**: Patch number within the month (starting from 0)

Examples:
- `2026.03.0` - First release in March 2026
- `2026.03.1` - First patch in March 2026
- `2026.04.0` - First release in April 2026
