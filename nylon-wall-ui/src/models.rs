// Re-export common types for convenience
pub use nylon_wall_common::conntrack::ConntrackInfo;
pub use nylon_wall_common::ddns::{DdnsEntry, DdnsProvider, DdnsStatus};
pub use nylon_wall_common::dhcp::{
    DhcpClientConfig, DhcpClientState, DhcpClientStatus, DhcpLease, DhcpLeaseState, DhcpPool,
    DhcpReservation,
};
pub use nylon_wall_common::log::PacketLog;
pub use nylon_wall_common::nat::{NatEntry, NatType};
pub use nylon_wall_common::protocol::{PortRange, Protocol};
pub use nylon_wall_common::route::{PolicyRoute, Route};
pub use nylon_wall_common::rule::{Direction, FirewallRule, RuleAction};
pub use nylon_wall_common::tls::{SniAction, SniRule, SniStats};
pub use nylon_wall_common::mdns::MdnsConfig;
pub use nylon_wall_common::oauth::{OAuthProvider, OAuthProviderType};
pub use nylon_wall_common::wireguard::{WgPeer, WgPeerStatus, WgServer};
pub use nylon_wall_common::wol::{WolDevice, WolRequest};
pub use nylon_wall_common::zone::{NetworkPolicy, Schedule, Zone};

/// System status response from the daemon
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SystemStatus {
    pub version: String,
    pub ebpf_loaded: bool,
    pub uptime_seconds: u64,
    #[serde(default)]
    pub ebpf_programs: Vec<EbpfProgramStatus>,
}

/// Individual eBPF program status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EbpfProgramStatus {
    pub name: String,
    pub prog_type: String,
    pub role: String,
    pub stage: Option<u32>,
}
