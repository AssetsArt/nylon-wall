# Nylon Wall - Implementation Checklist

## Phase 1: Foundation

### Workspace & Config
- [x] `Cargo.toml` - Workspace root (members: common, daemon, ui; exclude: ebpf)
- [ ] `/etc/nylon-wall/config.toml` - Default config template

### nylon-wall-common
- [x] `nylon-wall-common/Cargo.toml`
- [x] `nylon-wall-common/src/lib.rs` - Re-exports
- [x] `nylon-wall-common/src/rule.rs` - `FirewallRule`, `Direction`, `RuleAction`, `PortRange`
- [x] `nylon-wall-common/src/nat.rs` - `NatEntry`, `NatType`
- [x] `nylon-wall-common/src/route.rs` - `Route`, `PolicyRoute`
- [x] `nylon-wall-common/src/zone.rs` - `Zone`, `NetworkPolicy`, `Schedule`
- [x] `nylon-wall-common/src/conntrack.rs` - `ConntrackEntry`, `ConnState`
- [x] `nylon-wall-common/src/log.rs` - `PacketLog`, `MetricPoint`
- [x] `nylon-wall-common/src/protocol.rs` - `Protocol` enum, shared constants

### nylon-wall-ebpf
- [x] `nylon-wall-ebpf/Cargo.toml`
- [x] `nylon-wall-ebpf/src/main.rs` - eBPF entrypoint (XDP pass-all)
- [x] `nylon-wall-ebpf/src/common.rs` - Shared eBPF constants
- [x] `nylon-wall-ebpf/src/ingress.rs` - XDP ingress placeholder
- [x] `nylon-wall-ebpf/src/egress.rs` - TC egress placeholder
- [ ] ทดสอบ load/attach บน test interface

### nylon-wall-daemon
- [x] `nylon-wall-daemon/Cargo.toml`
- [x] `nylon-wall-daemon/src/main.rs` - Daemon entrypoint + tokio runtime
- [x] `nylon-wall-daemon/src/ebpf_loader.rs` - Load & attach eBPF programs (stub)
- [x] `nylon-wall-daemon/src/db.rs` - SlateDB init + helpers (open, get, put, scan, delete)
- [x] `nylon-wall-daemon/src/api.rs` - axum router with full CRUD endpoints
- [x] `nylon-wall-daemon/src/rule_engine.rs` - In-memory rule management
- [x] `nylon-wall-daemon/src/state.rs` - Conntrack reader (stub)
- [ ] ทดสอบ packet drop/allow บน test interface

### Docker
- [x] `docker-compose.yml` - Dev environment (daemon + UI)
- [x] `Dockerfile.daemon` - Multi-stage build (rust → debian-slim)
- [x] `Dockerfile.ui` - Multi-stage build (rust+dx → nginx)
- [x] `nginx.conf` - SPA fallback + API reverse proxy

---

## Phase 2: Core Firewall

### eBPF Programs
- [ ] `nylon-wall-ebpf/src/ingress.rs` - XDP ingress filter + rule evaluation
- [ ] `nylon-wall-ebpf/src/egress.rs` - TC egress filter + rule evaluation
- [ ] eBPF maps: `ingress_rules`, `egress_rules` (Array)
- [ ] eBPF maps: `conntrack` (LRU HashMap)
- [ ] eBPF maps: `events` (PerfEventArray)
- [ ] Connection tracking logic ใน eBPF (NEW/ESTABLISHED/RELATED/INVALID)

### Daemon - Rule Engine
- [x] `nylon-wall-daemon/src/rule_engine.rs` - Rule CRUD + compile to eBPF maps
- [x] `nylon-wall-daemon/src/state.rs` - Conntrack reader from eBPF maps (stub)
- [x] API: `GET /api/v1/rules` - List rules
- [x] API: `POST /api/v1/rules` - Create rule
- [x] API: `GET /api/v1/rules/{id}` - Get rule
- [x] API: `PUT /api/v1/rules/{id}` - Update rule
- [x] API: `DELETE /api/v1/rules/{id}` - Delete rule
- [x] API: `POST /api/v1/rules/{id}/toggle` - Enable/disable
- [x] API: `POST /api/v1/rules/reorder` - Reorder priorities
- [x] SlateDB persistence: rules CRUD with index-key pattern

### Dioxus UI - Basic
- [x] `nylon-wall-ui/Cargo.toml` - Dioxus 0.7 + router + gloo-net + lucide icons
- [x] `nylon-wall-ui/Dioxus.toml`
- [x] `nylon-wall-ui/src/main.rs` - UI entrypoint
- [x] `nylon-wall-ui/src/app.rs` - Root App + router + sidebar nav (k3rs-style dark theme)
- [x] `nylon-wall-ui/src/api_client.rs` - HTTP client (gloo-net)
- [x] `nylon-wall-ui/src/models.rs` - UI data models
- [x] `nylon-wall-ui/src/components/dashboard.rs` - Dashboard with stat cards + recent rules
- [x] `nylon-wall-ui/src/components/rules.rs` - Rules table + CRUD form + toggle/delete
- [x] `nylon-wall-ui/assets/tailwind.css` - Tailwind CSS v4 dark theme
- [x] `nylon-wall-ui/assets/main.css` - Font imports

---

## Phase 3: NAT & Routing

### eBPF Programs
- [ ] `nylon-wall-ebpf/src/nat.rs` - NAT processing (SNAT/DNAT/Masquerade)
- [ ] eBPF maps: `nat_table` (HashMap)
- [ ] SNAT - rewrite source IP/port on egress
- [ ] DNAT - rewrite dest IP/port on ingress
- [ ] Masquerade - auto SNAT to outgoing interface IP
- [ ] eBPF maps: `route_marks` (HashMap) - policy routing marks

### Daemon - NAT & Route
- [ ] `nylon-wall-daemon/src/nat.rs` - NAT CRUD + compile to eBPF maps
- [ ] `nylon-wall-daemon/src/route.rs` - Route management + kernel route integration
- [x] API: `GET /api/v1/nat` - List NAT entries
- [x] API: `POST /api/v1/nat` - Create NAT entry
- [x] API: `PUT /api/v1/nat/{id}` - Update NAT entry
- [x] API: `DELETE /api/v1/nat/{id}` - Delete NAT entry
- [x] API: `GET /api/v1/routes` - List routes
- [x] API: `POST /api/v1/routes` - Add route
- [x] API: `PUT /api/v1/routes/{id}` - Update route
- [x] API: `DELETE /api/v1/routes/{id}` - Delete route
- [ ] API: `GET /api/v1/routes/policy` - List policy routes
- [ ] API: `POST /api/v1/routes/policy` - Add policy route
- [x] SlateDB persistence: NAT + routes (via generic CRUD)

### Dioxus UI - NAT & Routes
- [x] `nylon-wall-ui/src/components/nat.rs` - NAT table + create form
- [x] `nylon-wall-ui/src/components/routes.rs` - Route table + static route editor
- [ ] Port forward wizard
- [ ] Policy route editor

---

## Phase 4: Network Policy & Zones

### eBPF Programs
- [ ] eBPF maps: `zone_map` (HashMap - ifindex -> zone_id)
- [ ] eBPF maps: `policy_map` (HashMap - zone pair -> policy rules)
- [ ] Zone-based packet evaluation ใน XDP/TC programs

### Daemon - Policy Engine
- [x] API: `GET /api/v1/zones` - List zones
- [x] API: `POST /api/v1/zones` - Create zone
- [x] API: `PUT /api/v1/zones/{id}` - Update zone
- [x] API: `DELETE /api/v1/zones/{id}` - Delete zone
- [x] API: `GET /api/v1/policies` - List policies
- [x] API: `POST /api/v1/policies` - Create policy
- [x] API: `PUT /api/v1/policies/{id}` - Update policy
- [x] API: `DELETE /api/v1/policies/{id}` - Delete policy
- [ ] Schedule-based policy evaluation (time/day matching)
- [x] SlateDB persistence: zones + policies (via generic CRUD)

### Dioxus UI - Policies
- [x] `nylon-wall-ui/src/components/policies.rs` - Zone cards + inter-zone policy table
- [ ] Zone create/edit forms
- [ ] Policy create/edit forms
- [ ] Schedule editor

---

## Phase 5: Monitoring & Polish

### eBPF Programs
- [ ] eBPF maps: `metrics_map` (PerCpuArray - counters)
- [ ] eBPF maps: `rate_limit` (PerCpuArray - token bucket)
- [ ] Perf event logging สำหรับ matched packets

### Daemon - Monitoring
- [ ] `nylon-wall-daemon/src/metrics.rs` - Prometheus metrics endpoint (`/metrics`)
- [ ] Packet log reader (perf event ring buffer -> SlateDB)
- [x] API: `GET /api/v1/conntrack` - List active connections
- [x] API: `GET /api/v1/logs` - Query packet logs (with filters)
- [ ] API: `WS /api/v1/ws/events` - WebSocket real-time event stream
- [ ] Log TTL auto-cleanup via SlateDB TTL

### Dioxus UI - Monitoring
- [x] `nylon-wall-ui/src/components/dashboard.rs` - Dashboard (stat cards, recent rules)
- [ ] Dashboard: live charts, top talkers, blocked IPs
- [x] `nylon-wall-ui/src/components/connections.rs` - Live conntrack table + stats
- [x] `nylon-wall-ui/src/components/logs.rs` - Log viewer with refresh
- [x] Logs: filters (src_ip, dst_ip, protocol, action)

---

## Phase 6: System & Hardening

### Daemon - System
- [x] API: `GET /api/v1/system/interfaces` - List network interfaces
- [x] API: `GET /api/v1/system/status` - Daemon & eBPF program status
- [ ] API: `POST /api/v1/system/apply` - Apply pending configuration
- [x] API: `POST /api/v1/system/backup` - Export full config from SlateDB
- [x] API: `POST /api/v1/system/restore` - Import config to SlateDB
- [ ] Rate limiting / QoS (token bucket in eBPF)
- [ ] IPv6 full support (all eBPF programs + rules)
- [ ] Performance tuning & benchmarking

### Dioxus UI - Settings
- [x] `nylon-wall-ui/src/components/settings.rs` - System info + backup/restore buttons
- [ ] Interface configuration UI
- [ ] Daemon settings editor

### Extras (Optional)
- [ ] DHCP server สำหรับ LAN interfaces
- [ ] DNS filtering (blocklist + custom responses + query logging)
