#![no_std]
#![no_main]

mod common;
mod nat;
mod scratchpad;
mod stages;
mod tls;

use aya_ebpf::{
    bindings::xdp_action,
    macros::{classifier, map, xdp},
    maps::{Array, HashMap, LruHashMap, PerCpuArray, PerfEventArray, ProgramArray},
    programs::{TcContext, XdpContext},
};

use nylon_wall_common::conntrack::{ConntrackEntry, ConntrackKey};
use nylon_wall_common::log::{EbpfMetrics, EbpfPacketEvent, EbpfRateState};
use nylon_wall_common::nat::{EbpfNatEntry, EbpfNatState};
use nylon_wall_common::rule::EbpfRule;
use nylon_wall_common::scratchpad::ScratchPad;
use nylon_wall_common::tls::EbpfSniEvent;
use nylon_wall_common::zone::EbpfPolicyValue;

use crate::common::*;

// === Firewall Rule Maps ===

#[map]
static INGRESS_RULES: Array<EbpfRule> = Array::with_max_entries(256, 0);

#[map]
static EGRESS_RULES: Array<EbpfRule> = Array::with_max_entries(256, 0);

#[map]
static INGRESS_RULE_COUNT: Array<u32> = Array::with_max_entries(1, 0);

#[map]
static EGRESS_RULE_COUNT: Array<u32> = Array::with_max_entries(1, 0);

// === Connection Tracking ===

#[map]
static CONNTRACK: LruHashMap<ConntrackKey, ConntrackEntry> =
    LruHashMap::with_max_entries(65536, 0);

// === Event & Logging ===

#[map]
static EVENTS: PerfEventArray<EbpfPacketEvent> = PerfEventArray::new(0);

#[map]
static RULE_HITS: HashMap<u32, u64> = HashMap::with_max_entries(512, 0);

// === Zone & Policy ===

#[map]
static ZONE_MAP: HashMap<u32, u32> = HashMap::with_max_entries(64, 0);

#[map]
static POLICY_MAP: HashMap<u32, EbpfPolicyValue> = HashMap::with_max_entries(256, 0);

// === NAT Maps ===

#[map]
static NAT_TABLE: Array<EbpfNatEntry> = Array::with_max_entries(64, 0);

#[map]
static NAT_ENTRY_COUNT: Array<u32> = Array::with_max_entries(1, 0);

#[map]
static NAT_CONNTRACK: LruHashMap<ConntrackKey, EbpfNatState> =
    LruHashMap::with_max_entries(16384, 0);

#[map]
static MASQUERADE_IP: Array<u32> = Array::with_max_entries(1, 0);

// === SNI Filtering ===

#[map]
static SNI_POLICY: HashMap<u32, u8> = HashMap::with_max_entries(16384, 0);

#[map]
static SNI_EVENTS: PerfEventArray<EbpfSniEvent> = PerfEventArray::new(0);

#[map]
static SNI_ENABLED: Array<u32> = Array::with_max_entries(1, 0);

// === Metrics & Rate Limiting ===

#[map]
static METRICS: Array<EbpfMetrics> = Array::with_max_entries(1, 0);

#[map]
static RATE_LIMIT: HashMap<u32, EbpfRateState> = HashMap::with_max_entries(256, 0);

// === Tail-Call Dispatch Maps ===

/// Per-CPU scratch area for passing state between tail-called programs.
#[map]
static SCRATCH: PerCpuArray<ScratchPad> = PerCpuArray::with_max_entries(1, 0);

/// XDP dispatch table: STAGE_NAT=0, STAGE_SNI=1, STAGE_RULES=2
#[map]
static XDP_DISPATCH: ProgramArray = ProgramArray::with_max_entries(8, 0);

/// TC dispatch table: STAGE_NAT=0, STAGE_SNI=1, STAGE_RULES=2
#[map]
static TC_DISPATCH: ProgramArray = ProgramArray::with_max_entries(8, 0);

// === XDP Entry Point ===

#[xdp]
pub fn nylon_wall_ingress(ctx: XdpContext) -> u32 {
    match process_ingress_entry(&ctx) {
        Ok(action) => action,
        Err(_) => xdp_action::XDP_PASS,
    }
}

/// Ingress entry point: parse packet, populate scratch, dispatch via tail call.
fn process_ingress_entry(ctx: &XdpContext) -> Result<u32, ()> {
    let data = ctx.data();
    let data_end = ctx.data_end();

    let pkt = match parse_packet(data, data_end) {
        Some(p) => p,
        None => return Ok(xdp_action::XDP_PASS),
    };

    let now = unsafe { aya_ebpf::helpers::bpf_ktime_get_ns() };
    let ifindex = unsafe { (*ctx.ctx).ingress_ifindex };

    // Update global metrics
    if let Some(m) = unsafe { METRICS.get_ptr_mut(0) } {
        unsafe {
            (*m).packets_total += 1;
            (*m).bytes_total += pkt.pkt_len as u64;
        }
    }

    // Write scratch pad for tail-called stages
    scratchpad::write_scratch(
        &pkt,
        nylon_wall_common::scratchpad::DIR_INGRESS,
        ifindex,
        now,
    );

    // Dispatch: try NAT → SNI → Rules → fallback PASS
    unsafe {
        XDP_DISPATCH.tail_call(ctx, nylon_wall_common::scratchpad::STAGE_NAT).ok();
        XDP_DISPATCH.tail_call(ctx, nylon_wall_common::scratchpad::STAGE_SNI).ok();
        XDP_DISPATCH.tail_call(ctx, nylon_wall_common::scratchpad::STAGE_RULES).ok();
    }
    Ok(xdp_action::XDP_PASS)
}

// === TC Entry Point ===

#[classifier]
pub fn nylon_wall_egress(ctx: TcContext) -> i32 {
    match process_egress_entry(&ctx) {
        Ok(action) => action,
        Err(_) => 0, // TC_ACT_OK
    }
}

/// Egress entry point: parse packet, save orig fields, populate scratch, dispatch.
fn process_egress_entry(ctx: &TcContext) -> Result<i32, ()> {
    let data = ctx.data();
    let data_end = ctx.data_end();

    let pkt = match parse_packet(data, data_end) {
        Some(p) => p,
        None => return Ok(0), // TC_ACT_OK
    };

    let now = unsafe { aya_ebpf::helpers::bpf_ktime_get_ns() };

    // Update global metrics
    if let Some(m) = unsafe { METRICS.get_ptr_mut(0) } {
        unsafe {
            (*m).packets_total += 1;
            (*m).bytes_total += pkt.pkt_len as u64;
        }
    }

    // Write scratch pad — orig_* fields are set to current (pre-NAT) values
    // by write_scratch. The NAT stage will update post-NAT fields if NAT is applied.
    scratchpad::write_scratch(
        &pkt,
        nylon_wall_common::scratchpad::DIR_EGRESS,
        0, // egress has no meaningful ifindex
        now,
    );

    // Dispatch: try NAT → SNI → Rules → fallback OK
    unsafe {
        TC_DISPATCH.tail_call(ctx, nylon_wall_common::scratchpad::STAGE_NAT).ok();
        TC_DISPATCH.tail_call(ctx, nylon_wall_common::scratchpad::STAGE_SNI).ok();
        TC_DISPATCH.tail_call(ctx, nylon_wall_common::scratchpad::STAGE_RULES).ok();
    }
    Ok(0) // TC_ACT_OK
}

// === XDP Tail Call Targets ===

#[xdp]
pub fn ingress_nat(ctx: XdpContext) -> u32 {
    match stages::ingress_nat::process(&ctx) {
        Ok(action) => action,
        Err(_) => xdp_action::XDP_PASS,
    }
}

#[xdp]
pub fn ingress_sni(ctx: XdpContext) -> u32 {
    match stages::ingress_sni::process(&ctx) {
        Ok(action) => action,
        Err(_) => xdp_action::XDP_PASS,
    }
}

#[xdp]
pub fn ingress_rules(ctx: XdpContext) -> u32 {
    match stages::ingress_rules::process(&ctx) {
        Ok(action) => action,
        Err(_) => xdp_action::XDP_PASS,
    }
}

// === TC Tail Call Targets ===

#[classifier]
pub fn egress_nat(ctx: TcContext) -> i32 {
    match stages::egress_nat::process(&ctx) {
        Ok(action) => action,
        Err(_) => 0, // TC_ACT_OK
    }
}

#[classifier]
pub fn egress_sni(ctx: TcContext) -> i32 {
    match stages::egress_sni::process(&ctx) {
        Ok(action) => action,
        Err(_) => 0, // TC_ACT_OK
    }
}

#[classifier]
pub fn egress_rules(ctx: TcContext) -> i32 {
    match stages::egress_rules::process(&ctx) {
        Ok(action) => action,
        Err(_) => 0, // TC_ACT_OK
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
