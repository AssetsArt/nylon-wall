use nylon_wall_common::l4proxy::L4ProxyRule;
use tracing::info;

use crate::AppState;

/// Sync all enabled L4 proxy rules to eBPF maps.
/// For now this is a placeholder — eBPF map interaction will be added
/// when the eBPF programs are compiled on Linux.
pub async fn sync_l4proxy_to_ebpf(state: &AppState) {
    let rules = state
        .db
        .scan_prefix::<L4ProxyRule>("l4proxy:")
        .await
        .unwrap_or_default();

    let enabled_count = rules.iter().filter(|(_, r)| r.enabled).count();
    info!("Syncing {} L4 proxy rules to eBPF ({} enabled)", rules.len(), enabled_count);

    // TODO: When eBPF programs are built on Linux:
    // 1. Clear L4_PROXY_TABLE
    // 2. For each enabled rule, select upstream via loadbalance::select_upstream()
    // 3. Write EbpfL4ProxyKey → EbpfL4ProxyEntry to map
}
