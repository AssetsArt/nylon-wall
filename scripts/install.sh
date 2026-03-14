#!/bin/sh
# Nylon Wall — one-line installer
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/AssetsArt/nylon-wall/main/scripts/install.sh | sh
#   curl -fsSL https://raw.githubusercontent.com/AssetsArt/nylon-wall/main/scripts/install.sh | sh -s -- --version v0.1.0
#
# Options:
#   --version <tag>   Install a specific version (default: latest)
#   --prefix <path>   Install prefix (default: /usr/local)
#   --help            Show this help

set -eu

REPO="AssetsArt/nylon-wall"
VERSION=""
PREFIX="/usr/local"
BIN_DIR=""
CONFIG_DIR="/etc/nylon-wall"
DATA_DIR="/var/lib/nylon-wall"

# ─── Colors ───────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

log()   { printf "${CYAN}[nylon-wall]${NC} %s\n" "$*"; }
ok()    { printf "${GREEN}[nylon-wall]${NC} %s\n" "$*"; }
warn()  { printf "${YELLOW}[nylon-wall]${NC} %s\n" "$*"; }
error() { printf "${RED}[nylon-wall]${NC} %s\n" "$*" >&2; }
fatal() { error "$*"; exit 1; }

# ─── Parse arguments ─────────────────────────────────────────────────
while [ $# -gt 0 ]; do
    case "$1" in
        --version)  VERSION="$2"; shift 2 ;;
        --prefix)   PREFIX="$2"; shift 2 ;;
        --help|-h)  sed -n '2,10p' "$0" | sed 's/^# \?//'; exit 0 ;;
        *)          fatal "Unknown option: $1" ;;
    esac
done

BIN_DIR="${PREFIX}/bin"

# ─── Platform detection ──────────────────────────────────────────────
detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "$OS" in
        Linux)  ;;
        *)      fatal "Nylon Wall only supports Linux (got: $OS)" ;;
    esac

    case "$ARCH" in
        x86_64|amd64)   ARCH="x86_64" ;;
        aarch64|arm64)  ARCH="aarch64" ;;
        *)              fatal "Unsupported architecture: $ARCH" ;;
    esac

    log "Detected platform: linux/${ARCH}"
}

# ─── Fetch latest version ────────────────────────────────────────────
resolve_version() {
    if [ -n "$VERSION" ]; then
        log "Using specified version: ${VERSION}"
        return
    fi

    log "Fetching latest version..."

    if command -v curl >/dev/null 2>&1; then
        VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
    elif command -v wget >/dev/null 2>&1; then
        VERSION=$(wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
    else
        fatal "Neither curl nor wget found. Please install one of them."
    fi

    if [ -z "$VERSION" ]; then
        fatal "Could not determine latest version. Use --version to specify."
    fi

    log "Latest version: ${VERSION}"
}

# ─── Download ─────────────────────────────────────────────────────────
download() {
    url="$1"
    dest="$2"

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$url" -o "$dest"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$dest" "$url"
    fi
}

# ─── Check root ───────────────────────────────────────────────────────
need_root() {
    if [ "$(id -u)" -ne 0 ]; then
        if command -v sudo >/dev/null 2>&1; then
            SUDO="sudo"
        else
            fatal "This installer needs root privileges. Please run with sudo."
        fi
    else
        SUDO=""
    fi
}

# ─── Install ──────────────────────────────────────────────────────────
install_nylon_wall() {
    TMPDIR=$(mktemp -d)
    trap 'rm -rf "$TMPDIR"' EXIT

    ARTIFACT_PREFIX="nylon-wall-linux-${ARCH}"
    BASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"

    # Download daemon
    DAEMON_URL="${BASE_URL}/${ARTIFACT_PREFIX}-nylon-wall-daemon"
    log "Downloading nylon-wall-daemon..."
    download "$DAEMON_URL" "$TMPDIR/nylon-wall-daemon" || fatal "Failed to download daemon"
    chmod +x "$TMPDIR/nylon-wall-daemon"

    # Download eBPF binary
    EBPF_URL="${BASE_URL}/${ARTIFACT_PREFIX}-nylon-wall-ebpf"
    log "Downloading nylon-wall-ebpf..."
    download "$EBPF_URL" "$TMPDIR/nylon-wall-ebpf" || warn "eBPF binary not available (optional)"

    # Download config
    CONFIG_URL="${BASE_URL}/${ARTIFACT_PREFIX}-config.toml"
    log "Downloading default config..."
    download "$CONFIG_URL" "$TMPDIR/config.toml" || warn "Default config not available"

    # Download UI
    UI_URL="${BASE_URL}/nylon-wall-ui.tar.gz"
    log "Downloading web UI..."
    download "$UI_URL" "$TMPDIR/nylon-wall-ui.tar.gz" || warn "UI package not available"

    # Install binaries
    log "Installing to ${BIN_DIR}..."
    $SUDO mkdir -p "$BIN_DIR"
    $SUDO install -m 755 "$TMPDIR/nylon-wall-daemon" "$BIN_DIR/nylon-wall-daemon"

    if [ -f "$TMPDIR/nylon-wall-ebpf" ]; then
        $SUDO install -m 644 "$TMPDIR/nylon-wall-ebpf" "$BIN_DIR/nylon-wall-ebpf"
    fi

    # Install config
    if [ -f "$TMPDIR/config.toml" ] && [ ! -f "$CONFIG_DIR/config.toml" ]; then
        $SUDO mkdir -p "$CONFIG_DIR"
        $SUDO install -m 644 "$TMPDIR/config.toml" "$CONFIG_DIR/config.toml"
        ok "Installed default config to ${CONFIG_DIR}/config.toml"
    fi

    # Install UI
    UI_DIR="${PREFIX}/share/nylon-wall/ui"
    if [ -f "$TMPDIR/nylon-wall-ui.tar.gz" ]; then
        $SUDO mkdir -p "$UI_DIR"
        $SUDO tar -xzf "$TMPDIR/nylon-wall-ui.tar.gz" -C "$UI_DIR"
        ok "Installed web UI to ${UI_DIR}"
    fi

    # Create data directory
    $SUDO mkdir -p "$DATA_DIR"

    # Install systemd service
    install_systemd_service
}

# ─── Systemd service ─────────────────────────────────────────────────
install_systemd_service() {
    if [ ! -d /etc/systemd/system ]; then
        warn "systemd not found, skipping service installation"
        return
    fi

    SERVICE_FILE="/etc/systemd/system/nylon-wall.service"
    if [ -f "$SERVICE_FILE" ]; then
        warn "Service file already exists, skipping"
        return
    fi

    log "Installing systemd service..."
    $SUDO tee "$SERVICE_FILE" > /dev/null << 'UNIT'
[Unit]
Description=Nylon Wall Firewall Daemon
After=network.target
Wants=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/nylon-wall-daemon
Restart=on-failure
RestartSec=5
LimitNOFILE=65536
AmbientCapabilities=CAP_NET_ADMIN CAP_SYS_ADMIN CAP_BPF CAP_NET_RAW

[Install]
WantedBy=multi-user.target
UNIT

    $SUDO systemctl daemon-reload
    ok "Installed systemd service: nylon-wall"
}

# ─── Main ─────────────────────────────────────────────────────────────
main() {
    echo ""
    printf "${BOLD}${CYAN}  ┌─────────────────────────────┐${NC}\n"
    printf "${BOLD}${CYAN}  │     Nylon Wall Installer     │${NC}\n"
    printf "${BOLD}${CYAN}  └─────────────────────────────┘${NC}\n"
    echo ""

    detect_platform
    resolve_version
    need_root
    install_nylon_wall

    echo ""
    printf "${BOLD}${GREEN}  ✓ Nylon Wall ${VERSION} installed successfully!${NC}\n"
    echo ""
    log "Quick start:"
    log "  sudo systemctl enable --now nylon-wall"
    log "  Open http://localhost:9450 in your browser"
    echo ""
    log "Config:  ${CONFIG_DIR}/config.toml"
    log "Data:    ${DATA_DIR}"
    log "Logs:    journalctl -u nylon-wall -f"
    echo ""
}

main
