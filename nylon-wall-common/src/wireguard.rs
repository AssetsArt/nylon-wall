#[cfg(feature = "std")]
mod inner {
    use serde::{Deserialize, Serialize};

    /// WireGuard server (interface) configuration.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct WgServer {
        /// Listen port (default: 51820).
        pub listen_port: u16,
        /// Interface address (CIDR), e.g. "10.0.0.1/24".
        pub address: String,
        /// DNS servers to push to clients.
        #[serde(default)]
        pub dns: Vec<String>,
        /// Private key (base64). Auto-generated if empty.
        #[serde(default)]
        pub private_key: String,
        /// Public key (derived from private key).
        #[serde(default)]
        pub public_key: String,
        /// Interface name (default: "wg0").
        #[serde(default = "default_iface")]
        pub interface: String,
        /// Whether the VPN is enabled.
        #[serde(default)]
        pub enabled: bool,
        /// External endpoint (hostname:port) for peer configs.
        #[serde(default)]
        pub endpoint: String,
    }

    fn default_iface() -> String {
        "wg0".to_string()
    }

    impl Default for WgServer {
        fn default() -> Self {
            Self {
                listen_port: 51820,
                address: "10.0.0.1/24".to_string(),
                dns: vec!["1.1.1.1".to_string()],
                private_key: String::new(),
                public_key: String::new(),
                interface: "wg0".to_string(),
                enabled: false,
                endpoint: String::new(),
            }
        }
    }

    /// WireGuard peer configuration.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct WgPeer {
        pub id: u32,
        /// Peer name / label.
        pub name: String,
        /// Peer public key (base64).
        pub public_key: String,
        /// Peer private key (base64). Stored for config download.
        #[serde(default)]
        pub private_key: String,
        /// Preshared key (optional, base64).
        #[serde(default)]
        pub preshared_key: String,
        /// Allowed IPs for this peer, e.g. "10.0.0.2/32".
        pub allowed_ips: String,
        /// Persistent keepalive interval (seconds, 0 = disabled).
        #[serde(default)]
        pub persistent_keepalive: u16,
        /// Whether this peer is enabled.
        #[serde(default = "default_true")]
        pub enabled: bool,
    }

    fn default_true() -> bool {
        true
    }

    /// Live peer status from `wg show`.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct WgPeerStatus {
        pub public_key: String,
        pub endpoint: String,
        pub last_handshake: String,
        pub transfer_rx: u64,
        pub transfer_tx: u64,
        pub allowed_ips: String,
    }
}

#[cfg(feature = "std")]
pub use inner::{WgPeer, WgPeerStatus, WgServer};
