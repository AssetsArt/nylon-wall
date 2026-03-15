/// Tail-call pipeline stage indices.
/// Each feature occupies a fixed slot in the ProgramArray dispatch maps.
pub const STAGE_NAT: u32 = 0;
pub const STAGE_SNI: u32 = 1;
pub const STAGE_RULES: u32 = 2;

/// Direction constants for ScratchPad.
pub const DIR_INGRESS: u8 = 0;
pub const DIR_EGRESS: u8 = 1;

/// Per-CPU scratch area shared between tail-called eBPF programs.
///
/// Stored in a `PerCpuArray<ScratchPad>` with a single entry (index 0).
/// Each tail-called stage reads/writes this struct to pass packet state
/// without relying on stack (which is reset across tail calls).
///
/// Field layout is carefully packed for alignment. Total size should stay
/// under 256 bytes to be eBPF-map-friendly.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ScratchPad {
    // === Parsed packet fields (from entry point) ===
    pub src_ip: u32,
    pub dst_ip: u32,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: u8,
    pub tcp_flags: u8,
    pub direction: u8,
    pub nat_applied: u8,
    pub pkt_len: u32,

    // === Original (pre-NAT) packet fields for conntrack ===
    pub orig_src_ip: u32,
    pub orig_dst_ip: u32,
    pub orig_src_port: u16,
    pub orig_dst_port: u16,

    // === Pipeline control ===
    /// Set to 1 when a stage has made a terminal decision (drop/pass).
    pub decided: u8,
    /// The decided action: 0 = pass, 1 = drop, 2 = reject.
    pub action: u8,
    pub should_log: u8,
    pub _pad1: u8,
    pub matched_rule_id: u32,

    // === Context from entry point ===
    pub ifindex: u32,
    pub _pad2: u32,
    pub timestamp: u64,

    // === Reserved for future use ===
    pub _reserved: [u8; 16],
}

#[cfg(feature = "aya-pod")]
unsafe impl aya::Pod for ScratchPad {}
