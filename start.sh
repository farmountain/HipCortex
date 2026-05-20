#!/usr/bin/env bash
# HipCortex start script for Replit
# Downloads the Linux AMD64 binary and starts the server on port 3030

set -e

BINARY="hipcortex-server"
RELEASE_URL="https://github.com/farmountain/HipCortex/releases/latest/download/hipcortex-linux-amd64"

if [ ! -f "$BINARY" ]; then
    echo "Downloading HipCortex binary..."
    curl -fsSL "$RELEASE_URL" -o "$BINARY"
    chmod +x "$BINARY"
    echo "✓ Downloaded"
fi

mkdir -p data
export DATA_DIR="./data"
export PORT="${PORT:-3030}"
export RUST_LOG="${RUST_LOG:-info}"

echo "Starting HipCortex on port $PORT..."
echo "Docs: https://github.com/farmountain/HipCortex"
exec "./$BINARY"
