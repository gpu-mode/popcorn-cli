#!/bin/bash
set -e

echo "Building Popcorn CLI (Rust version)..."
cargo build --release

echo "Build complete! Binary is available at: target/release/popcorn-cli"
echo "Run with: ./target/release/popcorn-cli <filepath>"