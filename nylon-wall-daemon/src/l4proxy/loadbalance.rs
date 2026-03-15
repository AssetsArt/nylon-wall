use nylon_wall_common::l4proxy::{LoadBalanceMode, UpstreamTarget};
use std::sync::atomic::{AtomicU64, Ordering};

static RR_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Select an upstream target based on the load balance mode.
pub fn select_upstream<'a>(
    targets: &'a [UpstreamTarget],
    mode: LoadBalanceMode,
    client_ip: Option<u32>,
) -> Option<&'a UpstreamTarget> {
    if targets.is_empty() {
        return None;
    }
    match mode {
        LoadBalanceMode::RoundRobin => {
            let idx = RR_COUNTER.fetch_add(1, Ordering::Relaxed) as usize % targets.len();
            Some(&targets[idx])
        }
        LoadBalanceMode::IpHash => {
            let ip = client_ip.unwrap_or(0);
            let hash = fnv1a_u32(ip);
            let idx = hash as usize % targets.len();
            Some(&targets[idx])
        }
    }
}

/// FNV-1a hash of a u32 value (matching eBPF side)
fn fnv1a_u32(value: u32) -> u32 {
    let bytes = value.to_be_bytes();
    let mut hash: u32 = 2166136261;
    for byte in &bytes {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(16777619);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(addr: &str, port: u16) -> UpstreamTarget {
        UpstreamTarget {
            address: addr.to_string(),
            port,
            weight: 1,
        }
    }

    #[test]
    fn round_robin_cycles() {
        let targets = vec![target("10.0.0.1", 80), target("10.0.0.2", 80), target("10.0.0.3", 80)];
        // Reset counter for deterministic test
        RR_COUNTER.store(0, Ordering::Relaxed);

        let first = select_upstream(&targets, LoadBalanceMode::RoundRobin, None).unwrap();
        assert_eq!(first.address, "10.0.0.1");

        let second = select_upstream(&targets, LoadBalanceMode::RoundRobin, None).unwrap();
        assert_eq!(second.address, "10.0.0.2");

        let third = select_upstream(&targets, LoadBalanceMode::RoundRobin, None).unwrap();
        assert_eq!(third.address, "10.0.0.3");

        // Wraps around
        let fourth = select_upstream(&targets, LoadBalanceMode::RoundRobin, None).unwrap();
        assert_eq!(fourth.address, "10.0.0.1");
    }

    #[test]
    fn ip_hash_deterministic() {
        let targets = vec![target("10.0.0.1", 80), target("10.0.0.2", 80)];
        let ip = 0xC0A80101u32; // 192.168.1.1

        let first = select_upstream(&targets, LoadBalanceMode::IpHash, Some(ip)).unwrap();
        let second = select_upstream(&targets, LoadBalanceMode::IpHash, Some(ip)).unwrap();
        assert_eq!(first.address, second.address);
    }
}
