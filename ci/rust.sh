#!/bin/sh
set -ex
rustup toolchain add "$RUST_VERSION" \
    --component clippy \
    --component rustfmt
rustup default "$RUST_VERSION"
