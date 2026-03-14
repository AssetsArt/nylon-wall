#![no_std]
#![no_main]

mod common;
mod egress;
mod ingress;

use aya_ebpf::{
    bindings::xdp_action,
    macros::{classifier, map, xdp},
    maps::{Array, HashMap, LruHashMap, PerfEventArray},
    programs::{TcContext, XdpContext},
};

use nylon_wall_common::conntrack::{ConntrackEntry, ConntrackKey};
use nylon_wall_common::log::{EbpfMetrics, EbpfPacketEvent, EbpfRateState};
use nylon_wall_common::rule::EbpfRule;
use nylon_wall_common::zone::EbpfPolicyValue;

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

// === Metrics & Rate Limiting ===

#[map]
static METRICS: Array<EbpfMetrics> = Array::with_max_entries(1, 0);

#[map]
static RATE_LIMIT: HashMap<u32, EbpfRateState> = HashMap::with_max_entries(256, 0);

// === XDP ingress program ===

#[xdp]
pub fn nylon_wall_ingress(ctx: XdpContext) -> u32 {
    match ingress::process_ingress(&ctx) {
        Ok(action) => action,
        Err(_) => xdp_action::XDP_PASS,
    }
}

// === TC egress program ===

#[classifier]
pub fn nylon_wall_egress(ctx: TcContext) -> i32 {
    match egress::process_egress(&ctx) {
        Ok(action) => action,
        Err(_) => 0, // TC_ACT_OK (pass)
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
