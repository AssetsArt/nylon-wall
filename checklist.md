# Nylon Wall - Implementation Checklist

## Phase 1: Foundation

### Workspace & Config
- [x] `Cargo.toml` - Workspace root (members: common, daemon, ui; exclude: ebpf)
- [x] `/etc/nylon-wall/config.toml` - Default config template

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
- [x] `nylon-wall-ebpf/src/ingress.rs` - XDP ingress filter + rule evaluation + NAT + zone + rate limiting
- [x] `nylon-wall-ebpf/src/egress.rs` - TC egress filter + rule evaluation + NAT + zone + rate limiting
- [x] eBPF maps: `ingress_rules`, `egress_rules` (Array)
- [x] eBPF maps: `conntrack` (LRU HashMap)
- [x] eBPF maps: `events` (PerfEventArray)
- [x] Connection tracking logic ใน eBPF (NEW/ESTABLISHED/RELATED/INVALID)
- [ ] ทดสอบ load/attach + packet filtering บน Linux

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
- [x] `nylon-wall-ebpf/src/nat.rs` - NAT processing (SNAT/DNAT/Masquerade)
- [x] eBPF maps: `nat_table` (Array)
- [x] SNAT - rewrite source IP/port on egress
- [x] DNAT - rewrite dest IP/port on ingress
- [x] Masquerade - auto SNAT to outgoing interface IP
- [x] eBPF maps: `nat_conntrack` (LRU HashMap) - NAT state for return traffic
- [x] eBPF maps: `masquerade_ip` (Array) - outgoing interface IP

### Daemon - NAT & Route
- [x] `nylon-wall-daemon/src/nat.rs` - NAT CRUD + compile to eBPF maps
- [x] `nylon-wall-daemon/src/route.rs` - Route management + kernel route integration
- [x] API: `GET /api/v1/nat` - List NAT entries
- [x] API: `POST /api/v1/nat` - Create NAT entry
- [x] API: `PUT /api/v1/nat/{id}` - Update NAT entry
- [x] API: `DELETE /api/v1/nat/{id}` - Delete NAT entry
- [x] API: `GET /api/v1/routes` - List routes
- [x] API: `POST /api/v1/routes` - Add route
- [x] API: `PUT /api/v1/routes/{id}` - Update route
- [x] API: `DELETE /api/v1/routes/{id}` - Delete route
- [x] API: `GET /api/v1/routes/policy` - List policy routes
- [x] API: `POST /api/v1/routes/policy` - Add policy route
- [x] SlateDB persistence: NAT + routes (via generic CRUD)

### Dioxus UI - NAT & Routes
- [x] `nylon-wall-ui/src/components/nat.rs` - NAT table + create form
- [x] `nylon-wall-ui/src/components/routes.rs` - Route table + static route editor
- [x] Port forward wizard
- [x] Policy route editor

---

## Phase 4: Network Policy & Zones

### eBPF Programs
- [x] eBPF maps: `zone_map` (HashMap - ifindex -> zone_id)
- [x] eBPF maps: `policy_map` (HashMap - zone pair -> policy rules)
- [x] Zone-based packet evaluation ใน XDP/TC programs

### Daemon - Policy Engine
- [x] API: `GET /api/v1/zones` - List zones
- [x] API: `POST /api/v1/zones` - Create zone
- [x] API: `PUT /api/v1/zones/{id}` - Update zone
- [x] API: `DELETE /api/v1/zones/{id}` - Delete zone
- [x] API: `GET /api/v1/policies` - List policies
- [x] API: `POST /api/v1/policies` - Create policy
- [x] API: `PUT /api/v1/policies/{id}` - Update policy
- [x] API: `DELETE /api/v1/policies/{id}` - Delete policy
- [x] Schedule-based policy evaluation (time/day matching)
- [x] SlateDB persistence: zones + policies (via generic CRUD)

### Dioxus UI - Policies
- [x] `nylon-wall-ui/src/components/policies.rs` - Zone cards + inter-zone policy table
- [x] Zone create/edit forms
- [x] Policy create/edit forms
- [x] Schedule editor

---

## Phase 5: Monitoring & Polish

### eBPF Programs
- [x] eBPF maps: `metrics` (Array - global counters)
- [x] eBPF maps: `rate_limit` (HashMap - per-rule token bucket)
- [x] Perf event logging สำหรับ matched packets

### Daemon - Monitoring
- [x] `nylon-wall-daemon/src/metrics.rs` - Prometheus metrics endpoint (`/metrics`)
- [x] Packet log reader (perf event ring buffer -> SlateDB)
- [x] API: `GET /api/v1/conntrack` - List active connections
- [x] API: `GET /api/v1/logs` - Query packet logs (with filters)
- [x] API: `WS /api/v1/ws/events` - WebSocket real-time event stream
- [x] Log TTL auto-cleanup (background task, 7-day TTL, hourly sweep)

### Dioxus UI - Monitoring
- [x] `nylon-wall-ui/src/components/dashboard.rs` - Dashboard (stat cards, recent rules)
- [x] Dashboard: recent logs, top blocked IPs
- [x] `nylon-wall-ui/src/components/connections.rs` - Live conntrack table + stats
- [x] `nylon-wall-ui/src/components/logs.rs` - Log viewer with refresh
- [x] Logs: filters (src_ip, dst_ip, protocol, action)

---

## Phase 6: System & Hardening

### Daemon - System
- [x] API: `GET /api/v1/system/interfaces` - List network interfaces
- [x] API: `GET /api/v1/system/status` - Daemon & eBPF program status
- [x] API: `POST /api/v1/system/apply` - Apply pending configuration
- [x] API: `POST /api/v1/system/backup` - Export full config from SlateDB
- [x] API: `POST /api/v1/system/restore` - Import config to SlateDB
- [x] Rate limiting / QoS (token bucket in eBPF)
- [ ] IPv6 full support (all eBPF programs + rules)
- [ ] Performance tuning & benchmarking

### Dioxus UI - Settings
- [x] `nylon-wall-ui/src/components/settings.rs` - System info + backup/restore buttons
- [x] Interface configuration UI
- [x] Daemon settings editor

---

## Phase 7: DHCP Server & Client

### nylon-wall-common - DHCP Types
- [x] `nylon-wall-common/src/dhcp.rs` - `DhcpPool`, `DhcpLease`, `DhcpLeaseState`, `DhcpReservation`
- [x] `nylon-wall-common/src/dhcp.rs` - `DhcpClientConfig`, `DhcpClientStatus`, `DhcpClientState`
- [x] `nylon-wall-common/src/lib.rs` - Add `pub mod dhcp`

### nylon-wall-daemon - DHCP Module
- [x] `nylon-wall-daemon/Cargo.toml` - Add `dhcproto`, `socket2`, `rand`
- [x] `nylon-wall-daemon/src/dhcp/mod.rs` - Module declarations + shared helpers
- [x] `nylon-wall-daemon/src/dhcp/packet.rs` - dhcproto wrapper (build/parse DHCP messages)
- [x] `nylon-wall-daemon/src/dhcp/socket.rs` - Raw UDP socket creation (Linux-only, SO_BINDTODEVICE)
- [x] `nylon-wall-daemon/src/dhcp/lease_manager.rs` - IP allocation, renewal, expiration logic
- [x] `nylon-wall-daemon/src/dhcp/server.rs` - DHCP server background task (per-interface)
- [x] `nylon-wall-daemon/src/dhcp/client.rs` - DHCP client state machine (per-WAN interface)
- [x] `nylon-wall-daemon/src/main.rs` - AppState fields (`dhcp_pool_notify`, `dhcp_client_statuses`) + spawn tasks

### Daemon - DHCP API
- [x] API: `GET /api/v1/dhcp/pools` - List DHCP pools
- [x] API: `POST /api/v1/dhcp/pools` - Create DHCP pool
- [x] API: `GET /api/v1/dhcp/pools/{id}` - Get DHCP pool
- [x] API: `PUT /api/v1/dhcp/pools/{id}` - Update DHCP pool
- [x] API: `DELETE /api/v1/dhcp/pools/{id}` - Delete DHCP pool
- [x] API: `POST /api/v1/dhcp/pools/{id}/toggle` - Enable/disable pool
- [x] API: `GET /api/v1/dhcp/leases` - List active leases
- [x] API: `DELETE /api/v1/dhcp/leases/{mac}` - Release lease
- [x] API: `POST /api/v1/dhcp/leases/{mac}/reserve` - Create reservation from lease
- [x] API: `GET /api/v1/dhcp/reservations` - List reservations
- [x] API: `POST /api/v1/dhcp/reservations` - Create reservation
- [x] API: `PUT /api/v1/dhcp/reservations/{id}` - Update reservation
- [x] API: `DELETE /api/v1/dhcp/reservations/{id}` - Delete reservation
- [x] API: `GET /api/v1/dhcp/clients` - List WAN DHCP clients
- [x] API: `POST /api/v1/dhcp/clients` - Create WAN client
- [x] API: `PUT /api/v1/dhcp/clients/{id}` - Update WAN client
- [x] API: `DELETE /api/v1/dhcp/clients/{id}` - Delete WAN client
- [x] API: `POST /api/v1/dhcp/clients/{id}/toggle` - Enable/disable client
- [x] API: `GET /api/v1/dhcp/clients/status` - Get all client statuses
- [x] API: `POST /api/v1/dhcp/clients/{interface}/release` - Release WAN lease
- [x] API: `POST /api/v1/dhcp/clients/{interface}/renew` - Renew WAN lease

### Dioxus UI - DHCP
- [x] `nylon-wall-ui/src/components/dhcp.rs` - DHCP page with 3-tab layout
- [x] Tab: Server Pools - Pool table + create/edit form + toggle/delete
- [x] Tab: Leases - Lease table (release/reserve) + Static reservations table + form
- [x] Tab: WAN Client - Client cards with live status + enable/disable/renew/release
- [x] `nylon-wall-ui/src/components/mod.rs` - Export `Dhcp` component
- [x] `nylon-wall-ui/src/app.rs` - Add `/dhcp` route + sidebar nav link

### Integration
- [x] `nylon-wall-daemon/src/metrics.rs` - DHCP Prometheus metrics (pools, leases, clients)
- [x] `nylon-wall-ui/src/components/dashboard.rs` - DHCP summary card (active leases + pools)
- [x] `nylon-wall-daemon/src/api.rs` - Backup/restore includes DHCP pools, reservations, clients
- [x] `docker-compose.yml` - Add `NET_RAW` capability
- [ ] ทดสอบ DHCP server assign IP ให้ LAN client
- [ ] ทดสอบ DHCP client ได้ IP จาก ISP

---

## Phase 8: Virtual Networking (VLAN + Bridge)

### nylon-wall-common - Virtual Network Types
- [ ] `nylon-wall-common/src/vnet.rs` - `VlanConfig`, `BridgeConfig` structs
  - `VlanConfig`: id, parent_interface, vlan_id (1-4094), ip_address (optional CIDR), enabled
  - `BridgeConfig`: id, name, ports (Vec<String> — interfaces/VLANs to attach), ip_address (optional CIDR), stp_enabled, enabled
- [ ] `nylon-wall-common/src/lib.rs` - Add `pub mod vnet`

### nylon-wall-daemon - VLAN Module
- [ ] `nylon-wall-daemon/src/vnet/mod.rs` - Module declarations
- [ ] `nylon-wall-daemon/src/vnet/vlan.rs` - Create/delete VLAN sub-interfaces
  - Create: `ip link add link {parent} name {parent}.{vlan_id} type vlan id {vlan_id}`
  - Delete: `ip link delete {parent}.{vlan_id}`
  - IP assign: `ip addr add {cidr} dev {parent}.{vlan_id}`
  - Bring up: `ip link set {parent}.{vlan_id} up`
- [ ] `nylon-wall-daemon/src/vnet/bridge.rs` - Create/delete Linux bridges
  - Create: `ip link add name {name} type bridge`
  - Add port: `ip link set {port} master {bridge}`
  - Remove port: `ip link set {port} nomaster`
  - Delete: `ip link delete {bridge}`
  - STP: `ip link set {bridge} type bridge stp_state {0|1}`
  - IP assign: `ip addr add {cidr} dev {bridge}`
  - Bring up: `ip link set {bridge} up`
- [ ] Persist configs in SlateDB (recreate VLANs + bridges on daemon restart)
- [ ] Startup order: create VLANs first, then bridges (bridges may reference VLAN interfaces)

### Daemon - VLAN API
- [ ] API: `GET /api/v1/vlans` - List VLAN interfaces
- [ ] API: `POST /api/v1/vlans` - Create VLAN sub-interface
- [ ] API: `PUT /api/v1/vlans/{id}` - Update VLAN (IP config)
- [ ] API: `DELETE /api/v1/vlans/{id}` - Delete VLAN sub-interface
- [ ] Validation: prevent duplicate VLAN ID on same parent interface
- [ ] Validation: parent interface must exist

### Daemon - Bridge API
- [ ] API: `GET /api/v1/bridges` - List bridges
- [ ] API: `POST /api/v1/bridges` - Create bridge
- [ ] API: `PUT /api/v1/bridges/{id}` - Update bridge (ports, IP, STP)
- [ ] API: `DELETE /api/v1/bridges/{id}` - Delete bridge
- [ ] API: `POST /api/v1/bridges/{id}/ports` - Add port to bridge
- [ ] API: `DELETE /api/v1/bridges/{id}/ports/{interface}` - Remove port from bridge
- [ ] Validation: port interface must exist (physical, VLAN, or other virtual)
- [ ] Validation: port not already in another bridge

### Dioxus UI - Virtual Networking
- [ ] `nylon-wall-ui/src/components/vnet.rs` - Virtual Networking page with 2-tab layout
- [ ] Tab: VLANs - VLAN table + create/edit form
  - Form fields: Parent Interface (select), VLAN ID (1-4094), IP Address (CIDR, optional)
- [ ] Tab: Bridges - Bridge cards/table + create/edit form
  - Form fields: Bridge Name, IP Address (CIDR, optional), STP toggle
  - Port management: multi-select of available interfaces (physical + VLANs)
  - Visual: show attached ports as tags/chips on bridge card
- [ ] Show VLANs + bridges in interface selects across all pages (rules, NAT, DHCP, routes)
- [ ] `nylon-wall-ui/src/components/mod.rs` - Export `Vnet` component
- [ ] `nylon-wall-ui/src/app.rs` - Add `/vnet` route + sidebar nav link (icon: LdNetwork)

### Integration
- [ ] Backup/restore includes VLAN + bridge configs
- [ ] Dashboard: VLAN + bridge count in system status
- [ ] DHCP pool can use VLAN/bridge interface (e.g. `eth0.10`, `br-lan`)
- [ ] Firewall rules can target VLAN/bridge interface
- [ ] Delete protection: warn if VLAN/bridge is used by rules, NAT, DHCP, routes
- [ ] ทดสอบ VLAN creation + bridge + DHCP pool on bridge interface

---

### Extras (Optional)
- [ ] DNS filtering (blocklist + custom responses + query logging)
