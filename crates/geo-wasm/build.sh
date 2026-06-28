#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

echo "=== Building geo-wasm ==="
wasm-pack build --target web --out-dir pkg

echo "=== Copying demo ==="
cp examples/demo.html pkg/

echo ""
echo "=== Build complete ==="
echo "Run:  npx serve pkg -l 8080 -C"
echo "Then open: http://localhost:8080/demo.html"
