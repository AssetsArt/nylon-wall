/// L4 Proxy key for eBPF map lookup
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct EbpfL4ProxyKey {
    pub protocol: u8,
    pub _pad: u8,
    pub port: u16,
    pub ip: u32,
}

/// L4 Proxy entry for eBPF map — selected upstream target
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct EbpfL4ProxyEntry {
    pub upstream_ip: u32,
    pub upstream_port: u16,
    pub enabled: u8,
    pub _pad: u8,
}

/// L4 Proxy NAT state for conntrack (return-path SNAT)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct EbpfL4ProxyNatState {
    pub original_dst_ip: u32,
    pub original_dst_port: u16,
    pub _pad: [u8; 2],
}

/// Per-rule counters for eBPF stats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct EbpfL4ProxyCounters {
    pub packets: u64,
    pub bytes: u64,
}

#[cfg(feature = "aya-pod")]
unsafe impl aya::Pod for EbpfL4ProxyKey {}
#[cfg(feature = "aya-pod")]
unsafe impl aya::Pod for EbpfL4ProxyEntry {}
#[cfg(feature = "aya-pod")]
unsafe impl aya::Pod for EbpfL4ProxyNatState {}
#[cfg(feature = "aya-pod")]
unsafe impl aya::Pod for EbpfL4ProxyCounters {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum L4Protocol {
    TCP = 6,
    UDP = 17,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum LoadBalanceMode {
    RoundRobin = 0,
    IpHash = 1,
}

#[cfg(feature = "std")]
mod inner {
    use super::*;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct UpstreamTarget {
        pub address: String,
        pub port: u16,
        #[serde(default = "default_weight")]
        pub weight: u32,
    }

    fn default_weight() -> u32 {
        1
    }

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct L4ProxyRule {
        pub id: u32,
        pub name: String,
        pub protocol: L4Protocol,
        pub listen_address: String,
        pub listen_port: u16,
        pub upstream_targets: Vec<UpstreamTarget>,
        pub load_balance: LoadBalanceMode,
        pub enabled: bool,
    }

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct L4ProxyStats {
        pub rule_id: u32,
        pub active_connections: u64,
        pub total_connections: u64,
        pub bytes_in: u64,
        pub bytes_out: u64,
    }
}

#[cfg(feature = "std")]
pub use inner::*;
