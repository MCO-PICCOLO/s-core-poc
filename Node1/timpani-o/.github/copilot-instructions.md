# Project Overview

This `Timpani-O` project is a C++ application that interacts with a time-triggered scheduling system for real-time tasks.
It includes a gRPC server that allows `Piccolo`, a workload orchestrator, to add new scheduling tables, and a gRPC client to notify `Piccolo` of deadline miss faults.
Additionally, it provides a D-Bus peer-to-peer server that offers the following time-triggered scheduling features for `Timpani-N` (also known as the Timpani node manager):

  - Send scheduling tables to `Timpani-N`
  - Receive deadline miss faults from `Timpani-N`
  - Multi-node synchronization for starting time-triggered tasks

## Folder Structure

- `src/`: Contains the main source code files for the `Timpani-O` program.
- `proto/`: Contains Protocol Buffers definitions for gRPC communication with the workload orchestrator.
- `cmake/`: Contains CMake modules for building the project.
- `tests/`: Contains unit tests for testing the project.

## Libraries and Dependencies

- CMake: For building the project.
- gRPC: For communication between `Timpani-O` and `Piccolo`.
- Protocol Buffers: For serializing structured data.

## Coding Style
- Refer to the Coding style section in the [README.md](../README.md) file.
