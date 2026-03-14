#[cfg(feature = "std")]
use crate::protocol::{PortRange, Protocol};
#[cfg(feature = "std")]
use crate::rule::RuleAction;

#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Zone {
    pub id: u32,
    pub name: String,
    pub interfaces: Vec<String>,
    pub default_policy: RuleAction,
}

#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NetworkPolicy {
    pub id: u32,
    pub name: String,
    pub enabled: bool,
    pub from_zone: String,
    pub to_zone: String,

    pub src_ip: Option<String>,
    pub dst_ip: Option<String>,
    pub dst_port: Option<PortRange>,
    pub protocol: Option<Protocol>,
    pub schedule: Option<Schedule>,

    pub action: RuleAction,
    pub log: bool,
    pub priority: u32,
}

#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Schedule {
    pub days: Vec<u8>,      // 0=Mon, 6=Sun
    pub start_time: String, // "HH:MM"
    pub end_time: String,   // "HH:MM"
}

/// eBPF zone mapping (interface index -> zone id)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct EbpfZoneMapping {
    pub ifindex: u32,
    pub zone_id: u32,
}

/// eBPF policy key (zone pair)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct EbpfPolicyKey {
    pub from_zone: u32,
    pub to_zone: u32,
}

/// eBPF policy value (action for zone pair)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct EbpfPolicyValue {
    pub action: u8,  // 0=Allow, 1=Drop, 2=Log
    pub log: u8,
    pub _pad: [u8; 2],
}

#[cfg(feature = "aya-pod")]
unsafe impl aya::Pod for EbpfZoneMapping {}

#[cfg(feature = "aya-pod")]
unsafe impl aya::Pod for EbpfPolicyKey {}

#[cfg(feature = "aya-pod")]
unsafe impl aya::Pod for EbpfPolicyValue {}
