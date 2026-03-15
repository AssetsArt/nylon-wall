//! eBPF program loader — loads bytecode, attaches to interfaces, and populates maps.

#[cfg(target_os = "linux")]
mod linux {
    use std::os::fd::{AsFd, AsRawFd};

    use aya::Ebpf;
    use aya::maps::{Array, ProgramArray};
    use aya::programs::{ProgramFd, SchedClassifier, Xdp, XdpFlags};
    use nylon_wall_common::nat::EbpfNatEntry;
    use nylon_wall_common::rule::EbpfRule;
    use nylon_wall_common::scratchpad::{STAGE_NAT, STAGE_RULES, STAGE_SNI};
    use nylon_wall_common::zone::EbpfPolicyValue;
    use tracing::info;

    const EBPF_OBJ_PATH: &str = "/usr/lib/nylon-wall/nylon-wall-ebpf";

    /// Load eBPF programs and attach them. Returns the Ebpf handle for map access.
    pub async fn load_and_attach() -> anyhow::Result<Ebpf> {
        info!("Loading eBPF bytecode from {}...", EBPF_OBJ_PATH);

        let data = std::fs::read(EBPF_OBJ_PATH).map_err(|e| {
            anyhow::anyhow!(
                "Failed to read eBPF object at {}: {}. \
                 Build it first with: cargo build -p nylon-wall-ebpf \
                 --target bpfel-unknown-none -Z build-std=core",
                EBPF_OBJ_PATH,
                e
            )
        })?;

        let mut bpf = Ebpf::load(&data)?;

        // Initialize eBPF logger (optional — warns if AYA_LOGS map doesn't exist)
        if let Err(e) = aya_log::EbpfLogger::init(&mut bpf) {
            tracing::debug!("eBPF logger not available: {}", e);
        }

        let iface = std::env::var("NYLON_WALL_IFACE").unwrap_or_else(|_| "eth0".to_string());

        // === Attach entry point programs ===

        // Attach XDP ingress
        let program: &mut Xdp = bpf
            .program_mut("nylon_wall_ingress")
            .ok_or_else(|| anyhow::anyhow!("XDP program not found"))?
            .try_into()?;
        program.load()?;
        program.attach(&iface, XdpFlags::default())?;
        info!("XDP ingress attached to {}", iface);

        // Attach TC egress
        let _ = std::process::Command::new("tc")
            .args(["qdisc", "add", "dev", &iface, "clsact"])
            .output();

        let tc_prog: &mut SchedClassifier = bpf
            .program_mut("nylon_wall_egress")
            .ok_or_else(|| anyhow::anyhow!("TC program not found"))?
            .try_into()?;
        tc_prog.load()?;
        tc_prog.attach(&iface, aya::programs::TcAttachType::Egress)?;
        info!("TC egress attached to {}", iface);

        // === Load tail-call stage programs (load only, do NOT attach) ===

        // XDP tail-call targets
        for name in ["ingress_nat", "ingress_sni", "ingress_rules"] {
            let prog: &mut Xdp = bpf
                .program_mut(name)
                .ok_or_else(|| anyhow::anyhow!("XDP tail program '{}' not found", name))?
                .try_into()?;
            prog.load()?;
            info!("Loaded XDP tail program: {}", name);
        }

        // TC tail-call targets
        for name in ["egress_nat", "egress_sni", "egress_rules"] {
            let prog: &mut SchedClassifier = bpf
                .program_mut(name)
                .ok_or_else(|| anyhow::anyhow!("TC tail program '{}' not found", name))?
                .try_into()?;
            prog.load()?;
            info!("Loaded TC tail program: {}", name);
        }

        // === Register tail programs into dispatch ProgramArrays ===

        register_tail_call(&mut bpf, "ingress_nat", "XDP_DISPATCH", STAGE_NAT)?;
        register_tail_call(&mut bpf, "ingress_sni", "XDP_DISPATCH", STAGE_SNI)?;
        register_tail_call(&mut bpf, "ingress_rules", "XDP_DISPATCH", STAGE_RULES)?;

        register_tail_call(&mut bpf, "egress_nat", "TC_DISPATCH", STAGE_NAT)?;
        register_tail_call(&mut bpf, "egress_sni", "TC_DISPATCH", STAGE_SNI)?;
        register_tail_call(&mut bpf, "egress_rules", "TC_DISPATCH", STAGE_RULES)?;

        info!("Registered 6 tail-call programs into dispatch maps");

        // Set initial rule/NAT counts to 0
        set_map_u32(&mut bpf, "INGRESS_RULE_COUNT", 0, 0)?;
        set_map_u32(&mut bpf, "EGRESS_RULE_COUNT", 0, 0)?;
        set_map_u32(&mut bpf, "NAT_ENTRY_COUNT", 0, 0)?;

        info!("eBPF programs loaded and attached successfully (2 entry + 6 tail)");
        Ok(bpf)
    }

    /// Register a tail-call program into a ProgramArray dispatch map.
    ///
    /// Works around aya 0.13's borrow conflict: `bpf.program()` (immutable)
    /// and `bpf.map_mut()` (mutable) can't coexist. We collect the program's
    /// raw fd first (Copy, releases borrow), dup it into an OwnedFd, then
    /// transmute to ProgramFd (which is a transparent newtype over OwnedFd).
    fn register_tail_call(
        bpf: &mut Ebpf,
        prog_name: &str,
        map_name: &str,
        index: u32,
    ) -> anyhow::Result<()> {
        use std::os::fd::{FromRawFd, OwnedFd};

        // Get the program's raw fd. as_raw_fd() returns Copy i32,
        // so the borrow on `bpf` is released at the semicolon.
        let prog_raw_fd = bpf
            .program(prog_name)
            .ok_or_else(|| anyhow::anyhow!("Program '{}' not found", prog_name))?
            .fd()?
            .as_fd()
            .as_raw_fd();
        // bpf borrow is dropped here (i32 is Copy)

        // dup() the fd to get an independent owned copy
        let duped_fd = unsafe { libc::dup(prog_raw_fd) };
        if duped_fd < 0 {
            return Err(anyhow::anyhow!(
                "Failed to dup fd for '{}': {}",
                prog_name,
                std::io::Error::last_os_error()
            ));
        }
        let owned_fd = unsafe { OwnedFd::from_raw_fd(duped_fd) };

        // SAFETY: ProgramFd is a #[repr(transparent)] newtype around OwnedFd.
        // Layout: ProgramFd(OwnedFd) — single field, identical memory representation.
        let prog_fd: ProgramFd = unsafe { core::mem::transmute(owned_fd) };

        // Now we can safely get a mutable borrow for the map
        let map = bpf
            .map_mut(map_name)
            .ok_or_else(|| anyhow::anyhow!("Map '{}' not found", map_name))?;
        let mut prog_array: ProgramArray<_> = ProgramArray::try_from(map)?;
        prog_array.set(index, &prog_fd, 0)?;

        Ok(())
    }

    /// Push firewall rules into eBPF maps using typed Array API.
    pub fn sync_rules_to_maps(
        bpf: &mut Ebpf,
        ingress_rules: &[EbpfRule],
        egress_rules: &[EbpfRule],
    ) -> anyhow::Result<()> {
        write_rules_to_map(bpf, "INGRESS_RULES", ingress_rules)?;
        set_map_u32(bpf, "INGRESS_RULE_COUNT", 0, ingress_rules.len() as u32)?;

        write_rules_to_map(bpf, "EGRESS_RULES", egress_rules)?;
        set_map_u32(bpf, "EGRESS_RULE_COUNT", 0, egress_rules.len() as u32)?;

        tracing::info!(
            "Synced {} ingress + {} egress rules to eBPF maps",
            ingress_rules.len(),
            egress_rules.len()
        );
        Ok(())
    }

    /// Read all conntrack entries from the eBPF LRU HashMap.
    pub fn read_conntrack(bpf: &mut Ebpf) -> Vec<nylon_wall_common::conntrack::ConntrackInfo> {
        use nylon_wall_common::conntrack::{
            ConnState, ConntrackEntry, ConntrackInfo, ConntrackKey,
        };

        let map = match bpf.map_mut("CONNTRACK") {
            Some(m) => m,
            None => return Vec::new(),
        };

        let conntrack: aya::maps::HashMap<_, ConntrackKey, ConntrackEntry> =
            match aya::maps::HashMap::try_from(map) {
                Ok(m) => m,
                Err(_) => return Vec::new(),
            };

        let mut entries = Vec::new();
        for item in conntrack.iter() {
            let (key, entry) = match item {
                Ok(pair) => pair,
                Err(_) => continue,
            };

            let src_ip = std::net::Ipv4Addr::from(u32::from_be(key.src_ip));
            let dst_ip = std::net::Ipv4Addr::from(u32::from_be(key.dst_ip));

            let protocol = match key.protocol {
                6 => "TCP",
                17 => "UDP",
                1 => "ICMP",
                _ => "Other",
            };

            let state = match entry.state {
                0 => ConnState::New,
                1 => ConnState::Established,
                2 => ConnState::Related,
                4 => ConnState::Closing,
                _ => ConnState::Invalid,
            };

            entries.push(ConntrackInfo {
                src_ip: src_ip.to_string(),
                dst_ip: dst_ip.to_string(),
                src_port: key.src_port,
                dst_port: key.dst_port,
                protocol: protocol.to_string(),
                state,
                packets_in: entry.packets_in,
                packets_out: entry.packets_out,
                bytes_in: entry.bytes_in,
                bytes_out: entry.bytes_out,
                last_seen: entry.last_seen,
                timeout: entry.timeout,
            });
        }
        entries
    }

    /// Push NAT entries into eBPF maps.
    pub fn sync_nat_to_maps(
        bpf: &mut Ebpf,
        entries: &[EbpfNatEntry],
    ) -> anyhow::Result<()> {
        let map = bpf
            .map_mut("NAT_TABLE")
            .ok_or_else(|| anyhow::anyhow!("NAT_TABLE map not found"))?;
        let mut array: Array<_, EbpfNatEntry> = Array::try_from(map)?;

        for (i, entry) in entries.iter().enumerate() {
            array.set(i as u32, *entry, 0)?;
        }

        set_map_u32(bpf, "NAT_ENTRY_COUNT", 0, entries.len() as u32)?;

        info!("Synced {} NAT entries to eBPF maps", entries.len());
        Ok(())
    }

    fn write_rules_to_map(
        bpf: &mut Ebpf,
        map_name: &str,
        rules: &[EbpfRule],
    ) -> anyhow::Result<()> {
        let map = bpf
            .map_mut(map_name)
            .ok_or_else(|| anyhow::anyhow!("Map {} not found", map_name))?;
        let mut array: Array<_, EbpfRule> = Array::try_from(map)?;

        for (i, rule) in rules.iter().enumerate() {
            array.set(i as u32, *rule, 0)?;
        }
        Ok(())
    }

    /// Sync zone mappings (ifindex → zone_id) to eBPF map.
    pub fn sync_zones_to_maps(
        bpf: &mut Ebpf,
        zone_mappings: &[(u32, u32)], // (ifindex, zone_id)
    ) -> anyhow::Result<()> {
        let map = bpf
            .map_mut("ZONE_MAP")
            .ok_or_else(|| anyhow::anyhow!("ZONE_MAP map not found"))?;
        let mut hashmap: aya::maps::HashMap<_, u32, u32> =
            aya::maps::HashMap::try_from(map)?;

        // Clear existing entries (best-effort)
        let existing_keys: Vec<u32> = hashmap
            .keys()
            .filter_map(|k| k.ok())
            .collect();
        for key in existing_keys {
            let _ = hashmap.remove(&key);
        }

        for (ifindex, zone_id) in zone_mappings {
            hashmap.insert(ifindex, zone_id, 0)?;
        }

        info!("Synced {} zone mappings to eBPF maps", zone_mappings.len());
        Ok(())
    }

    /// Sync zone policies to eBPF map.
    pub fn sync_policies_to_maps(
        bpf: &mut Ebpf,
        policies: &[(u32, EbpfPolicyValue)], // (key = from_zone<<16|to_zone, value)
    ) -> anyhow::Result<()> {
        let map = bpf
            .map_mut("POLICY_MAP")
            .ok_or_else(|| anyhow::anyhow!("POLICY_MAP map not found"))?;
        let mut hashmap: aya::maps::HashMap<_, u32, EbpfPolicyValue> =
            aya::maps::HashMap::try_from(map)?;

        let existing_keys: Vec<u32> = hashmap
            .keys()
            .filter_map(|k| k.ok())
            .collect();
        for key in existing_keys {
            let _ = hashmap.remove(&key);
        }

        for (key, value) in policies {
            hashmap.insert(key, value, 0)?;
        }

        info!("Synced {} zone policies to eBPF maps", policies.len());
        Ok(())
    }

    /// Enable `route_localnet` on interfaces that have DNAT rules targeting
    /// loopback addresses (127.0.0.0/8). Without this, the kernel drops packets
    /// that arrive on non-loopback interfaces with a loopback destination after
    /// XDP DNAT rewrite.
    pub fn ensure_route_localnet(interfaces: &[String]) {
        for iface in interfaces {
            let path = format!("/proc/sys/net/ipv4/conf/{}/route_localnet", iface);
            match std::fs::write(&path, "1") {
                Ok(_) => info!("Enabled route_localnet on {} for loopback DNAT", iface),
                Err(e) => tracing::warn!("Failed to enable route_localnet on {}: {}", iface, e),
            }
        }
    }

    /// Sync SNI policy entries (hash → action) to the eBPF SNI_POLICY hash map.
    pub fn sync_sni_to_maps(
        bpf: &mut Ebpf,
        entries: &[(u32, u8)], // (domain_hash, action)
    ) -> anyhow::Result<()> {
        let map = bpf
            .map_mut("SNI_POLICY")
            .ok_or_else(|| anyhow::anyhow!("SNI_POLICY map not found"))?;
        let mut hashmap: aya::maps::HashMap<_, u32, u8> =
            aya::maps::HashMap::try_from(map)?;

        // Clear existing entries
        let existing_keys: Vec<u32> = hashmap
            .keys()
            .filter_map(|k| k.ok())
            .collect();
        for key in existing_keys {
            let _ = hashmap.remove(&key);
        }

        // Insert new entries
        for (hash, action) in entries {
            hashmap.insert(hash, action, 0)?;
        }

        info!("Synced {} SNI policy entries to eBPF maps", entries.len());
        Ok(())
    }

    fn set_map_u32(bpf: &mut Ebpf, map_name: &str, index: u32, value: u32) -> anyhow::Result<()> {
        let map = bpf
            .map_mut(map_name)
            .ok_or_else(|| anyhow::anyhow!("Map {} not found", map_name))?;
        let mut array: Array<_, u32> = Array::try_from(map)?;
        array.set(index, value, 0)?;
        Ok(())
    }

    /// Convert a userspace FirewallRule to an EbpfRule.
    pub fn firewall_rule_to_ebpf(rule: &nylon_wall_common::rule::FirewallRule) -> EbpfRule {
        let (src_ip, src_mask) = parse_cidr(rule.src_ip.as_deref());
        let (dst_ip, dst_mask) = parse_cidr(rule.dst_ip.as_deref());

        EbpfRule {
            id: rule.id,
            priority: rule.priority,
            direction: rule.direction as u8,
            enabled: if rule.enabled { 1 } else { 0 },
            protocol: rule.protocol.map(|p| p as u8).unwrap_or(0),
            action: rule.action as u8,
            src_ip,
            src_mask,
            dst_ip,
            dst_mask,
            src_port_start: rule.src_port.map(|p| p.start).unwrap_or(0),
            src_port_end: rule.src_port.map(|p| p.end).unwrap_or(0),
            dst_port_start: rule.dst_port.map(|p| p.start).unwrap_or(0),
            dst_port_end: rule.dst_port.map(|p| p.end).unwrap_or(0),
            rate_limit_pps: rule.rate_limit_pps.unwrap_or(0),
            _padding: 0,
        }
    }

    fn parse_cidr(cidr: Option<&str>) -> (u32, u32) {
        let cidr = match cidr {
            Some(c) if !c.is_empty() => c,
            _ => return (0, 0),
        };

        let parts: Vec<&str> = cidr.split('/').collect();
        let ip: std::net::Ipv4Addr = match parts[0].parse() {
            Ok(ip) => ip,
            Err(_) => return (0, 0),
        };

        let prefix_len: u8 = if parts.len() > 1 {
            parts[1].parse().unwrap_or(32)
        } else {
            32
        };

        let ip_bits = u32::from(ip);
        let mask_bits = if prefix_len == 0 {
            0u32
        } else {
            !0u32 << (32 - prefix_len)
        };

        (ip_bits.to_be(), mask_bits.to_be())
    }
}

#[cfg(target_os = "linux")]
pub use linux::*;
