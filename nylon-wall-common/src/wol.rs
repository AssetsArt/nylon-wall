#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// A saved Wake-on-LAN device.
#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WolDevice {
    pub id: u32,
    pub name: String,
    /// MAC address in "aa:bb:cc:dd:ee:ff" format.
    pub mac: String,
    /// Optional broadcast interface (e.g. "eth0").
    #[serde(default)]
    pub interface: String,
    /// Last wake timestamp (RFC 3339).
    #[serde(default)]
    pub last_wake: Option<String>,
}

/// Request to send a WOL magic packet.
#[cfg(feature = "std")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WolRequest {
    pub mac: String,
    #[serde(default)]
    pub interface: String,
}
