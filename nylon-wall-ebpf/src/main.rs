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
use nylon_wall_common::log::EbpfPacketEvent;
use nylon_wall_common::rule::EbpfRule;

// === eBPF Maps ===

/// Ingress firewall rules (indexed array, max 256 rules).
#[map]
static INGRESS_RULES: Array<EbpfRule> = Array::with_max_entries(256, 0);

/// Egress firewall rules (indexed array, max 256 rules).
#[map]
static EGRESS_RULES: Array<EbpfRule> = Array::with_max_entries(256, 0);

/// Number of active ingress rules (single-element array to communicate count).
#[map]
static INGRESS_RULE_COUNT: Array<u32> = Array::with_max_entries(1, 0);

/// Number of active egress rules.
#[map]
static EGRESS_RULE_COUNT: Array<u32> = Array::with_max_entries(1, 0);

/// Connection tracking table (LRU, evicts oldest on full).
#[map]
static CONNTRACK: LruHashMap<ConntrackKey, ConntrackEntry> =
    LruHashMap::with_max_entries(65536, 0);

/// Packet event log (perf ring buffer to userspace).
#[map]
static EVENTS: PerfEventArray<EbpfPacketEvent> = PerfEventArray::new(0);

/// Per-rule hit counters (indexed by rule id).
#[map]
static RULE_HITS: HashMap<u32, u64> = HashMap::with_max_entries(512, 0);

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
    loop {}
}
