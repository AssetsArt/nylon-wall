#[cfg(feature = "std")]
use crate::protocol::Protocol;

#[cfg(feature = "std")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Route {
    pub id: u32,
    pub destination: String, // CIDR e.g. "10.0.0.0/8"
    pub gateway: Option<String>,
    pub interface: String,
    pub metric: u32,
    pub table: u32,
    pub enabled: bool,
}

#[cfg(feature = "std")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PolicyRoute {
    pub id: u32,
    pub src_ip: Option<String>,
    pub dst_ip: Option<String>,
    pub src_port: Option<crate::protocol::PortRange>,
    pub protocol: Option<Protocol>,
    pub route_table: u32,
    pub priority: u32,
}

/// eBPF route mark entry
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct EbpfRouteMark {
    pub src_ip: u32,
    pub src_mask: u32,
    pub dst_ip: u32,
    pub dst_mask: u32,
    pub protocol: u8,
    pub _pad: [u8; 3],
    pub fwmark: u32,
}
