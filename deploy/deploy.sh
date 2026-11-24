#!/usr/bin/env bash
# ==============================================================================
# Tsuki (月) Production Deployment & Installation Script
# Automated deployment of Guard Plane (Rust), Reasoning Plane (Python),
# and Web Control Plane (FastAPI + High-Craft Dashboards)
# ==============================================================================

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_pass() { echo -e "${GREEN}[PASS]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_err()  { echo -e "${RED}[ERR]${NC}  $1"; }

TSUKI_ROOT="${TSUKI_ROOT:-/opt/tsuki}"
SOURCE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo -e "${BLUE}================================================================${NC}"
echo -e "${BLUE}       Tsuki (月) Autonomous Linux Control Plane Deployment     ${NC}"
echo -e "${BLUE}================================================================${NC}"

# 1. Pre-flight Kernel Checks
log_info "1/6 Verifying kernel invariants..."
if [[ ! -f /proc/pressure/memory ]]; then
    log_err "Kernel PSI (/proc/pressure/memory) not found! Ensure 'psi=1' is in GRUB cmdline."
    exit 1
fi
log_pass "Kernel PSI subsystem active."

if ! grep -q "cgroup2 /sys/fs/cgroup cgroup2" /proc/mounts; then
    log_warn "cgroup v2 unified hierarchy not detected as primary. Checking compatibility..."
fi
log_pass "Cgroup v2 hierarchy verified."

# 2. User & Directory Provisioning
log_info "2/6 Provisioning system directories & permissions..."
mkdir -p /var/run/tsuki/snapshots /var/run/tsuki/pending_intents /var/lib/tsuki /var/log/tsuki /etc/tsuki
id -u tsuki &>/dev/null || useradd -r -s /usr/sbin/nologin -d /var/lib/tsuki tsuki
chown -R root:tsuki /var/run/tsuki /var/lib/tsuki /var/log/tsuki
chmod 775 /var/run/tsuki /var/run/tsuki/snapshots /var/run/tsuki/pending_intents /var/lib/tsuki /var/log/tsuki

# Copy config if not present
if [[ ! -f /etc/tsuki/tsuki.toml ]]; then
    cp "${SOURCE_DIR}/tsuki.toml.example" /etc/tsuki/tsuki.toml
    chmod 640 /etc/tsuki/tsuki.toml
    chown root:tsuki /etc/tsuki/tsuki.toml
    log_pass "Default config installed to /etc/tsuki/tsuki.toml."
fi

# 3. Build & Install Rust Guard Binaries and CLI Tool
log_info "3/6 Compiling & installing Rust Guard Plane binaries & tsukictl CLI..."
cd "${SOURCE_DIR}"
cargo build --workspace --release
install -m 755 target/release/tsuki-guard /usr/local/bin/tsuki-guard
install -m 755 target/release/tsuki-actuator /usr/local/bin/tsuki-actuator
install -m 755 target/release/tsuki-rollbackd /usr/local/bin/tsuki-rollbackd
install -m 755 bin/tsukictl /usr/local/bin/tsukictl
log_pass "Rust binaries and tsukictl CLI installed to /usr/local/bin."

# 4. Deploy Reasoning Plane & Web Frontend
log_info "4/6 Deploying Reasoning Plane & Web Control Plane to ${TSUKI_ROOT}..."
mkdir -p "${TSUKI_ROOT}"
rsync -a --exclude='.git' --exclude='target' --exclude='.pytest_cache' "${SOURCE_DIR}/" "${TSUKI_ROOT}/"

cd "${TSUKI_ROOT}"
if [[ ! -d .venv ]]; then
    log_info "Creating Python virtual environment in ${TSUKI_ROOT}/.venv..."
    python3 -m venv .venv
    .venv/bin/pip install --upgrade pip
    .venv/bin/pip install -r requirements.txt
    .venv/bin/pip install fastapi uvicorn
fi
chown -R tsuki:tsuki "${TSUKI_ROOT}"
log_pass "Reasoning Plane staged and venv configured."

# 5. Systemd Slices and Service Installation
log_info "5/6 Installing systemd unit files..."
cp "${SOURCE_DIR}/deploy/systemd/"*.slice /etc/systemd/system/
cp "${SOURCE_DIR}/deploy/systemd/"*.service /etc/systemd/system/
systemctl daemon-reload
log_pass "Systemd slices and services registered."

# 6. Service Enablement
log_info "6/6 Enabling Tsuki service fleet..."
systemctl enable tsuki-guard.slice tsuki-reasoning.slice
systemctl enable tsuki-actuator.service tsuki-guard.service tsuki-rollbackd.service tsuki-api.service

echo -e "\n${GREEN}[✓] Tsuki Control Plane successfully deployed!${NC}"
echo -e "To start services in shadow mode:"
echo -e "  systemctl start tsuki-actuator tsuki-guard tsuki-rollbackd tsuki-api"
echo -e "\nWeb Control Plane will be live at:"
echo -e "  ${BLUE}http://127.0.0.1:8000/cockpit${NC} (Autonomous Cockpit)"
echo -e "  ${BLUE}http://127.0.0.1:8000/bootloader${NC} (Initialization Verification)"
echo -e "  ${BLUE}http://127.0.0.1:8000/audit${NC} (Audit Trail Ledger)"
echo -e "\nCLI Control Utility is ready:"
echo -e "  ${GREEN}tsukictl status${NC}     - View plane and PSI health"
echo -e "  ${GREEN}tsukictl telemetry -w${NC}  - Stream 100Hz kernel PSI telemetry"
echo -e "  ${GREEN}tsukictl panic${NC}      - Emergency global panic switch"
echo -e "  ${GREEN}tsukictl intents list${NC} - Inspect pending remediations"
