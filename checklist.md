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

### Dioxus UI - WebSocket Real-time Events
- [x] `nylon-wall-ui/src/ws_client.rs` - WebSocket client with auto-reconnect + per-category event bus
- [x] `nylon-wall-ui/src/api_client.rs` - `ws_url()` helper (ws/wss from page location or config)
- [x] `nylon-wall-ui/src/app.rs` - `use_ws_provider()` in Layout (connects on mount)
- [x] Per-category `Signal<u64>` generation counters (rules, nat, routes, zones, policies, dhcp, sni, logs, system)
- [x] All components react to WebSocket events — auto-refetch on real-time changes
  - Dashboard (rules, nat, dhcp, logs, system), Rules, NAT, Routes, Policies (zones+policies),
    DHCP (pools, leases, reservations, clients), TLS/SNI, Connections, Logs, Settings

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

## Phase 8: Virtual Networking (VLAN + Bridge) ✅

### nylon-wall-common - Virtual Network Types
- [x] `nylon-wall-common/src/vnet.rs` - `VlanConfig`, `BridgeConfig` structs (std-gated)
  - `VlanConfig`: id, parent_interface, vlan_id (1-4094), ip_address (optional CIDR), enabled, `iface_name()` helper
  - `BridgeConfig`: id, name, ports (Vec<String>), ip_address (optional CIDR), stp_enabled, enabled
- [x] `nylon-wall-common/src/lib.rs` - Add `pub mod vnet`

### nylon-wall-daemon - VLAN Module
- [x] `nylon-wall-daemon/src/vnet/mod.rs` - Module declarations
- [x] `nylon-wall-daemon/src/vnet/vlan.rs` - Create/delete VLAN sub-interfaces (Linux cfg-gated)
- [x] `nylon-wall-daemon/src/vnet/bridge.rs` - Create/delete Linux bridges + port diff on update
- [x] Persist configs in SlateDB (`vlan:` and `bridge:` key prefixes)
- [ ] Startup order: create VLANs first, then bridges on daemon restart

### Daemon - VLAN API
- [x] API: `GET /api/v1/vnet/vlans` - List VLAN interfaces
- [x] API: `POST /api/v1/vnet/vlans` - Create VLAN sub-interface
- [x] API: `PUT /api/v1/vnet/vlans/{id}` - Update VLAN (IP config)
- [x] API: `DELETE /api/v1/vnet/vlans/{id}` - Delete VLAN sub-interface
- [x] API: `POST /api/v1/vnet/vlans/{id}/toggle` - Toggle VLAN enabled
- [x] Validation: prevent duplicate VLAN ID on same parent interface
- [x] Validation: parent interface required, VLAN ID 1-4094

### Daemon - Bridge API
- [x] API: `GET /api/v1/vnet/bridges` - List bridges
- [x] API: `POST /api/v1/vnet/bridges` - Create bridge
- [x] API: `PUT /api/v1/vnet/bridges/{id}` - Update bridge (ports diff, IP, STP)
- [x] API: `DELETE /api/v1/vnet/bridges/{id}` - Delete bridge
- [x] API: `POST /api/v1/vnet/bridges/{id}/toggle` - Toggle bridge enabled
- [x] Validation: bridge name required, duplicate name check

### WebSocket Events
- [x] `vlan_created`, `vlan_updated`, `vlan_toggled`, `vlan_deleted`
- [x] `bridge_created`, `bridge_updated`, `bridge_toggled`, `bridge_deleted`

### Backup/Restore
- [x] `vlan_configs` and `bridge_configs` in BackupData
- [x] `perform_restore` clears and restores VLAN + bridge prefixes
- [x] `snapshot_current` includes VLAN + bridge data

### Dioxus UI - Virtual Networking
- [x] `nylon-wall-ui/src/components/vnet.rs` - Virtual Networking page with 2-tab layout
- [x] Tab: VLANs - VLAN cards + create/edit form (parent interface, VLAN ID, IP)
- [x] Tab: Bridges - Bridge cards + create/edit form (name, ports, IP, STP toggle)
- [x] `nylon-wall-ui/src/components/mod.rs` - Export `Vnet` component
- [x] `nylon-wall-ui/src/app.rs` - Add `/vnet` route + sidebar nav link (icon: LdGitMerge)
- [ ] Show VLANs + bridges in interface selects across all pages (rules, NAT, DHCP, routes)

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

### Daemon - Auth (Phase 9A: Local Password)
- [x] `nylon-wall-daemon/src/auth.rs` - Session management (bcrypt password hash, JWT tokens)
- [x] SlateDB: store admin password hash (`auth:admin_password`) + JWT secret (`auth:jwt_secret`)
- [x] API: `GET /api/v1/auth/setup-check` - Check if initial setup is required
- [x] API: `POST /api/v1/auth/setup` - First-run password setup (returns JWT)
- [x] API: `POST /api/v1/auth/login` - Login (returns JWT)
- [x] API: `POST /api/v1/auth/logout` - Invalidate session (token revocation)
- [x] API: `PUT /api/v1/auth/password` - Change password
- [x] API: `GET /api/v1/auth/check` - Verify token validity
- [x] axum middleware: JWT validation on all `/api/v1/*` routes (except login/setup/setup-check)
- [x] First-run setup: if no password set, middleware allows all (UI redirects to setup)
- [x] WebSocket auth: token via query parameter `?token=` for browser WS connections

### Daemon - Auth (Phase 9B: OIDC/OAuth2)
- [x] OIDC/OAuth2 provider configuration (Google, GitHub, custom OIDC)
- [x] API: `GET /api/v1/auth/oauth/providers` - List enabled providers (public, no secrets)
- [x] API: `GET /api/v1/auth/oauth/manage` - List all providers (admin, secrets masked)
- [x] API: `POST /api/v1/auth/oauth/manage` - Add OAuth provider
- [x] API: `PUT /api/v1/auth/oauth/manage/{id}` - Update OAuth provider
- [x] API: `DELETE /api/v1/auth/oauth/manage/{id}` - Remove OAuth provider
- [x] API: `POST /api/v1/auth/oauth/manage/{id}/toggle` - Enable/disable provider
- [x] API: `GET /api/v1/auth/oauth/{id}/authorize` - Start OAuth flow (returns authorization URL)
- [x] API: `GET /api/v1/auth/oauth/callback` - OAuth callback (exchange code → JWT, redirect to UI)
- [x] CSRF state token management (10-min expiry)
- [x] Token exchange + userinfo fetch (Google, GitHub, custom OIDC)
- [x] UI: OAuth provider buttons on login page (dynamic, shows enabled providers)
- [x] UI: OAuth provider management in Settings (add/edit/delete/toggle)

### Dioxus UI - Auth
- [x] `nylon-wall-ui/src/components/login.rs` - Login page (password + centered card design)
- [x] `nylon-wall-ui/src/components/setup.rs` - First-run password setup page (with confirm)
- [x] JWT token storage in localStorage (`nylon_auth_token`)
- [x] `api_client.rs` - Attach `Authorization: Bearer` header to all requests
- [x] Auto-redirect to login on 401 response (clear token + navigate)
- [x] Session timeout handling (24h JWT expiry, auto-redirect on expired token)
- [x] Settings page: change password form
- [x] Sidebar: logout button
- [x] Auth guard in Layout (checks setup-check → token → validate on mount)

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

### nylon-wall-common - DDNS Types
- [x] `nylon-wall-common/src/ddns.rs` - `DdnsEntry`, `DdnsProvider`, `DdnsStatus`
- [x] `nylon-wall-common/src/lib.rs` - Add `pub mod ddns`

### Daemon - DDNS
- [x] `nylon-wall-daemon/src/ddns.rs` - DDNS updater (WAN IP detection + provider updates)
- [x] Support providers: Cloudflare, No-IP, DuckDNS, Dynu, custom URL
- [x] Background task: detect WAN IP change → update DNS record (per-entry loops)
- [x] SlateDB: store DDNS configs (`ddns:{id}`) + status (`ddns_status:{id}`)
- [x] API: `GET /api/v1/ddns` - List DDNS configs
- [x] API: `POST /api/v1/ddns` - Create DDNS config
- [x] API: `GET /api/v1/ddns/{id}` - Get DDNS config
- [x] API: `PUT /api/v1/ddns/{id}` - Update DDNS config
- [x] API: `DELETE /api/v1/ddns/{id}` - Delete DDNS config
- [x] API: `POST /api/v1/ddns/{id}/toggle` - Enable/disable
- [x] API: `POST /api/v1/ddns/{id}/update-now` - Force update now
- [x] API: `GET /api/v1/ddns/status` - All entry statuses (current IP, last update, errors)
- [x] WebSocket events: `ddns_created`, `ddns_updated`, `ddns_deleted`, `ddns_status_changed`
- [x] Startup: auto-start update loops for enabled entries

### Dioxus UI - DDNS
- [x] `nylon-wall-ui/src/components/ddns.rs` - DDNS config page
- [x] Provider selector + credentials form (per provider: Cloudflare zone_id, DuckDNS token, etc.)
- [x] Status display: current WAN IP, last update time, success/error badges
- [x] Force update button per entry
- [x] Toggle enable/disable, edit, delete with confirmation
- [x] Standalone `/ddns` route + sidebar nav link (LdGlobe icon under Network)
- [x] Stats cards: total entries, active, total updates
- [x] WebSocket real-time updates via event bus

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

## Phase 14: WireGuard VPN ✅

### Daemon - WireGuard
- [x] `nylon-wall-daemon/src/wireguard.rs` - WireGuard management via `wg` CLI / netlink
- [x] Create WireGuard interface: `ip link add wg0 type wireguard`
- [x] Key generation: `wg genkey`, `wg pubkey`
- [x] Apply config: `wg set wg0 listen-port {port} private-key {key}`
- [x] Peer management: `wg set wg0 peer {pubkey} allowed-ips {cidr} endpoint {addr}`
- [x] API: `GET/PUT /api/v1/vpn/server` - Server config CRUD
- [x] API: `POST /api/v1/vpn/server/toggle` - Toggle server enabled
- [x] API: `GET/POST /api/v1/vpn/peers` - List/create peers (auto-generate keys)
- [x] API: `PUT/DELETE /api/v1/vpn/peers/{id}` - Update/delete peer
- [x] API: `POST /api/v1/vpn/peers/{id}/toggle` - Toggle peer enabled
- [x] API: `GET /api/v1/vpn/peers/{id}/config` - Download peer config file
- [x] API: `GET /api/v1/vpn/status` - Live peer status from `wg show`
- [x] WebSocket events: `wg_server_updated`, `wg_peer_created`, `wg_peer_updated`, `wg_peer_deleted`
- [x] Backup/restore integration (wg_server + wg_peers)
- [ ] Auto-create firewall rules for VPN traffic (UDP listen port + wg0 interface)
- [ ] Auto-create NAT masquerade for VPN → LAN access

### Dioxus UI - WireGuard
- [x] `nylon-wall-ui/src/components/vpn.rs` - WireGuard VPN page
- [x] Server config form (listen port, address range, DNS, endpoint, interface)
- [x] Server toggle (enable/disable) with status display
- [x] Peer list with status indicators (connected/idle/disabled)
- [x] Peer config download button (.conf file)
- [x] Live peer status: transfer rx/tx, connection indicator
- [x] Peer CRUD (create/edit/delete with confirmation)
- [x] `nylon-wall-ui/src/app.rs` - Add `/vpn` route + sidebar nav link (icon: LdShieldCheck)
- [ ] QR code generation for mobile clients

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
- [x] `nylon-wall-common/src/wol.rs` - `WolDevice`, `WolRequest` types
- [x] `nylon-wall-daemon/src/wol.rs` - Magic packet builder + UDP broadcast sender (with unit tests)
- [x] API: `POST /api/v1/tools/wol` - Send magic packet (MAC + broadcast address)
- [x] API: `GET /api/v1/tools/wol/devices` - Saved WOL devices list
- [x] API: `POST /api/v1/tools/wol/devices` - Save device (name, MAC, interface)
- [x] API: `PUT /api/v1/tools/wol/devices/{id}` - Update device
- [x] API: `DELETE /api/v1/tools/wol/devices/{id}` - Remove saved device
- [x] API: `POST /api/v1/tools/wol/devices/{id}/wake` - Wake saved device (updates last_wake timestamp)
- [x] WebSocket events: `wol_device_created`, `wol_device_deleted`, `wol_sent`

### Network Diagnostics
- [x] API: `POST /api/v1/tools/ping` - Run ping with target validation
- [x] API: `POST /api/v1/tools/dns` - DNS lookup (dig/nslookup/host fallback)
- [x] API: `POST /api/v1/tools/traceroute` - Traceroute (traceroute/tracepath fallback)
- [x] Input validation: prevent command injection (alphanumeric + dots/hyphens/colons only)
- [x] UI: Tool tab selector (Ping / DNS Lookup / Traceroute) + target input + monospace output

### mDNS Reflector
- [x] `nylon-wall-daemon/src/mdns.rs` - mDNS reflector (forward mDNS between interfaces/VLANs)
- [x] Listen on `224.0.0.251:5353` on configured interfaces
- [x] Re-broadcast received mDNS packets to other configured interfaces
- [x] API: `GET /api/v1/tools/mdns` - Get mDNS reflector config
- [x] API: `PUT /api/v1/tools/mdns` - Set interfaces to reflect between
- [x] API: `POST /api/v1/tools/mdns/toggle` - Enable/disable reflector

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
- [x] `nylon-wall-ui/src/components/tools.rs` - Tools page with sections
- [x] Wake-on-LAN: device cards with wake button, quick-wake by MAC, add/edit/delete devices
- [x] `/tools` route + sidebar nav link (LdWrench icon under System)
- [x] mDNS reflector: interface multi-select + enable toggle
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

---

## Phase 19: L4 Proxy (eBPF)

L4 proxy ใช้ eBPF ทำ DNAT/SNAT ใน kernel สำหรับ TCP/UDP — pure kernel-space redirect ไม่มี userspace relay
แยกจาก NAT เดิม (Phase 3): NAT = simple port forward (1 target), L4 Proxy = reverse proxy + load balance (multiple upstreams + weight + strategy)
eBPF maps แยกชุดกัน ไม่ share กับ NAT

### nylon-wall-common - L4 Proxy Types
- [x] `nylon-wall-common/src/l4proxy.rs` - Shared types
  - **eBPF structs** (`#[repr(C)]`, `no_std`):
    - `EbpfL4ProxyKey`, `EbpfL4ProxyEntry`, `EbpfL4ProxyNatState`, `EbpfL4ProxyCounters`
  - **Userspace structs** (`#[cfg(feature = "std")]`):
    - `L4ProxyRule`: id, name, protocol (TCP/UDP), listen_address, listen_port, upstream_targets (Vec<UpstreamTarget>), load_balance (RoundRobin/IpHash), enabled
    - `UpstreamTarget`: address, port, weight
    - `LoadBalanceMode` enum: RoundRobin, IpHash
    - `L4ProxyStats`: rule_id, active_connections, total_connections, bytes_in, bytes_out
- [x] `nylon-wall-common/src/lib.rs` - Add `pub mod l4proxy`

### eBPF - L4 Proxy Fast-Path (kernel-space DNAT/SNAT)
- [ ] eBPF map: `L4_PROXY_TABLE` (HashMap<L4ProxyKey, EbpfL4ProxyEntry>, 256 entries)
  - Key: `{ protocol: u8, ip: u32, port: u16 }` (listen endpoint)
  - Value: `EbpfL4ProxyEntry` (upstream endpoint + flags)
- [ ] eBPF map: `L4_PROXY_CONNTRACK` (LruHashMap<ConntrackKey, L4ProxyNatState>, 16384 entries)
  - Track original src/dst for return-path SNAT
- [ ] eBPF map: `L4_PROXY_STATS` (PerCpuArray<L4ProxyCounters>)
  - Per-rule packet/byte counters (aggregated by daemon)
- [ ] `nylon-wall-ebpf/src/stages/ingress_l4proxy.rs` - XDP tail-call stage
  - Lookup (proto, dst_ip, dst_port) in `L4_PROXY_TABLE`
  - If match found:
    - Rewrite dst_ip/dst_port to upstream (DNAT)
    - Store original dst in `L4_PROXY_CONNTRACK` for return path
    - Recalculate IP/TCP/UDP checksums
    - XDP_TX or XDP_REDIRECT to upstream interface
  - If no match → continue tail-call pipeline (pass to next stage)
- [ ] `nylon-wall-ebpf/src/stages/egress_l4proxy.rs` - TC tail-call stage
  - Lookup return packets in `L4_PROXY_CONNTRACK`
  - Rewrite src_ip/src_port back to original listen endpoint (SNAT)
  - Recalculate checksums
- [ ] `nylon-wall-ebpf/src/main.rs` - Register new maps + tail-call stages
  - Add `STAGE_L4PROXY` constant to scratchpad dispatch (before NAT stage, or as stage 3)
  - Register `ingress_l4proxy` + `egress_l4proxy` in `XDP_DISPATCH` / `TC_DISPATCH`
- [ ] `nylon-wall-common/src/scratchpad.rs` - Add `STAGE_L4PROXY` constant

### nylon-wall-daemon - L4 Proxy Management
- [x] `nylon-wall-daemon/src/l4proxy/mod.rs` - Module declarations
- [x] `nylon-wall-daemon/src/l4proxy/sync.rs` - Sync rules to eBPF maps (placeholder — eBPF map writes added when eBPF programs built)
- [x] `nylon-wall-daemon/src/l4proxy/loadbalance.rs` - Load balancing strategies
  - Round-robin: atomic counter mod target count
  - IP hash: FNV-1a(client_ip) mod target count
- [ ] `nylon-wall-daemon/src/l4proxy/stats.rs` - Stats aggregation
  - Read `L4_PROXY_STATS` PerCpuArray → sum per-CPU counters
  - Expose per-rule: active_connections, total_connections, bytes_in, bytes_out
- [x] `nylon-wall-daemon/src/lib.rs` - Add `pub mod l4proxy`
- [ ] `nylon-wall-daemon/src/main.rs` - Start proxy engine on boot
  - Load rules from DB → sync to eBPF maps
- [ ] `nylon-wall-daemon/src/ebpf_loader.rs` - Load + attach L4 proxy eBPF programs
  - Register `ingress_l4proxy` / `egress_l4proxy` in dispatch tables

### Daemon - L4 Proxy API
- [x] API: `GET /api/v1/l4proxy/rules` - List all proxy rules
- [x] API: `POST /api/v1/l4proxy/rules` - Create proxy rule (sync eBPF map)
- [ ] API: `GET /api/v1/l4proxy/rules/{id}` - Get single rule
- [x] API: `PUT /api/v1/l4proxy/rules/{id}` - Update rule (re-sync eBPF map)
- [x] API: `DELETE /api/v1/l4proxy/rules/{id}` - Delete rule (remove from eBPF map)
- [x] API: `POST /api/v1/l4proxy/rules/{id}/toggle` - Enable/disable
- [ ] API: `GET /api/v1/l4proxy/stats` - All proxy stats (eBPF counters)
- [ ] API: `GET /api/v1/l4proxy/stats/{id}` - Stats for single rule
- [x] Validation: listen_port not already in use by another rule or system service
- [x] Validation: at least one upstream target required
- [x] WebSocket events: `l4proxy_created`, `l4proxy_updated`, `l4proxy_deleted`, `l4proxy_toggled`
- [x] Backup/restore: `l4proxy_rules` in BackupData

### Dioxus UI - L4 Proxy
- [x] `nylon-wall-ui/src/components/l4proxy.rs` - L4 Proxy page
- [ ] Stats cards: total rules, active rules, total connections, bandwidth in/out
- [x] Proxy rule table with data
  - Protocol badge (TCP/UDP)
  - Listen address:port → upstream targets display
  - Load balance mode badge
  - Toggle, edit, delete actions
- [x] Create/edit form:
  - Name, Protocol (TCP/UDP select), Listen address, Listen port
  - Upstream targets: multi-row (address, port, weight) with add/remove
  - Load balance mode (Round Robin / IP Hash)
- [x] `nylon-wall-ui/src/components/mod.rs` - Export `L4Proxy` component
- [x] `nylon-wall-ui/src/app.rs` - Add `/l4proxy` route + sidebar nav link (icon: LdArrowLeftRight)

### Testing
- [x] Unit test: Load balance strategies (round-robin, ip-hash determinism)
- [ ] Integration test: eBPF fast-path DNAT+SNAT (plain TCP)
- [ ] Integration test: eBPF fast-path UDP redirect
- [ ] Integration test: eBPF map sync on rule create/update/delete
- [ ] Performance test: eBPF fast-path throughput
