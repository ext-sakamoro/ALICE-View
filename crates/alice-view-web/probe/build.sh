#!/usr/bin/env bash
# Builds the probe (the crate with feature `probe`) into probe/pkg for probe/check.mjs.
set -euo pipefail
here=$(cd "$(dirname "$0")/.." && pwd)
cd "$here"
# the CLI must be the version of the wasm-bindgen crate the build resolves: say so now, not after the build
python3 probe/wasm_bindgen_match.py
cargo build --release --target wasm32-unknown-unknown --features probe
wasm-bindgen --target web --out-dir probe/pkg "${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/alice_view_web.wasm"
