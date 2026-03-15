use aya_ebpf::programs::XdpContext;
use aya_ebpf::bindings::xdp_action;
use nylon_wall_common::scratchpad::{STAGE_SNI, STAGE_RULES};

use crate::common::*;
use crate::scratchpad::read_scratch;

const XDP_PASS: u32 = xdp_action::XDP_PASS;

/// Ingress NAT stage (XDP tail call target).
///
/// 1. Read ScratchPad
/// 2. Apply reverse-SNAT and DNAT
/// 3. Update ScratchPad with post-NAT state
/// 4. Tail call to SNI stage → Rules stage → fallback PASS
pub fn process(ctx: &XdpContext) -> Result<u32, ()> {
    let scratch = match read_scratch() {
        Some(s) => s,
        None => return Ok(XDP_PASS),
    };

    let data = ctx.data();
    let data_end = ctx.data_end();

    // Parse current packet for NAT
    let pkt = match parse_packet(data, data_end) {
        Some(p) => p,
        None => return Ok(XDP_PASS),
    };

    // Apply NAT
    let nat_applied = crate::nat::try_reverse_nat_ingress(data, data_end, &pkt)
        || crate::nat::try_dnat_ingress(data, data_end, &pkt);

    if nat_applied {
        // Re-parse packet after NAT rewrite
        let pkt = match parse_packet(data, data_end) {
            Some(p) => p,
            None => return Ok(XDP_PASS),
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

    // Cascade: try SNI → Rules → fallback PASS
    unsafe {
        crate::XDP_DISPATCH.tail_call(ctx, STAGE_SNI).ok();
        crate::XDP_DISPATCH.tail_call(ctx, STAGE_RULES).ok();
    }
    Ok(XDP_PASS)
}
