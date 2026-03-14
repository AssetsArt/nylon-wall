#[cfg(feature = "std")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PacketLog {
    pub timestamp: i64,
    pub src_ip: String,
    pub dst_ip: String,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: String,
    pub action: String,
    pub rule_id: u32,
    pub interface: String,
    pub bytes: u32,
}

/// eBPF perf event structure for packet logging
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EbpfPacketEvent {
    pub timestamp: u64,
    pub src_ip: u32,
    pub dst_ip: u32,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: u8,
    pub action: u8,
    pub rule_id: u32,
    pub ifindex: u32,
    pub bytes: u32,
}

#[cfg(feature = "aya-pod")]
unsafe impl aya::Pod for EbpfPacketEvent {}

/// Global metrics counters for eBPF (single entry in PerCpuArray)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EbpfMetrics {
    pub packets_total: u64,
    pub bytes_total: u64,
    pub packets_dropped: u64,
    pub packets_allowed: u64,
    pub packets_nat: u64,
    pub _pad: u64,
}

/// Per-rule rate limiting state (token bucket)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EbpfRateState {
    pub tokens: u64,
    pub last_update: u64,
}

#[cfg(feature = "aya-pod")]
unsafe impl aya::Pod for EbpfMetrics {}

#[cfg(feature = "aya-pod")]
unsafe impl aya::Pod for EbpfRateState {}

#[cfg(feature = "std")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricPoint {
    pub name: String,
    pub value: f64,
    pub timestamp: i64,
    pub labels: std::collections::HashMap<String, String>,
}
