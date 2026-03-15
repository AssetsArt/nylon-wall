use aya_ebpf::programs::XdpContext;
use aya_ebpf::bindings::xdp_action;
use nylon_wall_common::scratchpad::STAGE_RULES;

use crate::common::*;
use crate::scratchpad::read_scratch;

const XDP_PASS: u32 = xdp_action::XDP_PASS;
const XDP_DROP: u32 = xdp_action::XDP_DROP;

/// Ingress SNI filtering stage (XDP tail call target).
///
/// Checks TLS ClientHello for blocked SNI domains.
/// If blocked, marks decided+action and returns DROP.
/// Otherwise tail-calls to Rules stage.
pub fn process(ctx: &XdpContext) -> Result<u32, ()> {
    let scratch = match read_scratch() {
        Some(s) => s,
        None => return Ok(XDP_PASS),
    };

    // Skip if already decided by a previous stage
    let decided = unsafe { (*scratch).decided };
    if decided != 0 {
        unsafe { crate::XDP_DISPATCH.tail_call(ctx, STAGE_RULES).ok(); }
        return Ok(XDP_PASS);
    }

    let sni_enabled = unsafe { crate::SNI_ENABLED.get(0).copied().unwrap_or(0) };
    let protocol = unsafe { (*scratch).protocol };
    let dst_port = unsafe { (*scratch).dst_port };

    if sni_enabled == 1 && protocol == IPPROTO_TCP && dst_port == 443 {
        let data = ctx.data();
        let data_end = ctx.data_end();

        let ip_base = data + ETH_HDR_LEN;
        if ip_base + 1 <= data_end {
            let ihl = unsafe { (*((ip_base) as *const u8) & 0x0F) as usize * 4 };
            let transport_base = ip_base + ihl;

            if crate::tls::check_sni_block(data, data_end, transport_base) {
                // SNI blocked — mark decision
                unsafe {
                    (*scratch).decided = 1;
                    (*scratch).action = 1; // DROP
                }
                if let Some(m) = unsafe { crate::METRICS.get_ptr_mut(0) } {
                    unsafe { (*m).packets_dropped += 1 };
                }
                return Ok(XDP_DROP);
            }
        }
    }

    // Cascade to Rules
    unsafe { crate::XDP_DISPATCH.tail_call(ctx, STAGE_RULES).ok(); }
    Ok(XDP_PASS)
}
