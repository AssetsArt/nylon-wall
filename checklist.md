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
- [x] `nylon-wall-ebpf/src/main.rs` - eBPF entrypoint + tail call dispatch (XDP + TC)
- [x] `nylon-wall-ebpf/src/common.rs` - Shared eBPF constants + packet parser
- [x] `nylon-wall-ebpf/src/scratchpad.rs` - PerCpuArray ScratchPad helpers (write/read)
- [x] `nylon-wall-ebpf/src/stages/` - Tail call stages (NAT → SNI → Rules per direction)
  - `ingress_nat.rs`, `ingress_sni.rs`, `ingress_rules.rs` (XDP)
  - `egress_nat.rs`, `egress_sni.rs`, `egress_rules.rs` (TC)
- [x] `nylon-wall-common/src/scratchpad.rs` - ScratchPad `#[repr(C)]` struct + stage constants
- [x] `nylon-wall-daemon/src/ebpf_loader.rs` - Load entry + tail programs, register ProgramArray
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

### eBPF - VLAN-aware packet parsing
- [ ] `nylon-wall-ebpf/src/common.rs` - Add `ETH_P_8021Q` constant (`0x8100`)
- [ ] `nylon-wall-ebpf/src/common.rs` - Add `ETH_P_8021AD` constant (`0x88A8`, QinQ double VLAN)
- [ ] `nylon-wall-ebpf/src/common.rs` - Update `parse_packet()` to handle VLAN tags:
  - If EtherType == `0x8100` or `0x88A8`: skip 4-byte VLAN tag, read real EtherType
  - Support stacked VLANs (QinQ): skip up to 2 VLAN tags
  - Shift `ip_base` offset accordingly (+4 per VLAN tag)
  - Extract VLAN ID from tag for `PacketInfo`
- [ ] `nylon-wall-ebpf/src/common.rs` - Add `vlan_id: u16` field to `PacketInfo` struct
- [ ] `nylon-wall-ebpf/src/ingress.rs` - VLAN ID available for zone/rule matching
- [ ] `nylon-wall-ebpf/src/egress.rs` - Same VLAN-aware parsing
- [ ] `nylon-wall-ebpf/src/nat.rs` - Same VLAN-aware parsing (NAT header rewrite at correct offset)
- [ ] Optional: eBPF rule matching by VLAN ID (add `vlan_id` field to `EbpfRule`)

### Integration
- [ ] Backup/restore includes VLAN + bridge configs
- [ ] Dashboard: VLAN + bridge count in system status
- [ ] DHCP pool can use VLAN/bridge interface (e.g. `eth0.10`, `br-lan`)
- [ ] Firewall rules can target VLAN/bridge interface
- [ ] Firewall rules can filter by VLAN ID (optional)
- [ ] Delete protection: warn if VLAN/bridge is used by rules, NAT, DHCP, routes
- [ ] ทดสอบ VLAN creation + bridge + DHCP pool on bridge interface
- [ ] ทดสอบ eBPF parse VLAN-tagged packets ถูกต้อง
- [ ] ทดสอบ eBPF filter by VLAN ID

---

## Phase 9: UI Authentication

### Daemon - Auth
- [ ] `nylon-wall-daemon/src/auth.rs` - Session management (bcrypt password hash, JWT tokens)
- [ ] SlateDB: store admin password hash (`auth:admin_password`)
- [ ] API: `POST /api/v1/auth/login` - Login (returns JWT)
- [ ] API: `POST /api/v1/auth/logout` - Invalidate session
- [ ] API: `PUT /api/v1/auth/password` - Change password
- [ ] API: `GET /api/v1/auth/check` - Verify token validity
- [ ] axum middleware: JWT validation on all `/api/v1/*` routes (except login)
- [ ] First-run setup: if no password set, force setup on first access

### Dioxus UI - Auth
- [ ] `nylon-wall-ui/src/components/login.rs` - Login page (username + password)
- [ ] `nylon-wall-ui/src/components/setup.rs` - First-run password setup page
- [ ] JWT token storage in localStorage
- [ ] `api_client.rs` - Attach `Authorization: Bearer` header to all requests
- [ ] Auto-redirect to login on 401 response
- [ ] Session timeout handling (auto-logout)
- [ ] Settings page: change password form

---

## Phase 10: Traffic Monitoring & Graphs

### eBPF - Per-interface traffic counters
- [ ] eBPF map: `iface_stats` (HashMap - ifindex → {rx_bytes, tx_bytes, rx_packets, tx_packets})
- [ ] `ingress.rs` - Increment per-interface rx counters
- [ ] `egress.rs` - Increment per-interface tx counters
- [ ] eBPF map: `ip_stats` (LRU HashMap - src_ip → {bytes_in, bytes_out, packets})
- [ ] Per-IP bandwidth tracking in XDP/TC

### Daemon - Traffic API
- [ ] `nylon-wall-daemon/src/traffic.rs` - Periodic read from eBPF maps → store time-series in SlateDB
- [ ] Background task: sample counters every 10 seconds, store 5-min/1-hour/1-day aggregates
- [ ] API: `GET /api/v1/traffic/interfaces` - Per-interface bandwidth (current + history)
- [ ] API: `GET /api/v1/traffic/top` - Top talkers (IPs by bandwidth)
- [ ] API: `GET /api/v1/traffic/history?interface={iface}&period={5m|1h|1d}` - Time-series data
- [ ] Auto-cleanup: keep 5-min data for 24h, 1-hour data for 30 days, 1-day data for 1 year

### Dioxus UI - Traffic
- [ ] `nylon-wall-ui/src/components/traffic.rs` - Traffic monitoring page
- [ ] Real-time bandwidth chart per interface (sparkline or area chart)
- [ ] Top talkers table (IP, total bytes, current rate)
- [ ] Period selector (live, 1h, 24h, 7d, 30d)
- [ ] Dashboard: mini bandwidth chart per interface card
- [ ] `nylon-wall-ui/src/app.rs` - Add `/traffic` route + sidebar nav link (icon: LdBarChart3)

---

## Phase 11: DNS Filtering

### eBPF - DNS interception
- [ ] eBPF map: `dns_blocklist` (HashMap - domain_hash → action)
- [ ] `ingress.rs` - Detect DNS queries (UDP port 53), extract domain name
- [ ] eBPF: compute domain hash, lookup in blocklist map
- [ ] eBPF: for blocked domains → rewrite DNS response with NXDOMAIN or custom IP (redirect)
- [ ] eBPF map: `dns_query_log` (PerfEventArray - domain, src_ip, action, timestamp)

### Daemon - DNS Module
- [ ] `nylon-wall-daemon/src/dns/mod.rs` - Module declarations
- [ ] `nylon-wall-daemon/src/dns/blocklist.rs` - Load blocklists (AdGuard, Steven Black, custom)
- [ ] Blocklist sources: download URL → parse domains → hash → push to eBPF map
- [ ] Background task: auto-update blocklists (configurable interval, default 24h)
- [ ] `nylon-wall-daemon/src/dns/logger.rs` - Read DNS query perf events → store in SlateDB
- [ ] API: `GET /api/v1/dns/blocklists` - List configured blocklists
- [ ] API: `POST /api/v1/dns/blocklists` - Add blocklist source (URL or custom)
- [ ] API: `DELETE /api/v1/dns/blocklists/{id}` - Remove blocklist
- [ ] API: `POST /api/v1/dns/blocklists/update` - Force re-download all blocklists
- [ ] API: `GET /api/v1/dns/whitelist` - Custom whitelist (always allow)
- [ ] API: `POST /api/v1/dns/whitelist` - Add domain to whitelist
- [ ] API: `DELETE /api/v1/dns/whitelist/{id}` - Remove from whitelist
- [ ] API: `GET /api/v1/dns/queries` - Query log (with filters: domain, src_ip, blocked/allowed)
- [ ] API: `GET /api/v1/dns/stats` - Block stats (total queries, blocked count, top blocked domains)

### Dioxus UI - DNS
- [ ] `nylon-wall-ui/src/components/dns.rs` - DNS filtering page with tabs
- [ ] Tab: Dashboard - block rate %, top blocked domains, queries over time chart
- [ ] Tab: Blocklists - list of sources + toggle enable/disable + add custom
- [ ] Tab: Whitelist - custom allow list management
- [ ] Tab: Query Log - searchable DNS query table (domain, client IP, blocked/allowed, timestamp)
- [ ] `nylon-wall-ui/src/app.rs` - Add `/dns` route + sidebar nav link (icon: LdShield)

---

## Phase 12: Dynamic DNS (DDNS)

### Daemon - DDNS
- [ ] `nylon-wall-daemon/src/ddns.rs` - DDNS updater
- [ ] Support providers: Cloudflare, No-IP, DuckDNS, Dynu, custom URL
- [ ] Background task: detect WAN IP change → update DNS record
- [ ] SlateDB: store DDNS configs (`ddns:{id}`)
- [ ] API: `GET /api/v1/ddns` - List DDNS configs
- [ ] API: `POST /api/v1/ddns` - Create DDNS config
- [ ] API: `PUT /api/v1/ddns/{id}` - Update DDNS config
- [ ] API: `DELETE /api/v1/ddns/{id}` - Delete DDNS config
- [ ] API: `POST /api/v1/ddns/{id}/update` - Force update now
- [ ] API: `GET /api/v1/ddns/{id}/status` - Last update status + current WAN IP

### Dioxus UI - DDNS
- [ ] `nylon-wall-ui/src/components/ddns.rs` - DDNS config page
- [ ] Provider selector + credentials form (per provider)
- [ ] Status display: current WAN IP, last update time, success/error
- [ ] Force update button
- [ ] Settings page integration or standalone `/ddns` route

---

## Phase 13: Multi-WAN Failover

### eBPF - Multi-WAN
- [ ] eBPF map: `wan_state` (Array - ifindex → {active, weight, health})
- [ ] eBPF: policy-based WAN selection (fwmark per WAN)
- [ ] eBPF: load balancing support (weighted round-robin via packet hash)

### Daemon - Multi-WAN
- [ ] `nylon-wall-daemon/src/multiwan.rs` - WAN health check + failover logic
- [ ] Health check methods: ping (ICMP), HTTP probe, DNS resolve
- [ ] Configurable check interval, fail threshold, recovery threshold
- [ ] On failover: update default route, update masquerade IP, notify eBPF
- [ ] Failover modes: active-passive, active-active (load balance)
- [ ] API: `GET /api/v1/wan/status` - WAN health status per interface
- [ ] API: `POST /api/v1/wan` - Configure WAN interfaces + failover policy
- [ ] API: `PUT /api/v1/wan/{id}` - Update WAN config
- [ ] API: `DELETE /api/v1/wan/{id}` - Remove WAN interface

### Dioxus UI - Multi-WAN
- [ ] `nylon-wall-ui/src/components/wan.rs` - Multi-WAN config page
- [ ] WAN interface cards with live health status (latency, packet loss)
- [ ] Failover mode selector (active-passive / load balance)
- [ ] Health check config form (method, interval, thresholds)
- [ ] Dashboard: WAN health indicator per interface
- [ ] `nylon-wall-ui/src/app.rs` - Add `/wan` route + sidebar nav link

---

## Phase 14: WireGuard VPN

### Daemon - WireGuard
- [ ] `nylon-wall-daemon/src/wireguard.rs` - WireGuard management via `wg` CLI / netlink
- [ ] Create WireGuard interface: `ip link add wg0 type wireguard`
- [ ] Key generation: `wg genkey`, `wg pubkey`
- [ ] Apply config: `wg set wg0 listen-port {port} private-key {key}`
- [ ] Peer management: `wg set wg0 peer {pubkey} allowed-ips {cidr} endpoint {addr}`
- [ ] API: `GET /api/v1/vpn/wireguard` - Get WireGuard server config
- [ ] API: `PUT /api/v1/vpn/wireguard` - Update server config (port, address, DNS)
- [ ] API: `GET /api/v1/vpn/wireguard/peers` - List peers
- [ ] API: `POST /api/v1/vpn/wireguard/peers` - Add peer (auto-generate keys)
- [ ] API: `DELETE /api/v1/vpn/wireguard/peers/{id}` - Remove peer
- [ ] API: `GET /api/v1/vpn/wireguard/peers/{id}/config` - Download peer config file
- [ ] Auto-create firewall rules for VPN traffic (UDP listen port + wg0 interface)
- [ ] Auto-create NAT masquerade for VPN → LAN access

### Dioxus UI - WireGuard
- [ ] `nylon-wall-ui/src/components/wireguard.rs` - WireGuard VPN page
- [ ] Server config form (listen port, address range, DNS)
- [ ] Peer table with QR code generation (for mobile clients)
- [ ] Peer config download button (.conf file)
- [ ] Live peer status: last handshake, transfer, endpoint
- [ ] `nylon-wall-ui/src/app.rs` - Add `/vpn` route + sidebar nav link (icon: LdShieldCheck)

---

## Phase 15: IDS/IPS (Intrusion Detection & Prevention)

### eBPF - Threat detection
- [ ] eBPF map: `threat_tracker` (LRU HashMap - src_ip → {syn_count, port_scan_count, timestamps})
- [ ] eBPF map: `ip_blacklist` (HashMap - ip → expiry_time) — auto-block list
- [ ] `ingress.rs` - Port scan detection: track unique dst_ports per src_ip within time window
- [ ] `ingress.rs` - SYN flood detection: track SYN rate per src_ip
- [ ] `ingress.rs` - Brute force detection: track connection attempts per src_ip to specific ports (22, 3389)
- [ ] `ingress.rs` - Check `ip_blacklist` before rule evaluation → instant drop
- [ ] eBPF map: `ids_events` (PerfEventArray - threat type, src_ip, details)

### Daemon - IDS/IPS
- [ ] `nylon-wall-daemon/src/ids/mod.rs` - IDS engine
- [ ] `nylon-wall-daemon/src/ids/detector.rs` - Read threat events → evaluate → auto-block
- [ ] Configurable thresholds: port scan (N ports in M seconds), SYN flood (N SYN/sec), brute force (N attempts)
- [ ] Auto-block: add to `ip_blacklist` eBPF map with TTL (configurable, default 1 hour)
- [ ] Manual block/unblock via API
- [ ] API: `GET /api/v1/ids/threats` - List detected threats
- [ ] API: `GET /api/v1/ids/blocked` - List currently blocked IPs
- [ ] API: `POST /api/v1/ids/blocked` - Manually block IP
- [ ] API: `DELETE /api/v1/ids/blocked/{ip}` - Unblock IP
- [ ] API: `GET /api/v1/ids/config` - Get IDS thresholds
- [ ] API: `PUT /api/v1/ids/config` - Update IDS thresholds
- [ ] API: `POST /api/v1/ids/toggle` - Enable/disable IDS

### Dioxus UI - IDS/IPS
- [ ] `nylon-wall-ui/src/components/ids.rs` - IDS/IPS page
- [ ] Threat dashboard: recent threats, blocked IPs count, attack types breakdown
- [ ] Blocked IP table with unblock button + TTL countdown
- [ ] Threshold config form (port scan, SYN flood, brute force)
- [ ] Enable/disable toggle
- [ ] Dashboard: threat count + blocked IPs indicator
- [ ] `nylon-wall-ui/src/app.rs` - Add `/ids` route + sidebar nav link (icon: LdRadar)

---

## Phase 16: GeoIP Blocking

### eBPF - GeoIP
- [ ] eBPF map: `geoip_db` (LPM Trie - IP prefix → country_code)
- [ ] eBPF map: `country_policy` (Array - country_code → action: allow/block)
- [ ] `ingress.rs` - Lookup src_ip in GeoIP LPM trie → check country policy
- [ ] `egress.rs` - Lookup dst_ip in GeoIP LPM trie → check country policy

### Daemon - GeoIP
- [ ] `nylon-wall-daemon/src/geoip.rs` - MaxMind GeoLite2 DB loader
- [ ] Download + parse GeoLite2-Country CSV/MMDB → populate eBPF LPM trie
- [ ] Background task: auto-update DB (monthly)
- [ ] API: `GET /api/v1/geoip/countries` - List all countries with block/allow status
- [ ] API: `PUT /api/v1/geoip/countries` - Set blocked countries list
- [ ] API: `POST /api/v1/geoip/update` - Force re-download GeoIP DB
- [ ] API: `GET /api/v1/geoip/lookup/{ip}` - Lookup country for an IP

### Dioxus UI - GeoIP
- [ ] `nylon-wall-ui/src/components/geoip.rs` - GeoIP page
- [ ] World map or country list with toggle block/allow per country
- [ ] Search/filter countries
- [ ] IP lookup tool
- [ ] Dashboard: blocked countries count
- [ ] `nylon-wall-ui/src/app.rs` - Add `/geoip` route + sidebar nav link (icon: LdGlobe)

---

## Phase 17: Utility Tools

### Wake-on-LAN
- [ ] API: `POST /api/v1/tools/wol` - Send magic packet (MAC + broadcast address)
- [ ] API: `GET /api/v1/tools/wol/devices` - Saved WOL devices list
- [ ] API: `POST /api/v1/tools/wol/devices` - Save device (name, MAC, interface)
- [ ] API: `DELETE /api/v1/tools/wol/devices/{id}` - Remove saved device
- [ ] Daemon: construct and send magic packet (6x `0xFF` + 16x MAC) via UDP broadcast

### mDNS Reflector
- [ ] `nylon-wall-daemon/src/mdns.rs` - mDNS reflector (forward mDNS between interfaces/VLANs)
- [ ] Listen on `224.0.0.251:5353` on configured interfaces
- [ ] Re-broadcast received mDNS packets to other configured interfaces
- [ ] API: `GET /api/v1/tools/mdns` - Get mDNS reflector config
- [ ] API: `PUT /api/v1/tools/mdns` - Set interfaces to reflect between
- [ ] API: `POST /api/v1/tools/mdns/toggle` - Enable/disable reflector

### UPnP/NAT-PMP
- [ ] `nylon-wall-daemon/src/upnp.rs` - UPnP IGD + NAT-PMP server
- [ ] Auto-create temporary NAT port forward rules on client request
- [ ] Configurable: enable/disable, allowed port ranges, max lease time
- [ ] API: `GET /api/v1/tools/upnp` - UPnP config + active mappings
- [ ] API: `PUT /api/v1/tools/upnp` - Update UPnP config
- [ ] API: `DELETE /api/v1/tools/upnp/mappings/{id}` - Force remove mapping

### Captive Portal
- [ ] `nylon-wall-daemon/src/captive.rs` - Captive portal redirect
- [ ] eBPF: redirect HTTP (port 80) from unauthenticated clients to portal page
- [ ] eBPF map: `portal_whitelist` (HashMap - src_ip → authenticated)
- [ ] Daemon serves portal HTML page
- [ ] On accept: add client IP to whitelist map (with TTL)
- [ ] API: `GET /api/v1/tools/captive` - Captive portal config
- [ ] API: `PUT /api/v1/tools/captive` - Update config (interface, message, terms)
- [ ] API: `POST /api/v1/tools/captive/toggle` - Enable/disable

### Dioxus UI - Tools
- [ ] `nylon-wall-ui/src/components/tools.rs` - Tools page with sections
- [ ] Wake-on-LAN: device cards with wake button
- [ ] mDNS reflector: interface multi-select + enable toggle
- [ ] UPnP: config + active port mappings table
- [ ] Captive portal: config form + connected clients list
- [ ] `nylon-wall-ui/src/app.rs` - Add `/tools` route + sidebar nav link (icon: LdWrench)

---

## Phase 18: TLS Inspection

### Level 1: SNI Filtering (eBPF — no decryption)
TLS ClientHello contains the domain name (SNI) in plaintext before encryption starts.
eBPF can parse this to block/allow HTTPS by domain without breaking encryption.

#### eBPF - SNI extraction
- [x] `nylon-wall-ebpf/src/tls.rs` - TLS ClientHello parser
  - Detect TCP port 443 + TLS record type `0x16` (Handshake) + HandshakeType `0x01` (ClientHello)
  - Walk TLS extensions to find SNI extension (type `0x0000`)
  - Extract server_name from SNI extension
  - FNV-1a hash domain name for eBPF map lookup (exact + wildcard parent domain)
- [x] eBPF map: `SNI_POLICY` (HashMap<u32,u8> - domain_hash → action: allow/block/log, 16384 entries)
- [x] eBPF map: `SNI_EVENTS` (PerfEventArray<EbpfSniEvent>)
- [x] eBPF map: `SNI_ENABLED` (Array<u32> - global feature flag)
- [x] `stages/egress_sni.rs` - TC tail-call stage: parse ClientHello → lookup SNI → drop/pass (with `pull_data(512)`)
- [x] `stages/ingress_sni.rs` - XDP tail-call stage: same for inbound TLS connections
- [x] Handle fragmented ClientHello (best-effort: AND-masked bounds, MAX_EXTENSIONS=16, MAX_DOMAIN_SCAN=64)
- [x] Tail call dispatch: entry → NAT (stage 0) → SNI (stage 1) → Rules (stage 2)

#### Daemon - SNI Filter
- [x] SNI policy management in `api.rs` (FNV-1a hash matching eBPF, wildcard `*.domain` support)
- [x] `sync_sni_to_ebpf()` - Domain list → compute hashes → push to eBPF `SNI_POLICY` map
- [x] `sync_sni_to_maps()` in `ebpf_loader.rs` - Clear and re-populate eBPF map on rule changes
- [x] API: `GET /api/v1/tls/sni/rules` - List SNI filter rules
- [x] API: `POST /api/v1/tls/sni/rules` - Add SNI rule (domain, action: allow/block/log, category)
- [x] API: `PUT /api/v1/tls/sni/rules/{id}` - Update SNI rule
- [x] API: `DELETE /api/v1/tls/sni/rules/{id}` - Remove SNI rule
- [x] API: `POST /api/v1/tls/sni/rules/{id}/toggle` - Enable/disable individual rule
- [x] API: `GET /api/v1/tls/sni/stats` - SNI statistics (inspected, blocked, allowed, logged)
- [x] API: `POST /api/v1/tls/sni/toggle` - Global enable/disable SNI filtering
- [x] API: `GET /api/v1/tls/sni/debug` - Debug map contents
- [ ] Category-based blocking: bulk import domain lists (ads, social media, adult, malware)

#### Dioxus UI - SNI Filtering
- [x] `nylon-wall-ui/src/components/tls.rs` - SNI Filtering page
  - Statistics cards (status, inspected, blocked, allowed, logged)
  - Global toggle button with confirmation modal
  - SNI rules table (domain, action, category, hits, status, actions)
  - Add/edit rule form (domain pattern, action block/allow/log, category, enabled)
  - Per-rule toggle and delete with confirmation modals
- [x] `nylon-wall-ui/src/app.rs` - `/tls` route + sidebar nav link (icon: LdLock, label: "TLS / SNI")
