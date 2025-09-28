#!/bin/bash
set -e

echo "🔧 Building Shekere WASM module..."

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "❌ wasm-pack not found. Installing..."
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

# Clean previous build
echo "🧹 Cleaning previous build..."
rm -rf pkg/

# Build the WASM package
echo "🚀 Building WASM package..."
wasm-pack build --target web --out-dir pkg --dev

echo "✅ WASM build complete!"
echo ""
echo "📋 To test the WASM module:"
echo "   1. Start a local web server in this directory:"
echo "      python3 -m http.server 8000"
echo "      # or"
echo "      npx serve -p 8000"
echo ""
echo "   2. Open http://localhost:8000/test.html in your browser"
echo ""
echo "⚠️  Make sure your browser supports WebGPU:"
echo "   - Chrome/Edge: Enable chrome://flags/#enable-unsafe-webgpu"
echo "   - Firefox: Set dom.webgpu.enabled=true in about:config"