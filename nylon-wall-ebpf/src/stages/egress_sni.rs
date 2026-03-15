use aya_ebpf::programs::TcContext;
use nylon_wall_common::scratchpad::STAGE_RULES;

use crate::common::*;
use crate::scratchpad::read_scratch;

const TC_ACT_OK: i32 = 0;
const TC_ACT_SHOT: i32 = 2;

/// Egress SNI filtering stage (TC tail call target).
///
/// CRITICAL: TC skb may have TCP payload in non-linear fragments.
/// Must call `ctx.pull_data(512)` to linearize before TLS parsing,
/// then re-read `ctx.data()/ctx.data_end()`.
pub fn process(ctx: &TcContext) -> Result<i32, ()> {
    let scratch = match read_scratch() {
        Some(s) => s,
        None => return Ok(TC_ACT_OK),
    };

    // Skip if already decided
    let decided = unsafe { (*scratch).decided };
    if decided != 0 {
        unsafe { crate::TC_DISPATCH.tail_call(ctx, STAGE_RULES).ok(); }
        return Ok(TC_ACT_OK);
    }

    let sni_enabled = unsafe { crate::SNI_ENABLED.get(0).copied().unwrap_or(0) };
    let protocol = unsafe { (*scratch).protocol };
    let dst_port = unsafe { (*scratch).dst_port };

    if sni_enabled == 1 && protocol == IPPROTO_TCP && dst_port == 443 {
        // Pull first 512 bytes into linear buffer for TLS parsing
        let _ = ctx.pull_data(512);
        // Re-read data pointers — pull_data may reallocate the buffer
        let data = ctx.data();
        let data_end = ctx.data_end();

        let ip_base = data + ETH_HDR_LEN;
        if ip_base + 1 <= data_end {
            let ihl = unsafe { (*((ip_base) as *const u8) & 0x0F) as usize * 4 };
            let transport_base = ip_base + ihl;

            if crate::tls::check_sni_block(data, data_end, transport_base) {
                // SNI blocked
                unsafe {
                    (*scratch).decided = 1;
                    (*scratch).action = 1; // DROP
                }
                if let Some(m) = unsafe { crate::METRICS.get_ptr_mut(0) } {
                    unsafe { (*m).packets_dropped += 1 };
                }
                return Ok(TC_ACT_SHOT);
            }
        }
    }

    // Cascade to Rules
    unsafe { crate::TC_DISPATCH.tail_call(ctx, STAGE_RULES).ok(); }
    Ok(TC_ACT_OK)
}
