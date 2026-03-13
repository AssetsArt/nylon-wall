/// DHCP server pool configuration
#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DhcpPool {
    pub id: u32,
    pub interface: String,
    pub enabled: bool,
    pub subnet: String,
    pub range_start: String,
    pub range_end: String,
    pub gateway: Option<String>,
    pub dns_servers: Vec<String>,
    pub domain_name: Option<String>,
    pub lease_time: u32,
}

/// Active DHCP lease
#[cfg(feature = "std")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DhcpLease {
    pub ip: String,
    pub mac: String,
    pub hostname: Option<String>,
    pub pool_id: u32,
    pub lease_start: i64,
    pub lease_end: i64,
    pub state: DhcpLeaseState,
}

/// State of a DHCP lease
#[cfg(feature = "std")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DhcpLeaseState {
    Active,
    Expired,
    Reserved,
}

/// Static DHCP reservation (MAC → IP mapping)
#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DhcpReservation {
    pub id: u32,
    pub pool_id: u32,
    pub mac: String,
    pub ip: String,
    pub hostname: Option<String>,
}

/// DHCP client configuration for a WAN interface
#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DhcpClientConfig {
    pub id: u32,
    pub interface: String,
    pub enabled: bool,
    pub hostname: Option<String>,
}

/// Runtime status of a DHCP client
#[cfg(feature = "std")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DhcpClientStatus {
    pub interface: String,
    pub state: DhcpClientState,
    pub ip: Option<String>,
    pub subnet_mask: Option<String>,
    pub gateway: Option<String>,
    pub dns_servers: Vec<String>,
    pub dhcp_server: Option<String>,
    pub lease_start: Option<i64>,
    pub lease_end: Option<i64>,
    pub last_renewed: Option<i64>,
}

/// State machine states for the DHCP client
#[cfg(feature = "std")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DhcpClientState {
    Idle,
    Discovering,
    Requesting,
    Bound,
    Renewing,
    Rebinding,
    Error,
}
