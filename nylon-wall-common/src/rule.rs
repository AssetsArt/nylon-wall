#[cfg(feature = "std")]
use crate::protocol::{Protocol, PortRange};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum Direction {
    Ingress = 0,
    Egress = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum Action {
    Allow = 0,
    Drop = 1,
    Log = 2,
    RateLimit = 3,
}

/// Compact rule representation for eBPF maps (fixed-size, repr(C))
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct EbpfRule {
    pub id: u32,
    pub priority: u32,
    pub direction: u8,
    pub enabled: u8,
    pub protocol: u8,
    pub action: u8,
    pub src_ip: u32,       // IPv4 in network byte order, 0 = any
    pub src_mask: u32,     // CIDR mask
    pub dst_ip: u32,
    pub dst_mask: u32,
    pub src_port_start: u16,
    pub src_port_end: u16,
    pub dst_port_start: u16,
    pub dst_port_end: u16,
    pub rate_limit_pps: u32,
    pub _padding: u32,
}

/// Full rule representation for userspace (with String fields, serde)
#[cfg(feature = "std")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FirewallRule {
    pub id: u32,
    pub name: String,
    pub priority: u32,
    pub direction: Direction,
    pub enabled: bool,

    // Match conditions
    pub src_ip: Option<String>,      // CIDR notation e.g. "192.168.1.0/24"
    pub dst_ip: Option<String>,
    pub src_port: Option<PortRange>,
    pub dst_port: Option<PortRange>,
    pub protocol: Option<Protocol>,
    pub interface: Option<String>,

    // Action
    pub action: Action,
    pub rate_limit_pps: Option<u32>,

    // Metadata
    pub hit_count: u64,
    pub created_at: i64,
    pub updated_at: i64,
}
