use aya_ebpf::programs::TcContext;
use nylon_wall_common::scratchpad::{STAGE_SNI, STAGE_RULES};

use crate::common::*;
use crate::scratchpad::read_scratch;

const TC_ACT_OK: i32 = 0;

/// Egress NAT stage (TC tail call target).
///
/// 1. Read ScratchPad (orig_* fields already saved by entry point)
/// 2. Apply reverse-DNAT and SNAT/Masquerade
/// 3. Update ScratchPad with post-NAT state
/// 4. Tail call to SNI stage → Rules stage → fallback OK
pub fn process(ctx: &TcContext) -> Result<i32, ()> {
    let scratch = match read_scratch() {
        Some(s) => s,
        None => return Ok(TC_ACT_OK),
    };

    let data = ctx.data();
    let data_end = ctx.data_end();

    let pkt = match parse_packet(data, data_end) {
        Some(p) => p,
        None => return Ok(TC_ACT_OK),
    };

    // Apply NAT
    let nat_applied = crate::nat::try_reverse_nat_egress(data, data_end, &pkt)
        || crate::nat::try_snat_egress(data, data_end, &pkt);

    if nat_applied {
        // Re-parse packet after NAT rewrite
        let pkt = match parse_packet(data, data_end) {
            Some(p) => p,
            None => return Ok(TC_ACT_OK),
        };

        // Update ScratchPad with post-NAT fields
        unsafe {
            (*scratch).src_ip = pkt.src_ip;
            (*scratch).dst_ip = pkt.dst_ip;
            (*scratch).src_port = pkt.src_port;
            (*scratch).dst_port = pkt.dst_port;
            (*scratch).nat_applied = 1;
        }
    }

    // Cascade: try SNI → Rules → fallback OK
    unsafe {
        crate::TC_DISPATCH.tail_call(ctx, STAGE_SNI).ok();
        crate::TC_DISPATCH.tail_call(ctx, STAGE_RULES).ok();
    }
    Ok(TC_ACT_OK)
}
