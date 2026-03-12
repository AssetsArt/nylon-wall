# Nylon Wall - Implementation Checklist

## Phase 1: Foundation

### Workspace & Config
- [ ] `Cargo.toml` - Workspace root (members: common, ebpf, daemon, ui)
- [ ] `/etc/nylon-wall/config.toml` - Default config template

### nylon-wall-common
- [ ] `nylon-wall-common/Cargo.toml`
- [ ] `nylon-wall-common/src/lib.rs` - Re-exports
- [ ] `nylon-wall-common/src/rule.rs` - `FirewallRule`, `Direction`, `Action`, `PortRange`
- [ ] `nylon-wall-common/src/nat.rs` - `NatEntry`, `NatType`
- [ ] `nylon-wall-common/src/route.rs` - `Route`, `PolicyRoute`
- [ ] `nylon-wall-common/src/zone.rs` - `Zone`, `NetworkPolicy`, `Schedule`
- [ ] `nylon-wall-common/src/conntrack.rs` - `ConntrackEntry`, `ConnState`
- [ ] `nylon-wall-common/src/log.rs` - `PacketLog`, `MetricPoint`
- [ ] `nylon-wall-common/src/protocol.rs` - `Protocol` enum, shared constants

### nylon-wall-ebpf
- [ ] `nylon-wall-ebpf/Cargo.toml`
- [ ] `nylon-wall-ebpf/src/main.rs` - eBPF entrypoint
- [ ] `nylon-wall-ebpf/src/common.rs` - Shared eBPF structs (repr(C))
- [ ] Basic XDP program - pass all traffic
- [ ] ทดสอบ load/attach บน test interface

### nylon-wall-daemon
- [ ] `nylon-wall-daemon/Cargo.toml`
- [ ] `nylon-wall-daemon/src/main.rs` - Daemon entrypoint + tokio runtime
- [ ] `nylon-wall-daemon/src/ebpf_loader.rs` - Load & attach eBPF programs
- [ ] `nylon-wall-daemon/src/db.rs` - SlateDB init + helpers (open, get, put, scan, delete)
- [ ] `nylon-wall-daemon/src/api.rs` - axum router skeleton
- [ ] ทดสอบ packet drop/allow บน test interface

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
- [ ] `nylon-wall-daemon/src/rule_engine.rs` - Rule CRUD + compile to eBPF maps
- [ ] `nylon-wall-daemon/src/state.rs` - Conntrack reader from eBPF maps
- [ ] API: `GET /api/v1/rules` - List rules
- [ ] API: `POST /api/v1/rules` - Create rule
- [ ] API: `GET /api/v1/rules/{id}` - Get rule
- [ ] API: `PUT /api/v1/rules/{id}` - Update rule
- [ ] API: `DELETE /api/v1/rules/{id}` - Delete rule
- [ ] API: `POST /api/v1/rules/{id}/toggle` - Enable/disable
- [ ] API: `POST /api/v1/rules/reorder` - Reorder priorities
- [ ] SlateDB persistence: rules CRUD with prefix scan

### Dioxus UI - Basic
- [ ] `nylon-wall-ui/Cargo.toml`
- [ ] `nylon-wall-ui/Dioxus.toml`
- [ ] `nylon-wall-ui/src/main.rs` - UI entrypoint
- [ ] `nylon-wall-ui/src/app.rs` - Root App + router + sidebar nav
- [ ] `nylon-wall-ui/src/api_client.rs` - HTTP client (reqwest)
- [ ] `nylon-wall-ui/src/models.rs` - UI data models
- [ ] `nylon-wall-ui/src/components/dashboard.rs` - Basic dashboard (stats only)
- [ ] `nylon-wall-ui/src/components/rules.rs` - Rules table + CRUD form

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
- [ ] API: `GET /api/v1/nat` - List NAT entries
- [ ] API: `POST /api/v1/nat` - Create NAT entry
- [ ] API: `PUT /api/v1/nat/{id}` - Update NAT entry
- [ ] API: `DELETE /api/v1/nat/{id}` - Delete NAT entry
- [ ] API: `GET /api/v1/routes` - List routes
- [ ] API: `POST /api/v1/routes` - Add route
- [ ] API: `PUT /api/v1/routes/{id}` - Update route
- [ ] API: `DELETE /api/v1/routes/{id}` - Delete route
- [ ] API: `GET /api/v1/routes/policy` - List policy routes
- [ ] API: `POST /api/v1/routes/policy` - Add policy route
- [ ] SlateDB persistence: NAT + routes

### Dioxus UI - NAT & Routes
- [ ] `nylon-wall-ui/src/components/nat.rs` - NAT table + SNAT/DNAT form + port forward wizard
- [ ] `nylon-wall-ui/src/components/routes.rs` - Route table + static/policy route editor

---

## Phase 4: Network Policy & Zones

### eBPF Programs
- [ ] eBPF maps: `zone_map` (HashMap - ifindex -> zone_id)
- [ ] eBPF maps: `policy_map` (HashMap - zone pair -> policy rules)
- [ ] Zone-based packet evaluation ใน XDP/TC programs

### Daemon - Policy Engine
- [ ] API: `GET /api/v1/zones` - List zones
- [ ] API: `POST /api/v1/zones` - Create zone
- [ ] API: `PUT /api/v1/zones/{id}` - Update zone
- [ ] API: `DELETE /api/v1/zones/{id}` - Delete zone
- [ ] API: `GET /api/v1/policies` - List policies
- [ ] API: `POST /api/v1/policies` - Create policy
- [ ] API: `PUT /api/v1/policies/{id}` - Update policy
- [ ] API: `DELETE /api/v1/policies/{id}` - Delete policy
- [ ] Schedule-based policy evaluation (time/day matching)
- [ ] SlateDB persistence: zones + policies

### Dioxus UI - Policies
- [ ] `nylon-wall-ui/src/components/policies.rs` - Zone manager + inter-zone matrix + policy editor

---

## Phase 5: Monitoring & Polish

### eBPF Programs
- [ ] eBPF maps: `metrics_map` (PerCpuArray - counters)
- [ ] eBPF maps: `rate_limit` (PerCpuArray - token bucket)
- [ ] Perf event logging สำหรับ matched packets

### Daemon - Monitoring
- [ ] `nylon-wall-daemon/src/metrics.rs` - Prometheus metrics endpoint (`/metrics`)
- [ ] Packet log reader (perf event ring buffer -> SlateDB)
- [ ] API: `GET /api/v1/conntrack` - List active connections
- [ ] API: `GET /api/v1/logs` - Query packet logs (with filters)
- [ ] API: `WS /api/v1/ws/events` - WebSocket real-time event stream
- [ ] Log TTL auto-cleanup via SlateDB TTL

### Dioxus UI - Monitoring
- [ ] `nylon-wall-ui/src/components/dashboard.rs` - Full dashboard (live charts, top talkers, blocked IPs)
- [ ] `nylon-wall-ui/src/components/connections.rs` - Live conntrack table + kill action
- [ ] `nylon-wall-ui/src/components/logs.rs` - Log viewer + filters + real-time stream + CSV export

---

## Phase 6: System & Hardening

### Daemon - System
- [ ] API: `GET /api/v1/system/interfaces` - List network interfaces
- [ ] API: `GET /api/v1/system/status` - Daemon & eBPF program status
- [ ] API: `POST /api/v1/system/apply` - Apply pending configuration
- [ ] API: `POST /api/v1/system/backup` - Export full config from SlateDB
- [ ] API: `POST /api/v1/system/restore` - Import config to SlateDB
- [ ] Rate limiting / QoS (token bucket in eBPF)
- [ ] IPv6 full support (all eBPF programs + rules)
- [ ] Performance tuning & benchmarking

### Dioxus UI - Settings
- [ ] `nylon-wall-ui/src/components/settings.rs` - Interface config + daemon settings + backup/restore

### Extras (Optional)
- [ ] DHCP server สำหรับ LAN interfaces
- [ ] DNS filtering (blocklist + custom responses + query logging)
- [ ] Documentation
