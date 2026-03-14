// Re-export common types for convenience
pub use nylon_wall_common::conntrack::ConntrackInfo;
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
pub use nylon_wall_common::zone::{NetworkPolicy, Schedule, Zone};

/// System status response from the daemon
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SystemStatus {
    pub version: String,
    pub ebpf_loaded: bool,
    pub uptime_seconds: u64,
}
