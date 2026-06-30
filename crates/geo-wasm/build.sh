#!/usr/bin/env bash
# Build geo-wasm WASM package for npm publish.
# Prerequisites: wasm-pack (cargo install wasm-pack)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== Building geo-wasm v0.2 ==="

# Build release WASM
wasm-pack build --target web --out-dir pkg --release

echo ""
echo "=== Build complete ==="
echo "Package: $(ls -lh pkg/geo_wasm_bg.wasm | awk '{print $5}') wasm"
echo ""
echo "To test locally:  npm link"
echo "To publish:       npm publish --access public"
