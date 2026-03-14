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

### Phase 1: Foundation
- [x] Workspace & config
- [x] Shared types (nylon-wall-common)
- [x] eBPF programs (XDP ingress, TC egress)
- [x] Daemon (axum API + SlateDB)
- [x] Docker dev environment

### Phase 2: Core Firewall
- [x] eBPF maps (ingress/egress rules, conntrack, events)
- [x] Connection tracking (NEW/ESTABLISHED/RELATED/INVALID)
- [x] Firewall rules CRUD + toggle/reorder
- [x] Web UI (Dioxus 0.7 dark theme + rule management)
- [ ] eBPF packet filtering ทดสอบบน Linux

### Phase 3: NAT & Routing
- [x] NAT API (SNAT/DNAT/Masquerade)
- [x] Static & policy-based routing API
- [x] NAT UI + port forward wizard
- [x] Route & policy route UI
- [ ] NAT eBPF processing (rewrite src/dst IP)
- [ ] Policy routing marks ใน eBPF

### Phase 4: Network Policy & Zones
- [x] Zone & policy CRUD API
- [x] Schedule-based policy evaluation
- [x] Zone & policy UI + schedule editor
- [ ] Zone eBPF maps (ifindex → zone_id, zone pair → policy)

### Phase 5: Monitoring
- [x] Prometheus metrics endpoint
- [x] Packet log reader + API
- [x] WebSocket real-time events
- [x] Dashboard, conntrack table, log viewer UI
- [ ] eBPF metrics & rate limit maps
- [ ] Log TTL auto-cleanup

### Phase 6: System & Hardening
- [x] System API (interfaces, status, apply, backup/restore)
- [x] Settings UI + interface config
- [x] CI/CD & installer
- [ ] Rate limiting / QoS (token bucket ใน eBPF)
- [ ] IPv6 full support
- [ ] Performance tuning & benchmarking

### Phase 7: DHCP
- [x] DHCP server (pools, leases, reservations)
- [x] DHCP client (WAN interface)
- [x] DHCP UI (3-tab layout)
- [x] Dashboard DHCP summary
- [ ] ทดสอบ DHCP server/client บน Linux

### Future
- [ ] DNS filtering (blocklist + query logging)
- [ ] VPN (WireGuard / IPsec)
- [ ] IDS/IPS integration
- [ ] Traffic shaping (QoS)
- [ ] High availability (HA)
- [ ] Multi-WAN failover
- [ ] SSL/TLS inspection

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## License

[MIT](LICENSE)
