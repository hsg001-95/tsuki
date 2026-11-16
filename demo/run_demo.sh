#!/usr/bin/env bash
set -euo pipefail

# Tsuki MVP End-to-End Demo Script (Milestone M5)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "🌙 Starting Tsuki (月) End-to-End Demonstration..."
cd "${WORKSPACE}"

# Build Rust binaries if not present
if [ ! -f "target/debug/tsuki-guard" ] || [ ! -f "target/debug/tsuki-actuator" ]; then
    echo "[*] Compiling Tsuki Rust Workspace..."
    cargo build --workspace
fi

# Run python demonstration script
.venv/bin/python demo/demo_runner.py
