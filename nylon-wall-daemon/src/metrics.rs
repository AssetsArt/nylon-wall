use std::sync::Arc;

use crate::AppState;
use nylon_wall_common::conntrack::ConntrackInfo;
use nylon_wall_common::log::PacketLog;
use nylon_wall_common::nat::NatEntry;
use nylon_wall_common::route::Route;
use nylon_wall_common::rule::FirewallRule;
use nylon_wall_common::zone::{NetworkPolicy, Zone};

/// Collect all metrics in Prometheus text exposition format.
pub async fn collect(state: &Arc<AppState>) -> String {
    let mut out = String::with_capacity(2048);

    // Uptime
    let uptime = state.started_at.elapsed().as_secs();
    prom_gauge(&mut out, "nylon_wall_uptime_seconds", "Daemon uptime in seconds", uptime);

    // eBPF loaded
    let ebpf: u64 = if cfg!(target_os = "linux") { 1 } else { 0 };
    prom_gauge(&mut out, "nylon_wall_ebpf_loaded", "Whether eBPF programs are loaded (1=yes, 0=no)", ebpf);

    // Resource counts from DB
    let rules = count_prefix::<FirewallRule>(state, "rule:").await;
    let nat = count_prefix::<NatEntry>(state, "nat:").await;
    let routes = count_prefix::<Route>(state, "route:").await;
    let zones = count_prefix::<Zone>(state, "zone:").await;
    let policies = count_prefix::<NetworkPolicy>(state, "policy:").await;
    let logs = count_prefix::<PacketLog>(state, "log:").await;
    let conntrack = count_prefix::<ConntrackInfo>(state, "conntrack:").await;

    prom_gauge(&mut out, "nylon_wall_rules_total", "Total firewall rules", rules);
    prom_gauge(&mut out, "nylon_wall_nat_entries_total", "Total NAT entries", nat);
    prom_gauge(&mut out, "nylon_wall_routes_total", "Total routes", routes);
    prom_gauge(&mut out, "nylon_wall_zones_total", "Total zones", zones);
    prom_gauge(&mut out, "nylon_wall_policies_total", "Total network policies", policies);
    prom_gauge(&mut out, "nylon_wall_logs_total", "Total packet log entries", logs);
    prom_gauge(&mut out, "nylon_wall_conntrack_entries", "Active connection tracking entries", conntrack);

    // WebSocket subscribers
    let ws_subs = state.event_tx.receiver_count() as u64;
    prom_gauge(&mut out, "nylon_wall_ws_subscribers", "Current WebSocket subscribers", ws_subs);

    out
}

async fn count_prefix<T: serde::de::DeserializeOwned>(state: &Arc<AppState>, prefix: &str) -> u64 {
    state
        .db
        .scan_prefix::<T>(prefix)
        .await
        .map(|v| v.len() as u64)
        .unwrap_or(0)
}

fn prom_gauge(out: &mut String, name: &str, help: &str, value: u64) {
    out.push_str(&format!("# HELP {} {}\n# TYPE {} gauge\n{} {}\n", name, help, name, name, value));
}
