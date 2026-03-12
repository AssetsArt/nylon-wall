/// Ethernet header size
pub const ETH_HDR_LEN: usize = 14;
/// IPv4 header minimum size
pub const IPV4_HDR_LEN: usize = 20;
/// TCP header minimum size
pub const TCP_HDR_LEN: usize = 20;
/// UDP header size
pub const UDP_HDR_LEN: usize = 8;

/// Ethertype constants
pub const ETH_P_IP: u16 = 0x0800;
pub const ETH_P_IPV6: u16 = 0x86DD;

/// IP protocol numbers
pub const IPPROTO_TCP: u8 = 6;
pub const IPPROTO_UDP: u8 = 17;
pub const IPPROTO_ICMP: u8 = 1;
