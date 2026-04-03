# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Calendar Versioning](https://calver.org/) (YYYY.MM.PATCH).

## [Unreleased]

## [2026.03.0] - 2026-03-11

### Added
- **Apex.OS Integration**: Full Apex.OS monitoring and task management support
  - Apex.OS test mode (`-a` flag) for running without TT schedule info
  - CPU affinity control for Apex.OS tasks
  - Coredata UDS client for periodic reporting
  - APEX_RESET message handling for demo scenarios
- **Hostname as Default Node ID**: Use system hostname when `-n` is not specified
- **pidfd Support**: Modern process management using pidfd and epoll
- **Hyperperiod Management**: Timer-based hyperperiod tracking with statistics
- **Perfetto UI Support**: Generate Perfetto-compatible JSON traces (replaces gnuplot)
- **Log Levels**: Configurable logging with 6 levels (silent to verbose)
- **ARM64 Cross-compilation**: Enhanced CMake configuration for ARM64 targets
- **DEB/RPM Packaging**: CPack configuration with proper dependencies and post-install scripts
- **System Requirements Checker**: `scripts/check-requirements.sh` for pre-flight validation
- **Documentation Restructure**: 
  - Unified `INSTALL.md` for all platforms
  - `docs/REQUIREMENTS.md` for kernel and system requirements
  - `docs/BUILDING.md` for build options and packaging
  - `docs/USAGE.md` for runtime configuration

### Changed
- **Modular Architecture**: Comprehensive refactoring with context-based design
  - Removed global variables, pass context to all functions
  - Consolidated state into context structure
  - Improved error handling with `tt_error_t` return types
- **Static libbpf**: Switch from dynamic to static linking for better portability
- **Logging System**: Replace printf/fprintf with structured logging macros
- **Function Naming**: Renamed functions for clarity and consistency
- **Memory Management**: New allocation macros with improved cleanup

### Removed
- Legacy dummy server and associated files
- Obsolete gnuplot-based tracing (replaced by Perfetto)
- Platform-specific README files (consolidated into INSTALL.md)

### Fixed
- Connection cleanup in TRPC
- Event accumulation from input files

## [1.0.0] - 2025-08-12

Initial stable release.

### Added
- Time-triggered task scheduler with eBPF monitoring
- BPF-based deadline miss detection
- Multi-node synchronization support (`-s` flag)
- RT priority configuration (`-P` flag)
- CPU affinity binding (`-c` flag)
- gnuplot data generation (`-g` flag)

---

[Unreleased]: http://mod.lge.com/hub/timpani/time-trigger/-/compare/v2026.03.0...main
[2026.03.0]: http://mod.lge.com/hub/timpani/time-trigger/-/compare/v1.0...v2026.03.0
[1.0.0]: http://mod.lge.com/hub/timpani/time-trigger/-/tags/v1.0
