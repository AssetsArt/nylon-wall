#[cfg(feature = "std")]
mod inner {
    use serde::{Deserialize, Serialize};

    /// mDNS reflector configuration.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct MdnsConfig {
        /// Whether the reflector is enabled.
        pub enabled: bool,
        /// Interfaces to reflect mDNS packets between.
        pub interfaces: Vec<String>,
    }

    impl Default for MdnsConfig {
        fn default() -> Self {
            Self {
                enabled: false,
                interfaces: Vec::new(),
            }
        }
    }
}

#[cfg(feature = "std")]
pub use inner::MdnsConfig;
