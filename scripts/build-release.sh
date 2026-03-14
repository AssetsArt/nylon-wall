#!/bin/bash
# Build nylon-wall release binaries for the current platform.
#
# Usage:
#   ./scripts/build-release.sh                         # build all components
#   ./scripts/build-release.sh --components daemon      # build specific components
#   ./scripts/build-release.sh --output dist            # custom output directory
#   ./scripts/build-release.sh --target x86_64-unknown-linux-gnu  # cross-compile
#
# Components:
#   daemon   - nylon-wall-daemon (firewall daemon, Linux only)
#   ebpf     - nylon-wall-ebpf (eBPF programs, Linux only, requires nightly)
#   ui       - nylon-wall-ui (web dashboard, requires dioxus-cli)
#
# Output:
#   <output_dir>/<binary>           – stripped release binaries
#   <output_dir>/checksums.sha256   – SHA256 checksums

set -euo pipefail

# ─── Configuration ────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUTPUT_DIR="dist"
TARGET=""
COMPONENTS=()
JOBS=""

# ─── Colors ───────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log()   { echo -e "${CYAN}[build-release]${NC} $*"; }
ok()    { echo -e "${GREEN}[build-release]${NC} $*"; }
warn()  { echo -e "${YELLOW}[build-release]${NC} $*"; }
error() { echo -e "${RED}[build-release]${NC} $*" >&2; }

# ─── Parse arguments ─────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --output|-o)
            OUTPUT_DIR="$2"; shift 2
            ;;
        --target|-t)
            TARGET="$2"; shift 2
            ;;
        --components|-c)
            shift
            while [[ $# -gt 0 && ! "$1" =~ ^-- ]]; do
                COMPONENTS+=("$1"); shift
            done
            ;;
        --jobs|-j)
            JOBS="$2"; shift 2
            ;;
        --help|-h)
            head -16 "$0" | tail -15 | sed 's/^# \?//'
            exit 0
            ;;
        *)
            error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# ─── Platform detection ──────────────────────────────────────────────
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)  PLATFORM="linux" ;;
    Darwin) PLATFORM="macos" ;;
    *)      error "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
    x86_64|amd64)   ARCH_LABEL="x86_64" ;;
    aarch64|arm64)  ARCH_LABEL="aarch64" ;;
    *)              error "Unsupported architecture: $ARCH"; exit 1 ;;
esac

log "Platform: ${PLATFORM}/${ARCH_LABEL}"

# ─── Determine components to build ───────────────────────────────────
if [[ ${#COMPONENTS[@]} -eq 0 ]]; then
    if [[ "$PLATFORM" == "linux" ]]; then
        COMPONENTS=(daemon ebpf ui)
    else
        COMPONENTS=(ui)
        warn "Daemon and eBPF are Linux-only; building UI only"
    fi
fi

log "Components: ${COMPONENTS[*]}"

# ─── Setup output directory ──────────────────────────────────────────
OUTPUT_PATH="$PROJECT_ROOT/$OUTPUT_DIR"
mkdir -p "$OUTPUT_PATH"

# ─── Build arguments ─────────────────────────────────────────────────
CARGO_ARGS=(--release)
if [[ -n "$TARGET" ]]; then
    CARGO_ARGS+=(--target "$TARGET")
fi
if [[ -n "$JOBS" ]]; then
    CARGO_ARGS+=(--jobs "$JOBS")
fi

# Determine target directory
if [[ -n "$TARGET" ]]; then
    TARGET_DIR="$PROJECT_ROOT/target/$TARGET/release"
else
    TARGET_DIR="$PROJECT_ROOT/target/release"
fi

# ─── Build each component ────────────────────────────────────────────
BUILT=()
FAILED=()

for comp in "${COMPONENTS[@]}"; do
    case "$comp" in
        daemon)
            if [[ "$PLATFORM" != "linux" ]]; then
                warn "Skipping daemon (Linux only)"
                continue
            fi
            log "Building nylon-wall-daemon..."
            if cargo build -p nylon-wall-daemon "${CARGO_ARGS[@]}" 2>&1; then
                if [[ -f "$TARGET_DIR/nylon-wall-daemon" ]]; then
                    cp "$TARGET_DIR/nylon-wall-daemon" "$OUTPUT_PATH/nylon-wall-daemon"
                    BUILT+=(nylon-wall-daemon)
                    ok "Built nylon-wall-daemon"
                else
                    error "Binary not found"; FAILED+=(daemon)
                fi
            else
                error "Failed to build daemon"; FAILED+=(daemon)
            fi
            ;;

        ebpf)
            if [[ "$PLATFORM" != "linux" ]]; then
                warn "Skipping eBPF (Linux only)"
                continue
            fi
            log "Building nylon-wall-ebpf (nightly)..."
            if cargo +nightly build \
                -p nylon-wall-ebpf \
                --target bpfel-unknown-none \
                -Z build-std=core \
                --release 2>&1; then
                EBPF_BIN="$PROJECT_ROOT/target/bpfel-unknown-none/release/nylon-wall-ebpf"
                if [[ -f "$EBPF_BIN" ]]; then
                    cp "$EBPF_BIN" "$OUTPUT_PATH/nylon-wall-ebpf"
                    BUILT+=(nylon-wall-ebpf)
                    ok "Built nylon-wall-ebpf"
                else
                    error "eBPF binary not found"; FAILED+=(ebpf)
                fi
            else
                error "Failed to build eBPF"; FAILED+=(ebpf)
            fi
            ;;

        ui)
            log "Building nylon-wall-ui..."
            if ! command -v dx &>/dev/null; then
                warn "dioxus-cli not found, installing..."
                cargo install dioxus-cli
            fi
            cd "$PROJECT_ROOT/nylon-wall-ui"
            if dx build --release 2>&1; then
                UI_DIR="target/dx/nylon-wall-ui/release/web/public"
                if [[ -d "$UI_DIR" ]]; then
                    tar -czf "$OUTPUT_PATH/nylon-wall-ui.tar.gz" -C "$UI_DIR" .
                    BUILT+=(nylon-wall-ui.tar.gz)
                    ok "Built nylon-wall-ui.tar.gz"
                else
                    error "UI build output not found"; FAILED+=(ui)
                fi
            else
                error "Failed to build UI"; FAILED+=(ui)
            fi
            cd "$PROJECT_ROOT"
            ;;

        *)
            error "Unknown component: $comp"
            error "Valid components: daemon ebpf ui"
            FAILED+=("$comp")
            ;;
    esac
done

# ─── Strip binaries ──────────────────────────────────────────────────
log "Stripping binaries..."
for bin in "${BUILT[@]}"; do
    bin_path="$OUTPUT_PATH/$bin"
    if file "$bin_path" 2>/dev/null | grep -q "ELF"; then
        strip "$bin_path" 2>/dev/null || warn "Could not strip $bin"
    fi
done

# ─── Copy config ─────────────────────────────────────────────────────
if [[ -f "$PROJECT_ROOT/config.toml" ]]; then
    cp "$PROJECT_ROOT/config.toml" "$OUTPUT_PATH/config.toml"
    BUILT+=(config.toml)
fi

# ─── Generate checksums ──────────────────────────────────────────────
log "Generating checksums..."
cd "$OUTPUT_PATH"
if command -v sha256sum &>/dev/null; then
    sha256sum "${BUILT[@]}" > checksums.sha256
elif command -v shasum &>/dev/null; then
    shasum -a 256 "${BUILT[@]}" > checksums.sha256
fi
cd "$PROJECT_ROOT"

# ─── Summary ─────────────────────────────────────────────────────────
echo ""
log "═══════════════════════════════════════════════════"
log "  Build Release Summary"
log "═══════════════════════════════════════════════════"
echo ""

for bin in "${BUILT[@]}"; do
    size=$(du -h "$OUTPUT_PATH/$bin" | cut -f1)
    ok "  $bin  ($size)"
done

if [[ ${#FAILED[@]} -gt 0 ]]; then
    echo ""
    for comp in "${FAILED[@]}"; do
        error "  FAILED: $comp"
    done
fi

echo ""
log "Output: $OUTPUT_PATH/"
log "Checksums: $OUTPUT_PATH/checksums.sha256"

if [[ ${#FAILED[@]} -gt 0 ]]; then
    error "${#FAILED[@]} component(s) failed to build"
    exit 1
fi

ok "All ${#BUILT[@]} components built successfully"
