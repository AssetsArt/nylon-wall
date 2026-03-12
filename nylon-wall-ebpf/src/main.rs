#![no_std]
#![no_main]

mod ingress;
mod egress;
mod common;

use aya_ebpf::{
    bindings::xdp_action,
    macros::xdp,
    programs::XdpContext,
};
use aya_log_ebpf::info;

#[xdp]
pub fn nylon_wall_ingress(ctx: XdpContext) -> u32 {
    match ingress::process_ingress(&ctx) {
        Ok(action) => action,
        Err(_) => xdp_action::XDP_PASS,
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
