use crate::db::Database;
use nylon_wall_common::dhcp::{DhcpLease, DhcpLeaseState, DhcpPool, DhcpReservation};
use std::net::Ipv4Addr;

pub struct LeaseManager<'a> {
    db: &'a Database,
}

impl<'a> LeaseManager<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Find an existing lease by MAC address.
    pub async fn find_lease_by_mac(&self, mac: &str) -> anyhow::Result<Option<DhcpLease>> {
        let key = format!("dhcp_lease:{}", mac);
        Ok(self.db.get::<DhcpLease>(&key).await?)
    }

    /// Allocate an IP for a client MAC, checking reservations first.
    pub async fn allocate_ip(
        &self,
        pool: &DhcpPool,
        mac: &str,
        reservations: &[DhcpReservation],
    ) -> anyhow::Result<Ipv4Addr> {
        // 1. Check if this MAC has a static reservation
        for res in reservations {
            if res.mac.eq_ignore_ascii_case(mac) && res.pool_id == pool.id {
                return Ok(res.ip.parse::<Ipv4Addr>()?);
            }
        }

        // 2. Check if this MAC already has an active lease
        if let Some(existing) = self.find_lease_by_mac(mac).await? {
            let now = chrono::Utc::now().timestamp();
            if existing.state == DhcpLeaseState::Active && existing.lease_end > now {
                return Ok(existing.ip.parse::<Ipv4Addr>()?);
            }
        }

        // 3. Find first available IP in pool range
        let start: Ipv4Addr = pool.range_start.parse()?;
        let end: Ipv4Addr = pool.range_end.parse()?;
        let start_u32 = u32::from(start);
        let end_u32 = u32::from(end);

        // Collect all currently assigned IPs
        let existing_leases = self.db.scan_prefix::<DhcpLease>("dhcp_lease:").await?;
        let now = chrono::Utc::now().timestamp();
        let used_ips: std::collections::HashSet<String> = existing_leases
            .iter()
            .filter(|(_, l)| {
                l.pool_id == pool.id && l.state == DhcpLeaseState::Active && l.lease_end > now
            })
            .map(|(_, l)| l.ip.clone())
            .collect();

        // Also exclude reserved IPs
        let reserved_ips: std::collections::HashSet<String> = reservations
            .iter()
            .filter(|r| r.pool_id == pool.id)
            .map(|r| r.ip.clone())
            .collect();

        for ip_u32 in start_u32..=end_u32 {
            let candidate = Ipv4Addr::from(ip_u32);
            let candidate_str = candidate.to_string();
            if !used_ips.contains(&candidate_str) && !reserved_ips.contains(&candidate_str) {
                return Ok(candidate);
            }
        }

        anyhow::bail!("DHCP pool {} exhausted: no available IPs", pool.id)
    }

    /// Create or update a lease in the database.
    pub async fn store_lease(&self, lease: &DhcpLease) -> anyhow::Result<()> {
        let key = format!("dhcp_lease:{}", lease.mac);
        self.db.put(&key, lease).await?;
        self.db.add_to_index("dhcp_lease:", &key).await?;
        Ok(())
    }

    /// Renew an existing lease by updating its expiry.
    pub async fn renew_lease(
        &self,
        mac: &str,
        lease_time: u32,
    ) -> anyhow::Result<Option<DhcpLease>> {
        let key = format!("dhcp_lease:{}", mac);
        if let Some(mut lease) = self.db.get::<DhcpLease>(&key).await? {
            let now = chrono::Utc::now().timestamp();
            lease.lease_start = now;
            lease.lease_end = now + lease_time as i64;
            lease.state = DhcpLeaseState::Active;
            self.db.put(&key, &lease).await?;
            Ok(Some(lease))
        } else {
            Ok(None)
        }
    }

    /// Release (delete) a lease by MAC address.
    pub async fn release_lease(&self, mac: &str) -> anyhow::Result<()> {
        let key = format!("dhcp_lease:{}", mac);
        self.db.delete(&key).await?;
        self.db.remove_from_index("dhcp_lease:", &key).await?;
        Ok(())
    }

    /// Scan all leases and mark expired ones.
    pub async fn expire_leases(&self) -> anyhow::Result<Vec<DhcpLease>> {
        let now = chrono::Utc::now().timestamp();
        let leases = self.db.scan_prefix::<DhcpLease>("dhcp_lease:").await?;
        let mut expired = Vec::new();

        for (key, mut lease) in leases {
            if lease.state == DhcpLeaseState::Active && lease.lease_end <= now {
                lease.state = DhcpLeaseState::Expired;
                self.db.put(&key, &lease).await?;
                expired.push(lease);
            }
        }

        Ok(expired)
    }

    /// Get all active leases.
    pub async fn list_leases(&self) -> anyhow::Result<Vec<DhcpLease>> {
        let leases = self.db.scan_prefix::<DhcpLease>("dhcp_lease:").await?;
        Ok(leases.into_iter().map(|(_, l)| l).collect())
    }
}
