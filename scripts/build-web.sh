#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if ! command -v wasm-bindgen >/dev/null; then
  echo 'Install the matching generator: cargo install wasm-bindgen-cli --version 0.2.104 --locked' >&2
  exit 1
fi
if [[ "$(wasm-bindgen --version)" != 'wasm-bindgen 0.2.104' ]]; then
  echo 'This build requires wasm-bindgen-cli 0.2.104 (matching Cargo.toml).' >&2
  exit 1
fi
cargo build --locked --lib --release --target wasm32-unknown-unknown
mkdir -p build/site/pkg
wasm-bindgen target/wasm32-unknown-unknown/release/boxmonitor.wasm --target web --out-dir build/site/pkg --no-typescript
cp web/index.html web/style.css web/app.js build/site/
touch build/site/.nojekyll
printf 'Built static demo in build/site\n'
