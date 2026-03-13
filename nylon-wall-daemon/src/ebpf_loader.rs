//! eBPF program loader — loads bytecode, attaches to interfaces, and populates maps.

#[cfg(target_os = "linux")]
mod linux {
    use aya::Ebpf;
    use aya::maps::Array;
    use aya::programs::{SchedClassifier, Xdp, XdpFlags};
    use nylon_wall_common::rule::EbpfRule;
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

        if let Err(e) = aya_log::EbpfLogger::init(&mut bpf) {
            tracing::warn!("Failed to initialize eBPF logger: {}", e);
        }

        let iface = std::env::var("NYLON_WALL_IFACE").unwrap_or_else(|_| "eth0".to_string());

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

        // Set initial rule counts to 0
        set_map_u32(&mut bpf, "INGRESS_RULE_COUNT", 0, 0)?;
        set_map_u32(&mut bpf, "EGRESS_RULE_COUNT", 0, 0)?;

        info!("eBPF programs loaded and attached successfully");
        Ok(bpf)
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
