#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum ConnState {
    New = 0,
    Established = 1,
    Related = 2,
    Invalid = 3,
}

impl core::fmt::Display for ConnState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ConnState::New => write!(f, "New"),
            ConnState::Established => write!(f, "Established"),
            ConnState::Related => write!(f, "Related"),
            ConnState::Invalid => write!(f, "Invalid"),
        }
    }
}

/// Connection tracking key for eBPF map lookups
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ConntrackKey {
    pub src_ip: u32,
    pub dst_ip: u32,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: u8,
    pub _pad: [u8; 3],
}

/// Connection tracking entry in eBPF map
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ConntrackEntry {
    pub state: u8,
    pub _pad: [u8; 3],
    pub packets_in: u64,
    pub packets_out: u64,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub last_seen: u64,
    pub timeout: u32,
    pub _pad2: u32,
}

/// Userspace-friendly conntrack view
#[cfg(feature = "std")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConntrackInfo {
    pub src_ip: String,
    pub dst_ip: String,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: String,
    pub state: ConnState,
    pub packets_in: u64,
    pub packets_out: u64,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub last_seen: u64,
    pub timeout: u32,
}
