#!/bin/bash
# Build Linux binary for Terminal-Bench using Docker
# This creates a native Linux aarch64 binary that can be used in the container

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "=== Building Linux binary for Terminal-Bench ==="
echo "Project directory: $PROJECT_DIR"

# Create output directory
mkdir -p "$SCRIPT_DIR/linux_binary"

# Build using a Rust Docker container
docker run --rm \
    -v "$PROJECT_DIR:/workspace" \
    -w /workspace \
    rust:bookworm \
    bash -c "
        echo 'Installing dependencies...'
        apt-get update && apt-get install -y pkg-config libssl-dev

        echo 'Cleaning old artifacts...'
        rm -rf target/release/fluent 2>/dev/null || true

        echo 'Building fluent-cli...'
        cargo build --release -p fluent-cli

        echo 'Copying binary...'
        # The binary is named fluent-cli by cargo, but we want it as fluent
        cp target/release/fluent-cli /workspace/tbench_adapter/linux_binary/fluent
        chmod +x /workspace/tbench_adapter/linux_binary/fluent

        echo 'Build complete!'
        file /workspace/tbench_adapter/linux_binary/fluent
    "

echo "=== Linux binary built successfully ==="
echo "Binary location: $SCRIPT_DIR/linux_binary/fluent"
ls -la "$SCRIPT_DIR/linux_binary/fluent"
