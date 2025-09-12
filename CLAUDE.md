# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Foundry is a blazing fast, portable and modular toolkit for Ethereum application development written in Rust. This is a fork for Taiko support (branch: multi-block-env).

The project consists of four main tools:
- **Forge**: Ethereum testing framework (like Truffle, Hardhat and DappTools)
- **Cast**: Swiss army knife for interacting with EVM smart contracts
- **Anvil**: Local Ethereum node, akin to Ganache, Hardhat Network
- **Chisel**: Fast, utilitarian, and verbose Solidity REPL

## Build and Development Commands

### Building
```bash
# Build all crates
cargo build --all

# Build with release optimizations
cargo build --release --all

# Build a specific tool
cargo build -p forge
cargo build -p cast
cargo build -p anvil
cargo build -p chisel
```

### Testing
```bash
# Run all tests
cargo test --all --all-features

# Run tests for a specific crate
cargo test -p forge
cargo test -p anvil

# Run a specific test
cargo test -p forge test_name

# Run tests that use forking (must contain "fork" in name)
cargo test fork

# Test cheatcodes
cargo cheats
```

### Code Quality
```bash
# Format code (must use nightly)
cargo +nightly fmt

# Check formatting without applying changes
cargo +nightly fmt -- --check

# Run clippy lints (must use nightly)
cargo +nightly clippy --all --all-targets --all-features -- -D warnings

# Run all checks (as required for PRs)
cargo check --all
```

## High-Level Architecture

### Core Crate Structure

The repository is organized as a Rust workspace with the following key crates:

**Main Tools:**
- `crates/forge/` - Testing framework implementation
- `crates/cast/` - CLI tool for contract interaction
- `crates/anvil/` - Local Ethereum node with submodules:
  - `anvil/core/` - Core node implementation
  - `anvil/rpc/` - RPC server implementation
  - `anvil/server/` - HTTP server
- `crates/chisel/` - Solidity REPL

**EVM Implementation:**
- `crates/evm/` - Core EVM tooling built on `revm`:
  - `evm/core/` - Core EVM executor and backend
  - `evm/evm/` - High-level EVM abstractions
  - `evm/fuzz/` - Fuzzing implementation
  - `evm/traces/` - Transaction tracing
  - `evm/coverage/` - Code coverage analysis

**Cheatcodes System:**
- `crates/cheatcodes/` - Testing cheatcodes implementation (Solidity calls that manipulate test environment)
- `crates/cheatcodes/spec/` - Cheatcode specifications

**Supporting Infrastructure:**
- `crates/config/` - Configuration management (foundry.toml handling)
- `crates/common/` - Shared utilities and types
- `crates/cli/` - CLI implementation for forge and cast
- `crates/fmt/` - Solidity code formatter
- `crates/debugger/` - Debugging tools
- `crates/verify/` - Contract verification
- `crates/script/` - Scripting support
- `crates/wallets/` - Wallet management

### Key Design Patterns

1. **Workspace Architecture**: The project uses Rust workspaces to manage multiple related crates with shared dependencies.

2. **EVM Integration**: Built on top of `revm` (Rust EVM implementation) with custom extensions for testing, tracing, and debugging.

3. **Cheatcodes**: A unique testing feature where special Solidity function calls can manipulate the test environment (time, block number, account balances, etc.).

4. **Incremental Compilation**: Smart caching and parallel compilation for fast builds.

5. **Configuration**: Uses `foundry.toml` for project configuration with profile support (default, CI, etc.).

## Taiko-Specific Modifications

This fork includes modifications for Taiko support:
- Updated to latest revm with multiple block environment support
- Better chain ID handling for Taiko networks
- Branch: `multi-block-env` (based on `v1-gwyneth`)

## Development Tips

1. **Running a Single Test**: Use `cargo test -p <crate> <test_name>` for faster iteration
2. **Debugging**: Enable debug builds by modifying the workspace Cargo.toml's dev profile if using a debugger
3. **Fork Tests**: Tests involving forking must have "fork" in their name
4. **Formatter**: Always use nightly Rust for formatting (`cargo +nightly fmt`)
5. **Pre-PR Checklist**: Run all quality checks before submitting PRs:
   ```bash
   cargo check --all
   cargo test --all --all-features
   cargo +nightly fmt -- --check
   cargo +nightly clippy --all --all-targets --all-features -- -D warnings
   ```