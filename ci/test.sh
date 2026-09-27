#!/bin/sh
set -ex
cargo clippy --workspace --quiet --all-targets -- -D warnings
cargo test --workspace --quiet --no-fail-fast -- --nocapture
