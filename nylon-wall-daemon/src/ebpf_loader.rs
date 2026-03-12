//! eBPF program loader — loads bytecode, attaches to interfaces, and populates maps.

#[cfg(target_os = "linux")]
pub async fn load_and_attach() -> anyhow::Result<()> {
    use std::net::Ipv4Addr;
    use std::sync::Arc;

    use aya::maps::{Array, HashMap, MapData};
    use aya::programs::{SchedClassifier, Xdp, XdpFlags};
    use aya::{include_bytes_aligned, Ebpf};
    use nylon_wall_common::rule::{Direction, EbpfRule, RuleAction};
    use tracing::info;

    use crate::AppState;

    info!("Loading eBPF bytecode...");

    let mut bpf = Ebpf::load(include_bytes_aligned!(
        "../../target/bpfel-unknown-none/release/nylon-wall-ebpf"
    ))?;

    // Optional: initialize aya-log for eBPF-side logging
    if let Err(e) = aya_log::EbpfLogger::init(&mut bpf) {
        tracing::warn!("Failed to initialize eBPF logger: {}", e);
    }

    // Attach XDP ingress program to the default interface
    let iface = std::env::var("NYLON_WALL_IFACE").unwrap_or_else(|_| "eth0".to_string());

    let program: &mut Xdp = bpf
        .program_mut("nylon_wall_ingress")
        .ok_or_else(|| anyhow::anyhow!("XDP program not found"))?
        .try_into()?;
    program.load()?;
    program.attach(&iface, XdpFlags::default())?;
    info!("XDP ingress attached to {}", iface);

    // Attach TC egress program
    let _ = std::process::Command::new("tc")
        .args(["qdisc", "add", "dev", &iface, "clsact"])
        .output(); // Ignore error if already exists

    let tc_prog: &mut SchedClassifier = bpf
        .program_mut("nylon_wall_egress")
        .ok_or_else(|| anyhow::anyhow!("TC program not found"))?
        .try_into()?;
    tc_prog.load()?;
    tc_prog.attach(&iface, aya::programs::TcAttachType::Egress)?;
    info!("TC egress attached to {}", iface);

    // Set initial rule counts to 0
    let mut ingress_count: Array<_, u32> =
        Array::try_from(bpf.map_mut("INGRESS_RULE_COUNT").unwrap())?;
    ingress_count.set(0, 0, 0)?;

    let mut egress_count: Array<_, u32> =
        Array::try_from(bpf.map_mut("EGRESS_RULE_COUNT").unwrap())?;
    egress_count.set(0, 0, 0)?;

    info!("eBPF programs loaded and attached successfully");
    Ok(())
}

#[cfg(target_os = "linux")]
/// Push firewall rules from the daemon DB into the eBPF maps.
/// Called after loading or whenever rules change.
pub fn sync_rules_to_maps(
    bpf: &mut aya::Ebpf,
    ingress_rules: &[nylon_wall_common::rule::EbpfRule],
    egress_rules: &[nylon_wall_common::rule::EbpfRule],
) -> anyhow::Result<()> {
    use aya::maps::Array;

    let mut ingress_map: Array<_, nylon_wall_common::rule::EbpfRule> =
        Array::try_from(bpf.map_mut("INGRESS_RULES").unwrap())?;
    for (i, rule) in ingress_rules.iter().enumerate() {
        ingress_map.set(i as u32, *rule, 0)?;
    }
    let mut ingress_count: Array<_, u32> =
        Array::try_from(bpf.map_mut("INGRESS_RULE_COUNT").unwrap())?;
    ingress_count.set(0, ingress_rules.len() as u32, 0)?;

    let mut egress_map: Array<_, nylon_wall_common::rule::EbpfRule> =
        Array::try_from(bpf.map_mut("EGRESS_RULES").unwrap())?;
    for (i, rule) in egress_rules.iter().enumerate() {
        egress_map.set(i as u32, *rule, 0)?;
    }
    let mut egress_count: Array<_, u32> =
        Array::try_from(bpf.map_mut("EGRESS_RULE_COUNT").unwrap())?;
    egress_count.set(0, egress_rules.len() as u32, 0)?;

    tracing::info!(
        "Synced {} ingress + {} egress rules to eBPF maps",
        ingress_rules.len(),
        egress_rules.len()
    );
    Ok(())
}

#[cfg(target_os = "linux")]
/// Convert a userspace FirewallRule to an EbpfRule for map insertion.
pub fn firewall_rule_to_ebpf(
    rule: &nylon_wall_common::rule::FirewallRule,
) -> nylon_wall_common::rule::EbpfRule {
    use nylon_wall_common::rule::EbpfRule;

    let (src_ip, src_mask) = parse_cidr(rule.src_ip.as_deref());
    let (dst_ip, dst_mask) = parse_cidr(rule.dst_ip.as_deref());

    EbpfRule {
        id: rule.id,
        priority: rule.priority,
        direction: rule.direction as u8,
        enabled: if rule.enabled { 1 } else { 0 },
        protocol: rule
            .protocol
            .map(|p| p as u8)
            .unwrap_or(0),
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

#[cfg(target_os = "linux")]
/// Parse "192.168.1.0/24" into (ip_u32_network_order, mask_u32_network_order).
/// Returns (0, 0) for None (matches any).
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

    // Store in network byte order to match what eBPF sees from packet headers
    (ip_bits.to_be(), mask_bits.to_be())
}

#[cfg(not(target_os = "linux"))]
pub async fn load_and_attach() -> anyhow::Result<()> {
    tracing::warn!("eBPF not available on this platform");
    Ok(())
}
