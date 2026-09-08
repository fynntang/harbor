#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
# Check formatting without modifying source files, then test and build locked dependencies.
cargo fmt --all -- --check
cargo test --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --release -p harbor-cli --locked
