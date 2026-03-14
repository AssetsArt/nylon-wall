//! NAT rule management — converts userspace NatEntry to eBPF map entries.

#[cfg(target_os = "linux")]
use nylon_wall_common::nat::EbpfNatEntry;

/// Convert a userspace NatEntry to an EbpfNatEntry for eBPF maps.
#[cfg(target_os = "linux")]
pub fn nat_entry_to_ebpf(entry: &nylon_wall_common::nat::NatEntry) -> EbpfNatEntry {
    let (src_ip, src_mask) = parse_cidr(entry.src_network.as_deref());
    let (dst_ip, dst_mask) = parse_cidr(entry.dst_network.as_deref());

    let translate_ip = entry
        .translate_ip
        .as_deref()
        .and_then(|ip| ip.parse::<std::net::Ipv4Addr>().ok())
        .map(|ip| u32::from(ip).to_be())
        .unwrap_or(0);

    EbpfNatEntry {
        id: entry.id,
        nat_type: entry.nat_type as u8,
        enabled: if entry.enabled { 1 } else { 0 },
        protocol: entry
            .protocol
            .map(|p| p as u8)
            .unwrap_or(0),
        _pad: 0,
        src_ip,
        src_mask,
        dst_ip,
        dst_mask,
        dst_port_start: entry.dst_port.map(|p| p.start).unwrap_or(0),
        dst_port_end: entry.dst_port.map(|p| p.end).unwrap_or(0),
        translate_ip,
        translate_port_start: entry.translate_port.map(|p| p.start).unwrap_or(0),
        translate_port_end: entry.translate_port.map(|p| p.end).unwrap_or(0),
    }
}

#[cfg(target_os = "linux")]
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
