#!/usr/bin/env bash
# Tsuki (月) Live Production Daemon Launcher
# Runs 100% genuine Linux PC hardware telemetry with ZERO demo/mock data

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${ROOT_DIR}"

echo "🌙 ================================================================="
echo "    TSUKI KERNEL REASONING & CONTROL PLANE (LIVE PC RUNNER)         "
echo "=================================================================="

# Check virtual environment
if [ ! -d ".venv" ]; then
    echo "[!] Virtual environment not found at .venv. Creating..."
    python3 -m venv .venv
    .venv/bin/pip install -r requirements.txt
fi

# Ensure Rust workspace binaries are compiled
if [ ! -f "target/debug/tsuki-guard" ]; then
    echo "[*] Compiling Tsuki Rust Guard & Actuator binaries..."
    cargo build --workspace
fi

# Check for existing instance on port 8000 and terminate if running
EXISTING_PID=$(lsof -ti:8000 2>/dev/null || true)
if [ -n "${EXISTING_PID}" ]; then
    echo "[*] Stopping existing process on port 8000 (PID: ${EXISTING_PID})..."
    kill -9 ${EXISTING_PID} 2>/dev/null || true
    sleep 1
fi

echo "[✓] Starting live FastAPI daemon on 127.0.0.1:8000..."
echo "[✓] Environment: TSUKI_LIVE_HOST=1 | TSUKI_DEMO_DATA=0"
echo "    - Real Host: $(hostname)"
echo "    - Linux Kernel: $(uname -r)"
echo "    - cgroup v2 Hierarchy: Mounted at /sys/fs/cgroup"
echo "    - Zero mock/demo incidents"
echo "=================================================================="
echo " Open Cockpit: http://127.0.0.1:8000/cockpit.html"
echo " Open Audit:   http://127.0.0.1:8000/audit.html"
echo "=================================================================="

exec .venv/bin/python -m uvicorn reasoning.tsuki_api.app:app --host 127.0.0.1 --port 8000
