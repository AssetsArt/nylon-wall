#[cfg(feature = "std")]
mod inner {
    use serde::{Deserialize, Serialize};

    /// VLAN sub-interface configuration.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct VlanConfig {
        pub id: u32,
        /// Parent physical interface (e.g. "eth0").
        pub parent_interface: String,
        /// VLAN ID (1-4094).
        pub vlan_id: u16,
        /// Optional IP address in CIDR notation (e.g. "192.168.10.1/24").
        #[serde(default)]
        pub ip_address: Option<String>,
        /// Whether this VLAN is enabled.
        #[serde(default)]
        pub enabled: bool,
    }

    impl Default for VlanConfig {
        fn default() -> Self {
            Self {
                id: 0,
                parent_interface: String::new(),
                vlan_id: 10,
                ip_address: None,
                enabled: false,
            }
        }
    }

    impl VlanConfig {
        /// Linux interface name for this VLAN (e.g. "eth0.100").
        pub fn iface_name(&self) -> String {
            format!("{}.{}", self.parent_interface, self.vlan_id)
        }
    }

    /// Linux bridge configuration.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct BridgeConfig {
        pub id: u32,
        /// Bridge interface name (e.g. "br-lan").
        pub name: String,
        /// Ports (interfaces) attached to this bridge.
        #[serde(default)]
        pub ports: Vec<String>,
        /// Optional IP address in CIDR notation.
        #[serde(default)]
        pub ip_address: Option<String>,
        /// Whether STP (Spanning Tree Protocol) is enabled.
        #[serde(default)]
        pub stp_enabled: bool,
        /// Whether this bridge is enabled.
        #[serde(default)]
        pub enabled: bool,
    }

    impl Default for BridgeConfig {
        fn default() -> Self {
            Self {
                id: 0,
                name: "br0".to_string(),
                ports: Vec::new(),
                ip_address: None,
                stp_enabled: false,
                enabled: false,
            }
        }
    }
}

#[cfg(feature = "std")]
pub use inner::{BridgeConfig, VlanConfig};
