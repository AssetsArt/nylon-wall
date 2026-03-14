#[cfg(feature = "std")]
use crate::protocol::{PortRange, Protocol};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum NatType {
    SNAT = 0,
    DNAT = 1,
    Masquerade = 2,
}

/// Compact NAT entry for eBPF maps
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct EbpfNatEntry {
    pub id: u32,
    pub nat_type: u8,
    pub enabled: u8,
    pub protocol: u8,
    pub _pad: u8,
    pub src_ip: u32,
    pub src_mask: u32,
    pub dst_ip: u32,
    pub dst_mask: u32,
    pub dst_port_start: u16,
    pub dst_port_end: u16,
    pub translate_ip: u32,
    pub translate_port_start: u16,
    pub translate_port_end: u16,
}

/// NAT connection state for tracking active translations (return traffic)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct EbpfNatState {
    pub original_ip: u32,
    pub original_port: u16,
    pub translated_ip: u32,
    pub translated_port: u16,
    pub nat_type: u8,
    pub _pad: [u8; 3],
}

#[cfg(feature = "aya-pod")]
unsafe impl aya::Pod for EbpfNatEntry {}

#[cfg(feature = "aya-pod")]
unsafe impl aya::Pod for EbpfNatState {}

/// Full NAT entry for userspace
#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NatEntry {
    pub id: u32,
    pub nat_type: NatType,
    pub enabled: bool,

    // Match
    pub src_network: Option<String>,
    pub dst_network: Option<String>,
    pub protocol: Option<Protocol>,
    pub dst_port: Option<PortRange>,
    pub in_interface: Option<String>,
    pub out_interface: Option<String>,

    // Translation
    pub translate_ip: Option<String>,
    pub translate_port: Option<PortRange>,
}
