/// Maximum domain name length stored in SNI events
pub const SNI_MAX_DOMAIN_LEN: usize = 64;

/// eBPF perf event for SNI-based TLS filtering
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EbpfSniEvent {
    pub timestamp: u64,
    pub src_ip: u32,
    pub dst_ip: u32,
    pub src_port: u16,
    pub dst_port: u16,
    pub domain_hash: u32,
    pub action: u8, // 0=allow, 1=block, 2=log
    pub domain_len: u8,
    pub _pad: [u8; 2],
    pub domain: [u8; SNI_MAX_DOMAIN_LEN], // first N bytes of domain name
}

#[cfg(feature = "aya-pod")]
unsafe impl aya::Pod for EbpfSniEvent {}

/// SNI policy entry in userspace
#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SniRule {
    pub id: u32,
    pub domain: String,   // e.g. "facebook.com" or "*.tiktok.com"
    pub action: SniAction, // Block, Allow, Log
    pub enabled: bool,
    pub hit_count: u64,
    pub category: Option<String>, // e.g. "social", "ads", "malware"
}

#[cfg(feature = "std")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SniAction {
    Allow,
    Block,
    Log,
}

/// SNI log entry stored in database
#[cfg(feature = "std")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SniLog {
    pub timestamp: i64,
    pub src_ip: String,
    pub dst_ip: String,
    pub src_port: u16,
    pub dst_port: u16,
    pub domain: String,
    pub action: String,
}

/// SNI filtering statistics
#[cfg(feature = "std")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SniStats {
    pub total_inspected: u64,
    pub total_blocked: u64,
    pub total_allowed: u64,
    pub total_logged: u64,
    pub enabled: bool,
}
