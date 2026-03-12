// === Ethernet / IP / Transport constants ===
pub const ETH_HDR_LEN: usize = 14;
pub const IPV4_HDR_LEN: usize = 20;
pub const TCP_HDR_LEN: usize = 20;
pub const UDP_HDR_LEN: usize = 8;

pub const ETH_P_IP: u16 = 0x0800;
pub const ETH_P_IPV6: u16 = 0x86DD;

pub const IPPROTO_ICMP: u8 = 1;
pub const IPPROTO_TCP: u8 = 6;
pub const IPPROTO_UDP: u8 = 17;

// Maximum rules to evaluate per packet (bounded loop for eBPF verifier)
pub const MAX_RULES: u32 = 256;

// === Packet parsing result ===

/// Parsed packet header information extracted in the XDP/TC programs.
#[derive(Clone, Copy)]
pub struct PacketInfo {
    pub src_ip: u32,
    pub dst_ip: u32,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: u8,
    pub pkt_len: u32,
}

// === Inline helpers ===

/// Check if an IP matches a rule's IP/mask pair. mask=0 means "any".
#[inline(always)]
pub fn ip_match(packet_ip: u32, rule_ip: u32, rule_mask: u32) -> bool {
    if rule_mask == 0 {
        return true;
    }
    (packet_ip & rule_mask) == (rule_ip & rule_mask)
}

/// Check if a port falls within [start, end]. start=0 && end=0 means "any".
#[inline(always)]
pub fn port_match(port: u16, start: u16, end: u16) -> bool {
    if start == 0 && end == 0 {
        return true;
    }
    port >= start && port <= end
}

/// Parse Ethernet + IPv4 + TCP/UDP headers from XDP context.
/// Returns None if not IPv4 or packet is too short.
#[inline(always)]
pub fn parse_packet(data: usize, data_end: usize) -> Option<PacketInfo> {
    // Need at least Ethernet + IPv4 header
    if data + ETH_HDR_LEN + IPV4_HDR_LEN > data_end {
        return None;
    }

    let eth_proto = unsafe {
        let ptr = (data + 12) as *const [u8; 2];
        u16::from_be_bytes(*ptr)
    };

    if eth_proto != ETH_P_IP {
        return None; // Not IPv4, skip
    }

    let ip_base = data + ETH_HDR_LEN;

    let protocol = unsafe { *((ip_base + 9) as *const u8) };
    let total_len = unsafe {
        let ptr = (ip_base + 2) as *const [u8; 2];
        u16::from_be_bytes(*ptr) as u32
    };
    let src_ip = unsafe { *((ip_base + 12) as *const u32) };
    let dst_ip = unsafe { *((ip_base + 16) as *const u32) };

    // IP header length (IHL field, lower 4 bits of first byte, in 32-bit words)
    let ihl = unsafe { (*((ip_base) as *const u8) & 0x0F) as usize * 4 };
    if ihl < IPV4_HDR_LEN {
        return None;
    }

    let transport_base = ip_base + ihl;
    let mut src_port: u16 = 0;
    let mut dst_port: u16 = 0;

    if protocol == IPPROTO_TCP {
        if transport_base + TCP_HDR_LEN > data_end {
            return None;
        }
        unsafe {
            src_port = u16::from_be_bytes(*((transport_base) as *const [u8; 2]));
            dst_port = u16::from_be_bytes(*((transport_base + 2) as *const [u8; 2]));
        }
    } else if protocol == IPPROTO_UDP {
        if transport_base + UDP_HDR_LEN > data_end {
            return None;
        }
        unsafe {
            src_port = u16::from_be_bytes(*((transport_base) as *const [u8; 2]));
            dst_port = u16::from_be_bytes(*((transport_base + 2) as *const [u8; 2]));
        }
    }
    // ICMP: ports stay 0

    Some(PacketInfo {
        src_ip,
        dst_ip,
        src_port,
        dst_port,
        protocol,
        pkt_len: total_len,
    })
}
