#!/bin/sh
# Compile le moteur en WebAssembly et le copie dans web/. Aucune dépendance externe.
set -eu
cd "$(dirname "$0")"
cargo build -p perspective_core --target wasm32-unknown-unknown --release --locked --offline
cp target/wasm32-unknown-unknown/release/perspective_core.wasm web/engine.wasm
ls -l web/engine.wasm
