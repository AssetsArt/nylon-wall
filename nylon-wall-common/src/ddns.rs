#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// Supported DDNS provider types.
#[cfg(feature = "std")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DdnsProvider {
    Cloudflare,
    NoIp,
    DuckDns,
    Dynu,
    Custom,
}

/// A DDNS configuration entry.
#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DdnsEntry {
    pub id: u32,
    pub provider: DdnsProvider,
    pub hostname: String,
    /// For Cloudflare: zone_id; for others: username
    #[serde(default)]
    pub username: String,
    /// API token / password
    #[serde(default)]
    pub token: String,
    /// Custom update URL (only for Custom provider)
    #[serde(default)]
    pub custom_url: String,
    /// Check interval in seconds (default: 300 = 5 minutes)
    #[serde(default = "default_interval")]
    pub interval_secs: u64,
    pub enabled: bool,
}

#[cfg(feature = "std")]
fn default_interval() -> u64 {
    300
}

/// Runtime status of a DDNS entry.
#[cfg(feature = "std")]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DdnsStatus {
    pub id: u32,
    pub current_ip: Option<String>,
    pub last_update: Option<String>,
    pub last_error: Option<String>,
    pub update_count: u64,
}
