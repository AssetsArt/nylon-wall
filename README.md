<p align="center">
  <img src="https://img.shields.io/badge/rust-2024_edition-orange?logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/eBPF-XDP%20%2B%20TC-blueviolet" alt="eBPF">
  <img src="https://img.shields.io/badge/UI-Dioxus%200.7-blue" alt="Dioxus">
  <img src="https://img.shields.io/github/license/AssetsArt/nylon-wall" alt="License">
  <img src="https://img.shields.io/github/v/release/AssetsArt/nylon-wall" alt="Release">
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
- **DHCP** — Built-in DHCP server (pools, reservations, leases) and WAN DHCP client
- **Routing** — Static routes and policy-based routing (source/dest/port/protocol)
- **Network Zones & Policies** — Zone-based security model with inter-zone rules and time-based schedules
- **Rate Limiting** — Per-rule token bucket enforcement in eBPF
- **Web UI** — Modern dark-theme dashboard built with Dioxus + Tailwind CSS
- **REST API** — Full CRUD API for automation and integration
- **Real-time Monitoring** — Packet logging, metrics, WebSocket events
- **Backup & Restore** — Export/import full configuration

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
│     │ NAT      │ Policy   │ Schedule     │          │
│     │ Manager  │ Router   │ Engine       │          │
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

## Roadmap

- [x] Packet filtering (XDP/TC)
- [x] Firewall rules with enable/disable
- [x] NAT (SNAT/DNAT/Masquerade)
- [x] Static & policy-based routing
- [x] Zone-based network policies
- [x] DHCP server & client
- [x] Web management UI
- [x] Backup & restore
- [x] CI/CD & installer
- [ ] VPN (WireGuard / IPsec)
- [ ] IDS/IPS integration
- [ ] Traffic shaping (QoS)
- [ ] High availability (HA)
- [ ] Multi-WAN failover
- [ ] SSL/TLS inspection
- [ ] Prometheus metrics export

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## License

[MIT](LICENSE)
