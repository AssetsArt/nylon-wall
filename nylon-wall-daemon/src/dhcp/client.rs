use nylon_wall_common::dhcp::{DhcpClientConfig, DhcpClientState, DhcpClientStatus};
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::AppState;
use crate::events::WsEvent;

#[cfg(target_os = "linux")]
use {
    super::packet::{self, DhcpMessage},
    std::net::{Ipv4Addr, SocketAddr},
    tracing::debug,
};

/// Run the DHCP client state machine for a single WAN interface.
pub async fn run_dhcp_client(state: Arc<AppState>, config: DhcpClientConfig) {
    info!("DHCP client starting on interface {}", config.interface);

    loop {
        update_status(
            &state,
            &config.interface,
            DhcpClientState::Discovering,
            None,
        )
        .await;

        match dhcp_acquire(&state, &config).await {
            Ok(status) => {
                let lease_time = match (status.lease_start, status.lease_end) {
                    (Some(start), Some(end)) => (end - start) as u64,
                    _ => 86400,
                };

                update_status(
                    &state,
                    &config.interface,
                    DhcpClientState::Bound,
                    Some(&status),
                )
                .await;
                info!(
                    "DHCP client bound: {} on {} (lease {}s)",
                    status.ip.as_deref().unwrap_or("unknown"),
                    config.interface,
                    lease_time,
                );

                // Sleep until T1 (50% of lease time) for renewal
                let t1 = lease_time / 2;
                tokio::time::sleep(tokio::time::Duration::from_secs(t1)).await;

                // Attempt renewal
                update_status(
                    &state,
                    &config.interface,
                    DhcpClientState::Renewing,
                    Some(&status),
                )
                .await;
                info!("DHCP client renewing lease on {}", config.interface);

                // If renewal fails, sleep until T2 (87.5%) and try rebinding
                match dhcp_acquire(&state, &config).await {
                    Ok(new_status) => {
                        update_status(
                            &state,
                            &config.interface,
                            DhcpClientState::Bound,
                            Some(&new_status),
                        )
                        .await;
                        info!("DHCP lease renewed on {}", config.interface);
                        let new_lease = match (new_status.lease_start, new_status.lease_end) {
                            (Some(start), Some(end)) => (end - start) as u64,
                            _ => 86400,
                        };
                        let remaining = new_lease / 2;
                        tokio::time::sleep(tokio::time::Duration::from_secs(remaining)).await;
                    }
                    Err(e) => {
                        warn!(
                            "DHCP renewal failed on {}: {}, will rebind",
                            config.interface, e
                        );
                        update_status(
                            &state,
                            &config.interface,
                            DhcpClientState::Rebinding,
                            Some(&status),
                        )
                        .await;
                        // Wait a bit and try full discovery again
                        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                    }
                }
            }
            Err(e) => {
                error!("DHCP acquire failed on {}: {}", config.interface, e);
                update_status(&state, &config.interface, DhcpClientState::Error, None).await;
                // Retry after backoff
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            }
        }
    }
}

/// Perform a DHCP DISCOVER → OFFER → REQUEST → ACK exchange.
#[cfg(target_os = "linux")]
async fn dhcp_acquire(
    state: &Arc<AppState>,
    config: &DhcpClientConfig,
) -> anyhow::Result<DhcpClientStatus> {
    let socket = super::socket::create_client_socket(&config.interface).await?;
    let broadcast: SocketAddr = "255.255.255.255:67".parse()?;
    let xid: u32 = rand::random();

    // Get interface MAC address
    let mac = get_interface_mac(&config.interface)?;

    // Send DISCOVER
    let discover = packet::build_discover(&mac, xid, config.hostname.as_deref());
    socket.send_to(&discover, broadcast).await?;
    debug!("DHCPDISCOVER sent on {}", config.interface);

    // Wait for OFFER (with timeout)
    let mut buf = vec![0u8; 1500];
    let offer = tokio::time::timeout(tokio::time::Duration::from_secs(10), async {
        loop {
            let (len, _) = socket.recv_from(&mut buf).await?;
            let msg = DhcpMessage::parse(&buf[..len])?;
            if msg.xid() == xid {
                if let Some(dhcproto::v4::MessageType::Offer) = msg.message_type() {
                    return Ok::<_, anyhow::Error>(msg);
                }
            }
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("DHCPOFFER timeout"))??;

    let offered_ip = Ipv4Addr::from(offer.inner_yiaddr());
    let server_ip = offer.server_identifier().unwrap_or(Ipv4Addr::UNSPECIFIED);
    debug!("DHCPOFFER received: {} from {}", offered_ip, server_ip);

    // Send REQUEST
    let request = packet::build_request(&mac, xid, server_ip, offered_ip);
    socket.send_to(&request, broadcast).await?;
    debug!("DHCPREQUEST sent for {}", offered_ip);

    // Wait for ACK
    let ack = tokio::time::timeout(tokio::time::Duration::from_secs(10), async {
        loop {
            let (len, _) = socket.recv_from(&mut buf).await?;
            let msg = DhcpMessage::parse(&buf[..len])?;
            if msg.xid() == xid {
                match msg.message_type() {
                    Some(dhcproto::v4::MessageType::Ack) => {
                        return Ok::<_, anyhow::Error>(msg);
                    }
                    Some(dhcproto::v4::MessageType::Nak) => {
                        anyhow::bail!("DHCPNAK received");
                    }
                    _ => continue,
                }
            }
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("DHCPACK timeout"))??;

    // Extract options from ACK
    let subnet_mask = ack.subnet_mask().map(|m| m.to_string());
    let gateway = ack.router().map(|r| r.to_string());
    let dns_servers = ack
        .dns_servers()
        .into_iter()
        .map(|ip| ip.to_string())
        .collect();
    let lease_time = ack.lease_time().unwrap_or(86400);

    let now = chrono::Utc::now().timestamp();

    // Apply IP configuration to the interface
    apply_ip_config(
        &config.interface,
        &offered_ip.to_string(),
        subnet_mask.as_deref(),
        gateway.as_deref(),
    )
    .await?;

    let status = DhcpClientStatus {
        interface: config.interface.clone(),
        state: DhcpClientState::Bound,
        ip: Some(offered_ip.to_string()),
        subnet_mask,
        gateway,
        dns_servers,
        dhcp_server: Some(server_ip.to_string()),
        lease_start: Some(now),
        lease_end: Some(now + lease_time as i64),
        last_renewed: Some(now),
    };

    // Persist status
    let key = format!("dhcp_client_status:{}", config.interface);
    let _ = state.db.put(&key, &status).await;

    Ok(status)
}

#[cfg(not(target_os = "linux"))]
async fn dhcp_acquire(
    _state: &Arc<AppState>,
    config: &DhcpClientConfig,
) -> anyhow::Result<DhcpClientStatus> {
    info!(
        "DHCP client not available on this platform (interface {})",
        config.interface
    );
    anyhow::bail!("DHCP client requires Linux")
}

/// Apply IP configuration to a network interface using iproute2.
#[cfg(target_os = "linux")]
async fn apply_ip_config(
    interface: &str,
    ip: &str,
    mask: Option<&str>,
    gateway: Option<&str>,
) -> anyhow::Result<()> {
    // Calculate prefix length from subnet mask
    let prefix = mask
        .and_then(|m| m.parse::<Ipv4Addr>().ok())
        .map(|m| u32::from(m).count_ones())
        .unwrap_or(24);

    // Flush existing address
    let _ = tokio::process::Command::new("ip")
        .args(["addr", "flush", "dev", interface])
        .output()
        .await;

    // Add new address
    let addr = format!("{}/{}", ip, prefix);
    let output = tokio::process::Command::new("ip")
        .args(["addr", "add", &addr, "dev", interface])
        .output()
        .await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("ip addr add failed: {}", stderr);
    }

    // Add default route via gateway
    if let Some(gw) = gateway {
        // Remove old default route first
        let _ = tokio::process::Command::new("ip")
            .args(["route", "del", "default", "dev", interface])
            .output()
            .await;
        let output = tokio::process::Command::new("ip")
            .args(["route", "add", "default", "via", gw, "dev", interface])
            .output()
            .await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("ip route add failed: {}", stderr);
        }
    }

    info!(
        "Applied IP config: {}/{} gateway {:?} on {}",
        ip, prefix, gateway, interface
    );
    Ok(())
}

#[cfg(not(target_os = "linux"))]
async fn apply_ip_config(
    _interface: &str,
    _ip: &str,
    _mask: Option<&str>,
    _gateway: Option<&str>,
) -> anyhow::Result<()> {
    Ok(())
}

/// Get the MAC address of a network interface.
#[cfg(target_os = "linux")]
fn get_interface_mac(interface: &str) -> anyhow::Result<[u8; 6]> {
    let path = format!("/sys/class/net/{}/address", interface);
    let mac_str = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("Cannot read MAC for {}: {}", interface, e))?;
    super::parse_mac(mac_str.trim())
        .ok_or_else(|| anyhow::anyhow!("Invalid MAC format for {}", interface))
}

#[cfg(not(target_os = "linux"))]
fn get_interface_mac(_interface: &str) -> anyhow::Result<[u8; 6]> {
    Ok([0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01])
}

/// Update the DHCP client status in AppState and broadcast event.
async fn update_status(
    state: &Arc<AppState>,
    interface: &str,
    new_state: DhcpClientState,
    existing: Option<&DhcpClientStatus>,
) {
    let status = match existing {
        Some(s) => {
            let mut updated = s.clone();
            updated.state = new_state;
            updated
        }
        None => DhcpClientStatus {
            interface: interface.to_string(),
            state: new_state,
            ip: None,
            subnet_mask: None,
            gateway: None,
            dns_servers: Vec::new(),
            dhcp_server: None,
            lease_start: None,
            lease_end: None,
            last_renewed: None,
        },
    };

    // Store in DB
    let key = format!("dhcp_client_status:{}", interface);
    let _ = state.db.put(&key, &status).await;

    // Broadcast event
    let _ = state.event_tx.send(WsEvent::DhcpClientStatusChanged(
        serde_json::to_value(&status).unwrap_or_default(),
    ));
}
