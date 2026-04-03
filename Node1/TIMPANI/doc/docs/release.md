#<!--
#* SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
#* SPDX-License-Identifier: MIT
#-->

# TIMPANI

## Release Management


#### Textual RELEASE PLAN  Overview

```
TIMAPNI Release Roadmap (2026)
───────────────────────────────────────────────────────────────────────────────
Q1 2026                Q2 2026                Q3 2026                Q4 2026
─────────────┬────────────────────┬────────────────────────────┬───────────────
Analysis &   │ gRPC Service       │ gPTP Time Sync + gRPC      │ Integration &
Design for   │ Migration:         │ Integration (Multi-node),  │ Optimization,
Modern Arch  │ - timpani-o Rust   │ OSS Release Preparation    │ OSS Docs
OSS Vision   │ - timpani-n Rust   │                            │
─────────────┴────────────────────┴────────────────────────────┴───────────────

Milestone 1:                Milestone 2:                Milestone 3:                Milestone 4:
─────────────               ─────────────               ─────────────               ─────────────
- Architecture analysis     - Migrate timpani-o &       - Integrate gPTP time      - Final integration,
	& design finalized          timpani-n to Rust           sync with gRPC             optimization, and
- Define OSS scope            with gRPC services        - Enable multi-node          documentation
- Initial repo setup        - Single-node Rust            operation                 - Prepare OSS
															implementation            - Prepare for OSS            documentation:
														- Core scheduling &           open source release        README, API, docs,
															execution logic           - Finalize features &        contributing guide
														- POSIX timer support         stability testing
														- SCHED_DEADLINE,
															SCHED_FIFO, SCHED_RR

───────────────────────────────────────────────────────────────────────────────
Key Features to Port (across milestones):
- Timpani-O: Global scheduling (Rust, gRPC)
- Timpani-N: Local execution, microsecond precision (POSIX timers)
- Linux RT policies: SCHED_DEADLINE, SCHED_FIFO, SCHED_RR
- Hyperperiod synchronization
- Deadline miss detection
- BPF-based kernel monitoring
───────────────────────────────────────────────────────────────────────────────
```


---

## Overview

This release plan covers the migration and feature development for all major TIMPANI components:
- **Timpani-O** (Orchestrator)
- **Timpani-N** (Time Trigger Node)
- **Sample Apps** (Real-time Workload Demos)
- **libbpf** (eBPF Integration)

The plan is organized by quarterly milestones, with each milestone capturing the features, migration status, and deliverables for each component.

**Reference Implementations:**
- C++ Source: `timpani-o/src/`, `timpani-n/src/`, `sample-apps/src/`, `libtrpc/src/`
- Protocols: D-Bus (legacy), gRPC/protobuf (target)

---


### Release Process

This project uses GitHub Actions (CI/CD pipeline workflow) for the release process. Release artifacts are generated using the following commands:

```bash
# In the project directory
git tag vX.X.XX   # (e.g., git tag v1.0.00)
git push origin vX.X.XX
```

Once the tag is pushed, GitHub Actions triggers the building of release artifacts as defined in the release workflow sections. Release notes for each release are generated and managed with the help of [release notes](https://docs.github.com/en/repositories/releasing-projects-on-github/automatically-generated-release-notes).


