<p align="center">
  <img src="https://img.shields.io/badge/rust-2024_edition-orange?logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/eBPF-XDP%20%2B%20TC-blueviolet" alt="eBPF">
  <img src="https://img.shields.io/badge/UI-Dioxus%200.7-blue" alt="Dioxus">
  <img src="https://img.shields.io/badge/License-MIT-blue" alt="License">
  <a href="https://github.com/AssetsArt/nylon-wall/releases/latest"><img src="https://img.shields.io/github/v/release/AssetsArt/nylon-wall?display_name=tag" alt="Release"></a>
</p>

# Nylon Wall

**Open-source Linux network firewall** built entirely in Rust — using eBPF for high-performance kernel-space packet processing and a modern web UI for management.

Designed for homelabs, edge networks, and small/medium infrastructure.

## Quick Install

```bash
curl -fsSL https://raw.githubusercontent.com/AssetsArt/nylon-wall/main/scripts/install.sh | sh
```

Or with a specific version:

```bash
curl -fsSL https://raw.githubusercontent.com/AssetsArt/nylon-wall/main/scripts/install.sh | sh -s -- --version v0.1.0
```

After installation:

```bash
sudo systemctl enable --now nylon-wall
# Open http://localhost:9450
```

## Features

- **eBPF Packet Filtering** — XDP ingress + TC egress, line-rate performance in kernel space
- **Stateful Firewall** — Connection tracking (NEW/ESTABLISHED/RELATED/INVALID)
- **NAT** — SNAT, DNAT, masquerade, with a port-forward wizard
- **L4 Proxy** — Layer 4 load balancer with eBPF DNAT/SNAT, round-robin & IP hash strategies
- **TLS / SNI Filtering** — Block or allow traffic by domain name using SNI inspection in eBPF
- **WireGuard VPN** — Integrated VPN server with peer management and config download
- **VLAN & Bridge** — 802.1Q VLANs and Linux bridge management
- **DHCP** — Built-in DHCP server (pools, reservations, leases) and WAN DHCP client
- **Dynamic DNS** — Auto-update DNS records (Cloudflare, custom providers)
- **Routing** — Static routes and policy-based routing (source/dest/port/protocol)
- **Network Zones & Policies** — Zone-based security model with inter-zone rules and time-based schedules
- **Rate Limiting** — Per-rule token bucket enforcement in eBPF
- **Authentication** — JWT auth with brute-force protection, OAuth/OIDC support
- **Web UI** — Modern dark-theme dashboard built with Dioxus + Tailwind CSS
- **REST API** — Full CRUD API for automation and integration
- **Real-time Monitoring** — Packet logging, metrics, WebSocket events
- **Network Tools** — Ping, DNS lookup, traceroute, Wake-on-LAN, mDNS reflector
- **Backup & Restore** — Export/import full configuration with auto-revert safety

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                 Dioxus Web UI (:9450)               │
│            (Management & Monitoring)                │
├─────────────────────────────────────────────────────┤
│               REST API + WebSocket                  │
│                (axum HTTP server)                   │
├─────────────────────────────────────────────────────┤
│               Nylon Wall Daemon                     │
│     ┌──────────┬──────────┬──────────────┐          │
│     │ Rule     │ DHCP     │ Logging &    │          │
│     │ Engine   │ Server   │ Metrics      │          │
│     └──────────┴──────────┴──────────────┘          │
│     ┌──────────┬──────────┬──────────────┐          │
│     │ NAT /    │ Policy   │ WireGuard    │          │
│     │ L4 Proxy │ Router   │ VPN          │          │
│     └──────────┴──────────┴──────────────┘          │
│     ┌──────────┬──────────┬──────────────┐          │
│     │ VLAN /   │ DDNS /   │ SNI          │          │
│     │ Bridge   │ mDNS     │ Filter       │          │
│     └──────────┴──────────┴──────────────┘          │
├─────────────────────────────────────────────────────┤
│         eBPF Programs (aya - pure Rust)             │
│  ┌──────────┬──────────┬─────────────────────┐      │
│  │ XDP      │ TC       │ Connection          │      │
│  │ Ingress  │ Egress   │ Tracking            │      │
│  └──────────┴──────────┴─────────────────────┘      │
│           eBPF Maps (shared state)                  │
├─────────────────────────────────────────────────────┤
│                  Linux Kernel                       │
└─────────────────────────────────────────────────────┘
```

## Development

### Prerequisites

- Rust 1.86+ (edition 2024)
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- [Dioxus CLI](https://dioxuslabs.com/): `cargo install dioxus-cli`
- For eBPF: Linux kernel >= 5.15, nightly Rust, `bpf-linker`

### Build & Run

```bash
# Check workspace
cargo check

# Run daemon (API on :9450)
cargo run -p nylon-wall-daemon

# Run UI dev server (hot reload)
cd nylon-wall-ui && dx serve

# Build eBPF (Linux only, requires nightly)
cargo +nightly build -p nylon-wall-ebpf --target bpfel-unknown-none -Z build-std=core

# Build release
./scripts/build-release.sh
```

### Testing

```bash
# Unit tests
cargo test -p nylon-wall-common -p nylon-wall-daemon --lib

# Integration tests (requires wireguard-tools)
cargo test -p nylon-wall-daemon --test '*'

# Docker integration tests
docker compose -f docker-compose.test.yml up --build --abort-on-container-exit
```

### Docker

```bash
docker compose up -d        # Start
docker compose up -d --build # Rebuild
docker compose logs -f       # Logs
```

## Configuration

Default config: `/etc/nylon-wall/config.toml`

```toml
[daemon]
listen_addr = "0.0.0.0:9450"

[database]
path = "/var/lib/nylon-wall/slatedb"

[ebpf]
mode = "xdp"                # "xdp", "tc", or "both"
interfaces = ["eth0"]       # or ["all"] for auto-detect

[logging]
level = "info"              # trace, debug, info, warn, error
max_log_entries = 100000
log_ttl_seconds = 604800    # 7 days

[ui]
bind_addr = "0.0.0.0:8080"
```

## Releasing

```bash
# Tag a release (triggers GitHub Actions)
./scripts/tag-release.sh 0.1.0

# Or build locally
./scripts/build-release.sh --output dist
```

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## License

[MIT](LICENSE)
