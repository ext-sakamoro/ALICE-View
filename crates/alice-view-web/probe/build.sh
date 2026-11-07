#!/usr/bin/env bash
# Builds the probe (the crate with feature `probe`) into probe/pkg for probe/check.mjs.
set -euo pipefail
here=$(cd "$(dirname "$0")/.." && pwd)
cd "$here"
cargo build --release --target wasm32-unknown-unknown --features probe
wasm-bindgen --target web --out-dir probe/pkg "${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/alice_view_web.wasm"
