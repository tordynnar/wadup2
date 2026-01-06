#!/bin/bash
set -e

echo "Starting Go WASM build..."

cd /build/src

# Set environment for WASI build
export GOOS=wasip1
export GOARCH=wasm

# Build
go build -o /build/output/module.wasm .

echo "Build complete! WASM file written to /build/output/module.wasm"
