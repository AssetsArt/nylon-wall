use nylon_wall_common::dhcp::DhcpPool;
use std::sync::Arc;
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

use super::lease_manager::LeaseManager;
use crate::AppState;
use crate::events::WsEvent;

#[cfg(target_os = "linux")]
use {
    super::packet::{self, DhcpMessage},
    nylon_wall_common::dhcp::{DhcpLease, DhcpLeaseState, DhcpReservation},
    std::net::{Ipv4Addr, SocketAddr},
};

/// Run the DHCP server. Listens on all enabled pool interfaces.
/// Watches `pool_rx` for configuration changes to reload.
pub async fn run_dhcp_server(state: Arc<AppState>, mut pool_rx: watch::Receiver<()>) {
    info!("DHCP server starting...");

    loop {
        // Load enabled pools
        let pools = match state.db.scan_prefix::<DhcpPool>("dhcp_pool:").await {
            Ok(p) => p
                .into_iter()
                .map(|(_, pool)| pool)
                .filter(|p| p.enabled)
                .collect::<Vec<_>>(),
            Err(e) => {
                error!("Failed to load DHCP pools: {}", e);
                Vec::new()
            }
        };

        if pools.is_empty() {
            debug!("No enabled DHCP pools, waiting for configuration...");
            // Wait for pool configuration change
            if pool_rx.changed().await.is_err() {
                break; // Channel closed, shutting down
            }
            continue;
        }

        info!("DHCP server managing {} pool(s)", pools.len());

        // Spawn per-pool listener tasks
        let mut listener_handles = Vec::new();
        for pool in &pools {
            let pool = pool.clone();
            let state = Arc::clone(&state);
            let handle = tokio::spawn(async move {
                if let Err(e) = serve_pool(state, pool).await {
                    error!("DHCP pool listener error: {}", e);
                }
            });
            listener_handles.push(handle);
        }

        // Spawn lease expiration task
        let expiry_state = Arc::clone(&state);
        let expiry_handle = tokio::spawn(async move {
            run_lease_expiry(expiry_state).await;
        });

        // Wait for configuration change signal
        if pool_rx.changed().await.is_err() {
            break; // Channel closed
        }

        info!("DHCP pool configuration changed, reloading...");

        // Cancel existing listeners
        for handle in listener_handles {
            handle.abort();
        }
        expiry_handle.abort();
    }

    info!("DHCP server stopped");
}

/// Serve DHCP requests for a single pool on its interface.
#[cfg(target_os = "linux")]
async fn serve_pool(state: Arc<AppState>, pool: DhcpPool) -> anyhow::Result<()> {
    let socket = super::socket::create_server_socket(&pool.interface).await?;
    info!(
        "DHCP server listening on interface {} for pool {}",
        pool.interface, pool.id
    );

    let broadcast_addr: SocketAddr = "255.255.255.255:68".parse()?;
    let mut buf = vec![0u8; 1500];

    loop {
        let (len, _src) = socket.recv_from(&mut buf).await?;
        let data = &buf[..len];

        let msg = match DhcpMessage::parse(data) {
            Ok(m) => m,
            Err(e) => {
                debug!("Failed to parse DHCP packet: {}", e);
                continue;
            }
        };

        let msg_type = match msg.message_type() {
            Some(t) => t,
            None => continue,
        };

        let mac = msg.client_mac();
        let hostname = msg.hostname();

        // Determine server IP (gateway IP or first IP in subnet)
        let server_ip = pool
            .gateway
            .as_ref()
            .and_then(|g| g.parse::<Ipv4Addr>().ok())
            .unwrap_or(Ipv4Addr::new(0, 0, 0, 0));

        let lease_mgr = LeaseManager::new(&state.db);

        // Load reservations for this pool
        let reservations = state
            .db
            .scan_prefix::<DhcpReservation>("dhcp_reservation:")
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|(_, r)| r)
            .filter(|r| r.pool_id == pool.id)
            .collect::<Vec<_>>();

        match msg_type {
            dhcproto::v4::MessageType::Discover => {
                debug!(
                    "DHCPDISCOVER from {} ({})",
                    mac,
                    hostname.as_deref().unwrap_or("unknown")
                );

                match lease_mgr.allocate_ip(&pool, &mac, &reservations).await {
                    Ok(offer_ip) => {
                        let response = packet::build_offer(&msg, offer_ip, &pool, server_ip);
                        if let Err(e) = socket.send_to(&response, broadcast_addr).await {
                            warn!("Failed to send DHCPOFFER: {}", e);
                        } else {
                            debug!("DHCPOFFER {} -> {}", offer_ip, mac);
                        }
                    }
                    Err(e) => {
                        warn!("Cannot allocate IP for {}: {}", mac, e);
                    }
                }
            }

            dhcproto::v4::MessageType::Request => {
                debug!("DHCPREQUEST from {}", mac);

                let requested_ip = msg.requested_ip().or_else(|| {
                    let ci = msg.ciaddr();
                    if ci != Ipv4Addr::UNSPECIFIED {
                        Some(ci)
                    } else {
                        None
                    }
                });

                if let Some(req_ip) = requested_ip {
                    // Verify the IP is valid for this pool
                    let start: Ipv4Addr = pool.range_start.parse().unwrap_or(Ipv4Addr::UNSPECIFIED);
                    let end: Ipv4Addr = pool.range_end.parse().unwrap_or(Ipv4Addr::UNSPECIFIED);
                    let req_u32 = u32::from(req_ip);
                    let is_reserved = reservations
                        .iter()
                        .any(|r| r.mac.eq_ignore_ascii_case(&mac) && r.ip == req_ip.to_string());
                    let in_range = req_u32 >= u32::from(start) && req_u32 <= u32::from(end);

                    if in_range || is_reserved {
                        // Create/update the lease
                        let now = chrono::Utc::now().timestamp();
                        let lease = DhcpLease {
                            ip: req_ip.to_string(),
                            mac: mac.clone(),
                            hostname,
                            pool_id: pool.id,
                            lease_start: now,
                            lease_end: now + pool.lease_time as i64,
                            state: DhcpLeaseState::Active,
                        };

                        if let Err(e) = lease_mgr.store_lease(&lease).await {
                            error!("Failed to store lease: {}", e);
                            let nak = packet::build_nak(&msg, server_ip);
                            let _ = socket.send_to(&nak, broadcast_addr).await;
                            continue;
                        }

                        let response = packet::build_ack(&msg, req_ip, &pool, server_ip);
                        if let Err(e) = socket.send_to(&response, broadcast_addr).await {
                            warn!("Failed to send DHCPACK: {}", e);
                        } else {
                            info!(
                                "DHCPACK {} -> {} ({})",
                                req_ip,
                                mac,
                                lease.hostname.as_deref().unwrap_or("")
                            );
                            // Broadcast lease event
                            let _ = state.event_tx.send(WsEvent::DhcpLeaseChanged(
                                serde_json::to_value(&lease).unwrap_or_default(),
                            ));
                        }
                    } else {
                        // IP not valid for this pool
                        let nak = packet::build_nak(&msg, server_ip);
                        let _ = socket.send_to(&nak, broadcast_addr).await;
                        debug!("DHCPNAK {} (requested {} not in pool range)", mac, req_ip);
                    }
                } else {
                    let nak = packet::build_nak(&msg, server_ip);
                    let _ = socket.send_to(&nak, broadcast_addr).await;
                }
            }

            dhcproto::v4::MessageType::Release => {
                info!("DHCPRELEASE from {}", mac);
                if let Err(e) = lease_mgr.release_lease(&mac).await {
                    warn!("Failed to release lease for {}: {}", mac, e);
                }
            }

            _ => {
                debug!("Ignoring DHCP message type {:?} from {}", msg_type, mac);
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
async fn serve_pool(_state: Arc<AppState>, pool: DhcpPool) -> anyhow::Result<()> {
    info!(
        "DHCP server for pool {} (interface {}) not available on this platform",
        pool.id, pool.interface
    );
    // On non-Linux, just idle forever
    std::future::pending::<()>().await;
    Ok(())
}

/// Periodically expire stale leases.
async fn run_lease_expiry(state: Arc<AppState>) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
    loop {
        interval.tick().await;
        let lease_mgr = LeaseManager::new(&state.db);
        match lease_mgr.expire_leases().await {
            Ok(expired) => {
                if !expired.is_empty() {
                    debug!("Expired {} DHCP leases", expired.len());
                    for lease in &expired {
                        let _ = state.event_tx.send(WsEvent::DhcpLeaseChanged(
                            serde_json::to_value(lease).unwrap_or_default(),
                        ));
                    }
                }
            }
            Err(e) => {
                warn!("Lease expiry check failed: {}", e);
            }
        }
    }
}
