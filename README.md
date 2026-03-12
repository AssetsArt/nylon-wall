# Nylon Wall

Network firewall built with Rust, eBPF, and Dioxus.

Uses [aya](https://github.com/aya-rs/aya) for kernel-space packet processing (XDP/TC) and [Dioxus](https://dioxuslabs.com/) 0.7 for the web management UI.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                 Dioxus Web UI (:8080)               │
│            (Management & Monitoring)                │
├─────────────────────────────────────────────────────┤
│               REST API Layer (:9450)                │
│                (axum HTTP server)                   │
├─────────────────────────────────────────────────────┤
│               Nylon Wall Daemon                     │
│     ┌──────────┬──────────┬──────────────┐          │
│     │ Rule     │ State    │ Logging &    │          │
│     │ Engine   │ Manager  │ Metrics      │          │
│     └──────────┴──────────┴──────────────┘          │
├─────────────────────────────────────────────────────┤
│            eBPF Userspace Controller                │
│              (aya - pure Rust)                      │
├─────────────────────────────────────────────────────┤
│              Linux Kernel (eBPF)                    │
│  ┌──────────┬──────────┬──────────┬──────────┐      │
│  │ XDP      │ TC       │ Cgroup   │ Socket   │      │
│  │ Programs │ Programs │ Programs │ Programs │      │
│  └──────────┴──────────┴──────────┴──────────┘      │
│         eBPF Maps (shared state/config)             │
└─────────────────────────────────────────────────────┘
```

## Project Structure

```
nylon-wall/
├── nylon-wall-common/     # Shared types (no_std + std)
├── nylon-wall-daemon/     # Userspace daemon (axum + SlateDB + aya)
├── nylon-wall-ebpf/       # eBPF programs (XDP/TC, no_std)
├── nylon-wall-ui/         # Dioxus 0.7 web UI (WASM)
├── docker-compose.yml     # Dev environment
├── spec.md                # Full specification
└── checklist.md           # Implementation checklist
```

## Tech Stack

| Component | Technology |
|-----------|-----------|
| eBPF Framework | [aya](https://github.com/aya-rs/aya) |
| Web UI | [Dioxus](https://dioxuslabs.com/) 0.7 + Tailwind CSS |
| HTTP Server | axum 0.8 |
| Database | [SlateDB](https://slatedb.io/) (embedded KV store) |
| Serialization | serde + serde_json |
| Logging | tracing |

## Quick Start (Docker)

```bash
docker compose up -d
```

- **UI**: http://localhost:8080
- **API**: http://localhost:9450/api/v1

## Development

### Prerequisites

- Rust 1.86+ (edition 2024)
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- [Dioxus CLI](https://dioxuslabs.com/): `cargo install dioxus-cli`
- For eBPF: Linux kernel >= 5.15, `bpf-linker`

### Build & Run

```bash
# Check workspace (common + daemon)
cargo check

# Run daemon (API server on :9450)
cargo run -p nylon-wall-daemon

# Run UI dev server (hot reload)
cd nylon-wall-ui && dx serve

# Check UI compiles
cargo check -p nylon-wall-ui --target wasm32-unknown-unknown

# Build eBPF programs (Linux only, requires nightly + bpf-linker)
cargo build -p nylon-wall-ebpf --target bpfel-unknown-none -Z build-std=core
```

### Docker Compose (Dev)

```bash
# Start both daemon + UI
docker compose up -d

# Rebuild after changes
docker compose up -d --build

# View logs
docker compose logs -f daemon
docker compose logs -f ui
```

## Features

- **Packet Filtering** — Ingress (XDP) and egress (TC) rule-based filtering
- **NAT** — SNAT, DNAT, and masquerade
- **Routing** — Static routes and policy-based routing
- **Network Policies** — Zone-based security model with inter-zone policies
- **Connection Tracking** — Stateful inspection (NEW/ESTABLISHED/RELATED/INVALID)
- **Rate Limiting** — Per-rule token bucket in eBPF
- **Monitoring** — Packet logging, Prometheus metrics, real-time dashboard

## API

Base: `http://localhost:9450/api/v1`

| Resource | Endpoints |
|----------|-----------|
| Rules | `GET/POST /rules`, `GET/PUT/DELETE /rules/{id}`, `POST /rules/{id}/toggle` |
| NAT | `GET/POST /nat`, `PUT/DELETE /nat/{id}` |
| Routes | `GET/POST /routes`, `PUT/DELETE /routes/{id}` |
| Zones | `GET/POST /zones`, `PUT/DELETE /zones/{id}` |
| Policies | `GET/POST /policies`, `PUT/DELETE /policies/{id}` |
| System | `GET /system/status` |

## UI Pages

| Page | Path | Description |
|------|------|-------------|
| Dashboard | `/` | Stats overview, recent rules |
| Rules | `/rules` | Firewall rule CRUD with toggle/delete |
| NAT | `/nat` | SNAT/DNAT/Masquerade management |
| Routes | `/routes` | Static route management |
| Policies | `/policies` | Zones and inter-zone policies |
| Logs | `/logs` | Packet log viewer |
| Settings | `/settings` | System info, backup/restore |

## License

MIT
