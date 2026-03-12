# Nylon Wall - Network Firewall Specification

## Overview

Nylon Wall เป็นระบบ Network Firewall ที่สร้างด้วย Rust โดยใช้ eBPF (Extended Berkeley Packet Filter) สำหรับ packet processing ในระดับ kernel และ Dioxus สำหรับ Web UI ในการจัดการและ monitor ระบบ

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   Dioxus Web UI                     │
│            (Management & Monitoring)                │
├─────────────────────────────────────────────────────┤
│                  REST API Layer                     │
│               (axum HTTP server)                    │
├─────────────────────────────────────────────────────┤
│               Nylon Wall Daemon                     │
│     ┌──────────┬──────────┬──────────────┐          │
│     │ Rule     │ State    │ Logging &    │          │
│     │ Engine   │ Manager  │ Metrics      │          │
│     └──────────┴──────────┴──────────────┘          │
├─────────────────────────────────────────────────────┤
│            eBPF Userspace Controller                │
│         (aya - Rust eBPF library)                   │
├─────────────────────────────────────────────────────┤
│              Linux Kernel (eBPF)                    │
│  ┌──────────┬──────────┬──────────┬──────────┐      │
│  │ XDP      │ TC       │ Cgroup   │ Socket   │      │
│  │ Programs │ Programs │ Programs │ Programs │      │
│  └──────────┴──────────┴──────────┴──────────┘      │
│         eBPF Maps (shared state/config)             │
└─────────────────────────────────────────────────────┘
```

## Tech Stack

| Component | Technology | Purpose |
|-----------|-----------|---------|
| eBPF Framework | [aya](https://github.com/aya-rs/aya) | เขียน eBPF programs ใน Rust |
| Web UI | [Dioxus](https://dioxuslabs.com/) | Fullstack web UI |
| HTTP Server | axum | REST API backend |
| Database | [SlateDB](https://slatedb.io/) | Embedded KV store บน object storage เก็บ rules, logs, config |
| Object Store | object_store (local filesystem / S3) | Storage backend สำหรับ SlateDB |
| Serialization | serde + serde_json | Config & API payloads |
| Logging | tracing | Structured logging |
| IPC | Unix domain socket / gRPC (tonic) | Daemon <-> UI communication |

## Project Structure

```
nylon-wall/
├── Cargo.toml                    # Workspace root
├── nylon-wall-ebpf/              # eBPF programs (kernel space)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs               # eBPF entrypoint
│       ├── ingress.rs            # Ingress filter (XDP)
│       ├── egress.rs             # Egress filter (TC)
│       ├── nat.rs                # NAT processing
│       └── common.rs             # Shared eBPF structs
├── nylon-wall-common/            # Shared types (kernel + userspace)
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs                # PacketAction, Rule, NatEntry, etc.
├── nylon-wall-daemon/            # Userspace daemon
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs               # Daemon entrypoint
│       ├── ebpf_loader.rs        # Load/attach eBPF programs
│       ├── rule_engine.rs        # Rule CRUD & compilation to eBPF maps
│       ├── nat.rs                # NAT table management
│       ├── route.rs              # Routing table management
│       ├── state.rs              # Connection tracking state
│       ├── api.rs                # REST API (axum)
│       ├── metrics.rs            # Prometheus-compatible metrics
│       └── db.rs                 # SlateDB persistence (key-value store)
├── nylon-wall-ui/                # Dioxus Web UI
│   ├── Cargo.toml
│   ├── Dioxus.toml
│   ├── assets/
│   └── src/
│       ├── main.rs               # UI entrypoint
│       ├── app.rs                # Root App component
│       ├── components/
│       │   ├── dashboard.rs      # Overview dashboard
│       │   ├── rules.rs          # Firewall rules management
│       │   ├── nat.rs            # NAT configuration
│       │   ├── routes.rs         # Routing table
│       │   ├── policies.rs       # Network policies
│       │   ├── logs.rs           # Log viewer
│       │   └── settings.rs       # System settings
│       ├── api_client.rs         # HTTP client to daemon API
│       └── models.rs             # UI data models
└── spec.md
```

---

## Features

### 1. Packet Filtering (Ingress / Egress)

**eBPF Program Type:** XDP (ingress), TC (egress)

#### Ingress Filtering
- กรอง packet ขาเข้าที่ระดับ XDP (เร็วที่สุด ก่อนเข้า kernel network stack)
- รองรับ filter ตาม:
  - Source/Destination IP (IPv4/IPv6)
  - Source/Destination Port
  - Protocol (TCP, UDP, ICMP)
  - Interface
  - TCP flags (SYN, ACK, FIN, RST)
  - Packet size range
- Actions: `ALLOW`, `DROP`, `LOG`, `RATE_LIMIT`
- Priority-based rule ordering

#### Egress Filtering
- กรอง packet ขาออกผ่าน TC (Traffic Control) hook
- รองรับ filter เดียวกับ ingress
- ใช้สำหรับป้องกัน data exfiltration
- สามารถ block outbound connections ไปยัง IP/domain ที่กำหนด

#### Rule Structure

```rust
struct FirewallRule {
    id: u32,
    name: String,
    priority: u32,              // ลำดับการ evaluate (ต่ำ = ก่อน)
    direction: Direction,       // Ingress | Egress
    enabled: bool,

    // Match conditions
    src_ip: Option<IpNetwork>,  // CIDR notation
    dst_ip: Option<IpNetwork>,
    src_port: Option<PortRange>,
    dst_port: Option<PortRange>,
    protocol: Option<Protocol>, // TCP | UDP | ICMP | Any
    interface: Option<String>,

    // Action
    action: Action,             // Allow | Drop | Log | RateLimit(pps)

    // Metadata
    hit_count: u64,
    created_at: i64,
    updated_at: i64,
}
```

### 2. NAT (Network Address Translation)

**eBPF Program Type:** TC (ingress + egress)

#### SNAT (Source NAT / Masquerade)
- แปลง source IP ของ packet ขาออกเป็น IP ของ interface ปลายทาง
- ใช้สำหรับ internet sharing จาก private network
- รองรับ port range mapping

#### DNAT (Destination NAT / Port Forwarding)
- แปลง destination IP/port ของ packet ขาเข้า
- ใช้สำหรับ expose internal services
- รองรับ 1:1 NAT และ port forwarding

#### NAT Table Structure

```rust
struct NatEntry {
    id: u32,
    nat_type: NatType,          // SNAT | DNAT | Masquerade
    enabled: bool,

    // Match
    src_network: Option<IpNetwork>,
    dst_network: Option<IpNetwork>,
    protocol: Option<Protocol>,
    dst_port: Option<PortRange>,
    in_interface: Option<String>,
    out_interface: Option<String>,

    // Translation
    translate_ip: Option<IpAddr>,
    translate_port: Option<PortRange>,
}
```

### 3. Routing

**eBPF Program Type:** XDP + kernel routing table integration

#### Static Routes
- เพิ่ม/ลบ static routes ผ่าน UI
- รองรับ default gateway, per-subnet routes
- Metric-based route selection

#### Policy-Based Routing
- Route ตาม source IP, port, protocol
- Multiple routing tables
- ใช้ร่วมกับ eBPF เพื่อ mark packets สำหรับ policy routing

#### Route Structure

```rust
struct Route {
    id: u32,
    destination: IpNetwork,     // e.g., 10.0.0.0/8
    gateway: Option<IpAddr>,
    interface: String,
    metric: u32,
    table: u32,                 // routing table ID
    enabled: bool,
}

struct PolicyRoute {
    id: u32,
    // Match
    src_ip: Option<IpNetwork>,
    dst_ip: Option<IpNetwork>,
    src_port: Option<PortRange>,
    protocol: Option<Protocol>,
    // Action
    route_table: u32,
    priority: u32,
}
```

### 4. Network Policy

Zone-based policy model ที่จัดกลุ่ม interfaces เป็น zones

#### Zones
- กำหนด security zones (e.g., `wan`, `lan`, `dmz`, `guest`)
- แต่ละ zone มี default policy (allow/deny)
- กำหนด inter-zone policy

#### Policy Structure

```rust
struct Zone {
    id: u32,
    name: String,               // e.g., "lan", "wan", "dmz"
    interfaces: Vec<String>,    // e.g., ["eth0", "eth1"]
    default_policy: Action,     // Allow | Drop
}

struct NetworkPolicy {
    id: u32,
    name: String,
    enabled: bool,
    from_zone: String,
    to_zone: String,

    // Match (optional, ถ้าไม่กำหนดจะ match ทุก traffic ใน zone pair)
    src_ip: Option<IpNetwork>,
    dst_ip: Option<IpNetwork>,
    dst_port: Option<PortRange>,
    protocol: Option<Protocol>,
    schedule: Option<Schedule>,  // time-based policy

    action: Action,
    log: bool,
    priority: u32,
}

struct Schedule {
    days: Vec<Weekday>,         // Mon-Sun
    start_time: NaiveTime,      // e.g., 08:00
    end_time: NaiveTime,        // e.g., 17:00
}
```

### 5. Connection Tracking (Conntrack)

**eBPF Map Type:** LRU Hash Map

- Track stateful connections (TCP/UDP/ICMP)
- อนุญาต return traffic อัตโนมัติสำหรับ established connections
- Connection states: `NEW`, `ESTABLISHED`, `RELATED`, `INVALID`
- Timeout per protocol (TCP: 3600s, UDP: 300s, ICMP: 30s)
- Conntrack table viewable ผ่าน UI

```rust
struct ConntrackEntry {
    src_ip: IpAddr,
    dst_ip: IpAddr,
    src_port: u16,
    dst_port: u16,
    protocol: u8,
    state: ConnState,
    packets_in: u64,
    packets_out: u64,
    bytes_in: u64,
    bytes_out: u64,
    last_seen: u64,             // timestamp
    timeout: u32,
}
```

### 6. Rate Limiting / QoS

**eBPF Map Type:** Per-CPU Array (token bucket)

- Rate limit per rule, per source IP, per zone
- Token bucket algorithm ใน eBPF
- Configurable burst size
- Bandwidth limiting (bytes/sec) และ packet limiting (packets/sec)

### 7. Logging & Monitoring

#### Packet Logging
- Log matched packets ไปยัง ring buffer (eBPF perf event)
- Daemon อ่าน events แล้วเขียนลง SlateDB + stream ไป UI
- Log fields: timestamp, src/dst IP, port, protocol, action, rule_id, interface

#### Metrics (Prometheus-compatible)
- Packets/bytes per rule, per interface, per zone
- Connection count, new connections/sec
- Drop rate, allow rate
- NAT translation count
- CPU usage ของ eBPF programs
- Expose ที่ `/metrics` endpoint

#### Real-time Dashboard
- Live traffic graph (packets/sec, bytes/sec)
- Top talkers (source/dest IP)
- Recent blocked packets
- Connection count per zone
- System health (CPU, memory, interfaces status)

### 8. DHCP Server (Optional Phase 2)

- Built-in DHCP server สำหรับ LAN interfaces
- IP pool management
- Static lease mapping (MAC -> IP)
- DNS server assignment

### 9. DNS Filtering (Optional Phase 2)

- Block domains ตาม blocklist
- Custom DNS responses
- DNS query logging

---

## eBPF Map Design

```
┌─────────────────────────────────────────────────┐
│               eBPF Maps                         │
├──────────────────┬──────────────────────────────┤
│ Map Name         │ Type / Purpose               │
├──────────────────┼──────────────────────────────┤
│ ingress_rules    │ Array - ingress rules        │
│ egress_rules     │ Array - egress rules         │
│ nat_table        │ HashMap - NAT entries        │
│ conntrack        │ LRU HashMap - conn state     │
│ rate_limit       │ PerCpuArray - token bucket   │
│ zone_map         │ HashMap - ifindex->zone      │
│ policy_map       │ HashMap - zone pair rules    │
│ metrics_map      │ PerCpuArray - counters       │
│ events           │ PerfEventArray - log events  │
│ blocked_ips      │ HashMap - IP blocklist       │
│ route_marks      │ HashMap - policy routing     │
└──────────────────┴──────────────────────────────┘
```

## SlateDB Storage Design

SlateDB เป็น embedded key-value store ที่ใช้ object storage เป็น backend
สำหรับ local deployment ใช้ filesystem, สำหรับ cloud ใช้ S3/GCS/ABS

### Key Schema (prefix-based namespacing)

```
Key Pattern                          → Value (serde JSON bytes)
─────────────────────────────────────────────────────────────
rule:{id}                            → FirewallRule
rule:index:priority                  → Vec<u32> (ordered rule IDs)
nat:{id}                             → NatEntry
route:{id}                           → Route
route:policy:{id}                    → PolicyRoute
zone:{id}                            → Zone
policy:{id}                          → NetworkPolicy
log:{timestamp}:{seq}                → PacketLog
metric:{name}:{timestamp}            → MetricPoint
config:{key}                         → ConfigValue
backup:{timestamp}                   → SnapshotData
```

### Key Design Decisions

- **Prefix scan** ใช้ SlateDB range scan เพื่อ list ข้อมูลตาม type
  (e.g., scan `rule:` .. `rule:\xff` จะได้ rules ทั้งหมด)
- **Log retention** ใช้ TTL ของ SlateDB เพื่อ auto-expire logs เก่า
- **Serialization** ใช้ `serde_json` serialize structs เป็น bytes สำหรับ value
- **Atomic updates** ใช้ SlateDB transactions สำหรับ multi-key updates
  (e.g., อัปเดต rule + reorder priority index พร้อมกัน)
- **Object store backend** - local filesystem สำหรับ single-node,
  S3-compatible สำหรับ distributed/backup scenarios

### Rust Usage Pattern

```rust
use slatedb::Db;
use slatedb::object_store::local::LocalFileSystem;
use std::sync::Arc;

// Initialize
let object_store = Arc::new(LocalFileSystem::new_with_prefix("/var/lib/nylon-wall/slatedb")?);
let db = Db::open("/", object_store).await?;

// Store a rule (serialize to JSON bytes)
let rule = FirewallRule { id: 1, name: "block-ssh".into(), .. };
let key = format!("rule:{}", rule.id);
db.put(key.as_bytes(), serde_json::to_vec(&rule)?).await?;

// Get a rule
if let Some(value) = db.get(key.as_bytes()).await? {
    let rule: FirewallRule = serde_json::from_slice(&value)?;
}

// List all rules (range scan)
let mut iter = db.scan("rule:".as_bytes()..="rule:\xff".as_bytes()).await?;
while let Ok(Some(entry)) = iter.next().await {
    let rule: FirewallRule = serde_json::from_slice(&entry.value)?;
}
```

---

## API Design

Base URL: `http://localhost:9450/api/v1`

### Firewall Rules
| Method | Path | Description |
|--------|------|-------------|
| GET | /rules | List all rules |
| POST | /rules | Create rule |
| GET | /rules/{id} | Get rule by ID |
| PUT | /rules/{id} | Update rule |
| DELETE | /rules/{id} | Delete rule |
| POST | /rules/{id}/toggle | Enable/disable rule |
| POST | /rules/reorder | Reorder rule priorities |

### NAT
| Method | Path | Description |
|--------|------|-------------|
| GET | /nat | List NAT entries |
| POST | /nat | Create NAT entry |
| PUT | /nat/{id} | Update NAT entry |
| DELETE | /nat/{id} | Delete NAT entry |

### Routes
| Method | Path | Description |
|--------|------|-------------|
| GET | /routes | List routes |
| POST | /routes | Add route |
| PUT | /routes/{id} | Update route |
| DELETE | /routes/{id} | Delete route |
| GET | /routes/policy | List policy routes |
| POST | /routes/policy | Add policy route |

### Network Policy
| Method | Path | Description |
|--------|------|-------------|
| GET | /zones | List zones |
| POST | /zones | Create zone |
| PUT | /zones/{id} | Update zone |
| DELETE | /zones/{id} | Delete zone |
| GET | /policies | List policies |
| POST | /policies | Create policy |
| PUT | /policies/{id} | Update policy |
| DELETE | /policies/{id} | Delete policy |

### Monitoring
| Method | Path | Description |
|--------|------|-------------|
| GET | /conntrack | List active connections |
| GET | /logs | Query packet logs |
| GET | /metrics | Prometheus metrics |
| WS | /ws/events | Real-time event stream |

### System
| Method | Path | Description |
|--------|------|-------------|
| GET | /system/interfaces | List network interfaces |
| GET | /system/status | Daemon & eBPF status |
| POST | /system/apply | Apply pending config |
| POST | /system/backup | Export config |
| POST | /system/restore | Import config |

## UI Pages (Dioxus)

### 1. Dashboard (`/`)
- Traffic overview graph (real-time)
- Active connections count
- Top 5 blocked IPs
- Interface status cards
- Quick stats: total rules, active NAT entries, uptime

### 2. Firewall Rules (`/rules`)
- Table view: priority, name, direction, match, action, hits, status
- Create/edit rule form with live preview
- Drag-and-drop reorder
- Bulk enable/disable
- Import/export rules

### 3. NAT (`/nat`)
- SNAT/DNAT entries table
- Port forwarding wizard
- Masquerade toggle per interface

### 4. Routes (`/routes`)
- Routing table view
- Static route editor
- Policy route editor
- Route visualization (optional)

### 5. Network Policies (`/policies`)
- Zone management
- Inter-zone policy matrix
- Policy editor with schedule support

### 6. Connections (`/connections`)
- Live conntrack table
- Filter by state, protocol, IP
- Kill connection action

### 7. Logs (`/logs`)
- Packet log table with filters
- Real-time log stream
- Export to CSV

### 8. Settings (`/settings`)
- Interface configuration
- Daemon settings
- Backup/restore
- About / system info

---

## Implementation Phases

### Phase 1: Foundation
1. ตั้ง workspace structure (Cargo workspace)
2. สร้าง `nylon-wall-common` - shared types
3. สร้าง `nylon-wall-ebpf` - basic XDP program (pass/drop)
4. สร้าง `nylon-wall-daemon` - load eBPF, basic API
5. ทดสอบ packet drop/allow บน test interface

### Phase 2: Core Firewall
1. Implement ingress rules (XDP) + rule evaluation ใน eBPF
2. Implement egress rules (TC)
3. Connection tracking
4. Rule CRUD API + SlateDB persistence
5. Basic Dioxus UI - dashboard + rules page

### Phase 3: NAT & Routing
1. SNAT/Masquerade
2. DNAT/Port forwarding
3. Static routing integration
4. Policy-based routing
5. NAT + Routes UI pages

### Phase 4: Network Policy & Zones
1. Zone definition + interface mapping
2. Inter-zone policy engine
3. Schedule-based policies
4. Policy UI page with zone matrix

### Phase 5: Monitoring & Polish
1. Real-time metrics + Prometheus endpoint
2. Packet logging via perf events
3. WebSocket live event stream
4. Dashboard with charts (plotters or chart.js via Dioxus)
5. Connection viewer
6. Log viewer with search/filter

### Phase 6: Hardening & Extras
1. Config backup/restore
2. Rate limiting / QoS
3. IPv6 full support
4. Performance tuning & benchmarking
5. Documentation

---

## Build & Run Requirements

- **OS:** Linux (kernel >= 5.15 สำหรับ eBPF features)
- **Toolchain:** Rust nightly (สำหรับ eBPF compilation via aya)
- **Privileges:** root / CAP_BPF + CAP_NET_ADMIN
- **Build Dependencies:**
  - `bpf-linker` (สำหรับ compile eBPF programs)
  - `dioxus-cli` (สำหรับ build UI)

### Build Commands

```bash
# Install tools
cargo install bpf-linker
cargo install dioxus-cli

# Build eBPF programs
cargo build -p nylon-wall-ebpf --target bpfel-unknown-none -Z build-std=core

# Build daemon
cargo build -p nylon-wall-daemon --release

# Build UI
cd nylon-wall-ui && dx build --release

# Run (requires root)
sudo ./target/release/nylon-wall-daemon
```

## Configuration File

Default: `/etc/nylon-wall/config.toml`

```toml
[daemon]
listen_addr = "127.0.0.1:9450"
db_path = "/var/lib/nylon-wall/slatedb"   # SlateDB local object store path
log_level = "info"

[ebpf]
xdp_mode = "driver"            # driver | skb | hw
interfaces = ["eth0", "eth1"]

[ui]
enabled = true
bind_addr = "0.0.0.0:8080"
```
