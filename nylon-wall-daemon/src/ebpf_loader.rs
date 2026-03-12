//! eBPF program loader — loads bytecode, attaches to interfaces, and populates maps.

#[cfg(target_os = "linux")]
mod linux {
    use aya::maps::Array;
    use aya::programs::{SchedClassifier, Xdp, XdpFlags};
    use aya::Ebpf;
    use nylon_wall_common::rule::EbpfRule;
    use tracing::info;

    const EBPF_OBJ_PATH: &str = "/usr/lib/nylon-wall/nylon-wall-ebpf";

    pub async fn load_and_attach() -> anyhow::Result<()> {
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

    fn write_rules_to_map(bpf: &mut Ebpf, map_name: &str, rules: &[EbpfRule]) -> anyhow::Result<()> {
        let map = bpf.map_mut(map_name)
            .ok_or_else(|| anyhow::anyhow!("Map {} not found", map_name))?;
        let mut array: Array<_, EbpfRule> = Array::try_from(map)?;

        for (i, rule) in rules.iter().enumerate() {
            array.set(i as u32, *rule, 0)?;
        }
        Ok(())
    }

    fn set_map_u32(bpf: &mut Ebpf, map_name: &str, index: u32, value: u32) -> anyhow::Result<()> {
        let map = bpf.map_mut(map_name)
            .ok_or_else(|| anyhow::anyhow!("Map {} not found", map_name))?;
        let mut array: Array<_, u32> = Array::try_from(map)?;
        array.set(index, value, 0)?;
        Ok(())
    }

    /// Convert a userspace FirewallRule to an EbpfRule.
    pub fn firewall_rule_to_ebpf(
        rule: &nylon_wall_common::rule::FirewallRule,
    ) -> EbpfRule {
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

#[cfg(not(target_os = "linux"))]
pub async fn load_and_attach() -> anyhow::Result<()> {
    tracing::warn!("eBPF not available on this platform");
    Ok(())
}
