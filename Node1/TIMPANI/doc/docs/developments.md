
<!--
* SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
* SPDX-License-Identifier: MIT
-->

# TIMPANI Development Guide

This document describes the development workflow, testing, static analysis, and best practices for contributing to the TIMPANI project.

---

## Table of Contents

1. [Development Workflow](#development-workflow)
2. [Branching Strategy](#branching-strategy)
3. [Pull Requests & Code Review](#pull-requests--code-review)
4. [Continuous Integration](#continuous-integration)
5. [Static Analysis](#static-analysis)
6. [Testing](#testing)
7. [Documentation](#documentation)

---

## Development Workflow

1. Fork the repository and clone your fork.
2. Create a new branch for your feature or bugfix.
3. Make your changes, following the [coding rules](../contribution/coding-rule.md).
4. Run static analysis and tests locally.
5. Push your branch and open a Pull Request (PR) against the `development_0.5` branch (not main).
6. Request a review and address any feedback.
7. Once the milestone is reached, changes from `development_0.5` will be merged to `main`.

---

## Branching Strategy

- **main**: Stable, production-ready code.
- **development_0.5**: All ongoing development until June 2026. All developers should use this branch for features and bugfixes.
- **feature/xxx**: New features (branch from development_0.5).
- **bugfix/xxx**: Bug fixes (branch from development_0.5).
- **hotfix/xxx**: Urgent fixes (branch from main).

---

## Pull Requests & Code Review

- All changes must go through a Pull Request (PR).
- PRs must be made against the `development_0.5` branch, not `main`.
- PRs should be linked to an issue or feature request when possible.
- Ensure your code passes all checks (formatting, lint, tests).
- At least one reviewer must approve before merging.
- Use clear commit messages and PR descriptions.

---

## Continuous Integration

The project uses CI (e.g., GitHub Actions) to automatically run static analysis, tests, and build checks on every PR and push to main branches.

Typical CI checks include:
- rustfmt (formatting)
- clippy (lint)
- cargo test (unit/integration tests)
- cargo audit (security)
- cargo deny (license/dependency checks)
- cargo tarpaulin (coverage, optional)

---

## Static analysis

### [rustfmt](https://github.com/rust-lang/rustfmt)

Rustfmt formats Rust code according to the official rust style guide.
It enforces consistency across codebases and making code easier to read and review.

#### Using rustfmt

Step 1: Install Rustfmt.

```bash
rustup component add rustfmt
```

Step 2: Format Your Code: Recursively formats all `.rs` files in the project.

```bash
# in src directory
make fmt
```

### [clippy](https://doc.rust-lang.org/nightly/clippy/)

Clippy is a collection of lints to catch common mistakes and improve Rust code.
It analyzes source code and provides suggestions for idiomatic Rust practices
to performance improvements and potential bugs.

#### Using clippy

Step 1: Install Clippy.

```bash
rustup component add clippy
```

Step 2: Run Clippy: Runs the linter on your project.
It reports warnings and suggestions for your code.

```bash
# in src directory
make clippy
```

*Optional*: Automatically fix warnings: Applies safe automatic fixes to clippy warnings.

```bash
# directory located `Cargo.toml` like `src/server/apiserver`
cargo clippy --fix
```

### [cargo audit](https://crates.io/crates/cargo-audit) - Security vulnerability scanner

Cargo Audit scans your `Cargo.lock` file for crates with known security
vulnerabilities using the RustSec Advisory Database.
It helps you ensure your dependencies are secure.

#### Using cargo-audit

Step 1: Install Cargo Audit.

```bash
cargo install cargo-audit
```

Step 2: Run: Scans the dependency tree for known vulnerabilities and outdated crates.

```bash
# directory located `Cargo.lock` like `src/server`, `src/player`, `src/agent`
cargo audit
```

### [cargo deny](https://crates.io/crates/cargo-deny) - Dependency & License checker

Cargo Deny checks for issues including:It’s used to enforce policies for project dependencies.

- Duplicate crates
- Insecure or unwanted licenses
- Vulnerabilities
- Unmaintained crates

#### Using cargo-deny

Step 1: Install Cargo Deny

```bash
cargo install cargo-deny
```

Step 2: Initialize Configuration (for New components only) -
Creates a default deny.toml file where you can configure license policies, bans and exceptions.

```bash
# directory located `Cargo.toml` like `src/server/apiserver`
# almost all crates already done.
cargo deny init
```

Step 3: Run Check -
Analyzes dependency metadata and reports issues related to licenses, advisories and duplicates.

```bash
# directory located `Cargo.toml` like `src/server/apiserver`
cargo deny check
```

### [cargo udeps](https://crates.io/crates/cargo-udeps) - Unused Dependency Detector

Cargo Udeps identifies unused dependencies in your `Cargo.toml` Keeping unused dependencies can bloat the project and expose unnecessary vulnerabilities."For this requires Nightly Rust to run".

*Caution* : rustc 1.86.0 or above is required. (on July 18th, 2025)

#### Using cargo-udeps

Step 1: Install Cargo Udeps

```bash
cargo install cargo-udeps
rustup install nightly
```

Step 2: Nightly run -
This runs the unused dependency checker and lists packages not being used.

```bash
cargo +nightly udeps
```


---

## Testing

### Unit Tests

Unit tests can be executed using the following command:

```bash
# In any directory containing `Cargo.toml`
cargo test
```

### Integration Tests

Integration tests should be placed in the `tests/` directory of each crate. For more information, refer to the [Rust documentation](https://doc.rust-lang.org/rust-by-example/testing/integration_testing.html).


### Code Coverage

Use [cargo-tarpaulin](https://crates.io/crates/cargo-tarpaulin) to measure test coverage (see above for usage).

**Required:** Code coverage must be at least 80% for a PR to be accepted.

---

## Documentation

- All public functions, structs, and modules should have Rust doc comments (`///`).
- Each crate should have a `README.md` describing its purpose, usage, and examples.
- Update the main project documentation as needed when adding new features or modules.
- Use `cargo doc --open` to build and preview documentation locally.

---

## [cargo tarpaulin](https://crates.io/crates/cargo-tarpaulin) - Code coverage

cargo-tarpaulin is a code coverage tool specifically designed for Rust projects.
It works by instrumenting the Rust code, running the tests,and
generating a detailed report that shows which lines of code were covered during testing.
It supports a variety of Rust test strategies, including unit tests and integration tests.

### Install and run

To use cargo-tarpaulin you need to install it on your system.
This can be done easily via cargo:

```bash
cargo install cargo-tarpaulin

#Once installed, you can run the tool in your project by using the following command:
# in any directories include `Cargo.toml`
cargo tarpaulin
```

