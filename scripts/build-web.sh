#!/bin/bash
set -e

# cd to repository root
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
cd "$DIR/.."

echo "==> Running cargo fmt --all --check"
cargo fmt --all --check

echo "==> Running cargo test --workspace"
cargo test --workspace

echo "==> Running cargo check --workspace"
cargo check --workspace

echo "==> Building WASM using wasm-pack"
wasm-pack build crates/stonepen-wasm --target web --out-dir ../../web/pkg

echo "==> Done!"
