use nylon_wall_common::scratchpad::ScratchPad;

use crate::common::PacketInfo;

/// Write parsed packet info into the per-CPU scratch area.
/// Called by the entry-point program before tail-calling into stages.
#[inline(always)]
pub fn write_scratch(
    pkt: &PacketInfo,
    direction: u8,
    ifindex: u32,
    ts: u64,
) -> Option<*mut ScratchPad> {
    let ptr = unsafe { crate::SCRATCH.get_ptr_mut(0)? };
    unsafe {
        (*ptr).src_ip = pkt.src_ip;
        (*ptr).dst_ip = pkt.dst_ip;
        (*ptr).src_port = pkt.src_port;
        (*ptr).dst_port = pkt.dst_port;
        (*ptr).protocol = pkt.protocol;
        (*ptr).tcp_flags = pkt.tcp_flags;
        (*ptr).direction = direction;
        (*ptr).nat_applied = 0;
        (*ptr).pkt_len = pkt.pkt_len;
        // Original fields default to current (overwritten after NAT)
        (*ptr).orig_src_ip = pkt.src_ip;
        (*ptr).orig_dst_ip = pkt.dst_ip;
        (*ptr).orig_src_port = pkt.src_port;
        (*ptr).orig_dst_port = pkt.dst_port;
        // Pipeline control
        (*ptr).decided = 0;
        (*ptr).action = 0;
        (*ptr).should_log = 0;
        (*ptr)._pad1 = 0;
        (*ptr).matched_rule_id = 0;
        // Context
        (*ptr).ifindex = ifindex;
        (*ptr)._pad2 = 0;
        (*ptr).timestamp = ts;
    }
    Some(ptr)
}

/// Read the per-CPU scratch area. Returns a mutable pointer for stages.
#[inline(always)]
pub fn read_scratch() -> Option<*mut ScratchPad> {
    unsafe { crate::SCRATCH.get_ptr_mut(0) }
}
