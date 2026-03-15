use std::sync::Arc;
use std::collections::HashMap;

use nylon_wall_common::ddns::{DdnsEntry, DdnsProvider, DdnsStatus};
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::AppState;
use crate::events::WsEvent;

const STATUS_PREFIX: &str = "ddns_status:";

/// In-memory DDNS status cache + update handles.
pub struct DdnsManager {
    /// Abort handles for running update loops, keyed by entry id.
    tasks: Mutex<HashMap<u32, tokio::task::JoinHandle<()>>>,
}

impl DdnsManager {
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(HashMap::new()),
        }
    }

    /// Start update loop for an entry.
    pub async fn start(&self, state: Arc<AppState>, entry: DdnsEntry) {
        self.stop(entry.id).await;
        let id = entry.id;
        let handle = tokio::spawn(update_loop(state, entry));
        self.tasks.lock().await.insert(id, handle);
    }

    /// Stop update loop for an entry.
    pub async fn stop(&self, id: u32) {
        if let Some(handle) = self.tasks.lock().await.remove(&id) {
            handle.abort();
        }
    }

    /// Stop all loops.
    pub async fn stop_all(&self) {
        let mut tasks = self.tasks.lock().await;
        for (_, handle) in tasks.drain() {
            handle.abort();
        }
    }
}

/// Detect current WAN IP via public API.
async fn detect_wan_ip(client: &reqwest::Client) -> Result<String, String> {
    // Try multiple services for reliability
    let services = [
        "https://api.ipify.org",
        "https://ifconfig.me/ip",
        "https://icanhazip.com",
    ];
    for url in &services {
        match client.get(*url).timeout(std::time::Duration::from_secs(10)).send().await {
            Ok(resp) => {
                if let Ok(ip) = resp.text().await {
                    let ip = ip.trim().to_string();
                    if !ip.is_empty() && ip.parse::<std::net::IpAddr>().is_ok() {
                        return Ok(ip);
                    }
                }
            }
            Err(_) => continue,
        }
    }
    Err("Failed to detect WAN IP from all services".to_string())
}

/// Update DNS record at the provider.
async fn update_provider(
    client: &reqwest::Client,
    entry: &DdnsEntry,
    ip: &str,
) -> Result<(), String> {
    match entry.provider {
        DdnsProvider::Cloudflare => update_cloudflare(client, entry, ip).await,
        DdnsProvider::DuckDns => update_duckdns(client, entry, ip).await,
        DdnsProvider::NoIp => update_noip(client, entry, ip).await,
        DdnsProvider::Dynu => update_dynu(client, entry, ip).await,
        DdnsProvider::Custom => update_custom(client, entry, ip).await,
    }
}

async fn update_cloudflare(
    client: &reqwest::Client,
    entry: &DdnsEntry,
    ip: &str,
) -> Result<(), String> {
    // username = zone_id, token = API token
    // Step 1: find the record ID
    let zone_id = &entry.username;
    let list_url = format!(
        "https://api.cloudflare.com/client/v4/zones/{}/dns_records?type=A&name={}",
        zone_id, entry.hostname
    );
    let resp = client
        .get(&list_url)
        .header("Authorization", format!("Bearer {}", entry.token))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let record_id = body["result"][0]["id"]
        .as_str()
        .ok_or("DNS record not found in Cloudflare")?;

    // Step 2: update the record
    let update_url = format!(
        "https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
        zone_id, record_id
    );
    let update_body = serde_json::json!({
        "type": "A",
        "name": entry.hostname,
        "content": ip,
        "ttl": 120,
        "proxied": false,
    });
    let resp = client
        .put(&update_url)
        .header("Authorization", format!("Bearer {}", entry.token))
        .json(&update_body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!("Cloudflare update failed: {}", text))
    }
}

async fn update_duckdns(
    client: &reqwest::Client,
    entry: &DdnsEntry,
    ip: &str,
) -> Result<(), String> {
    // hostname = subdomain (without .duckdns.org), token = DuckDNS token
    let domain = entry.hostname.trim_end_matches(".duckdns.org");
    let url = format!(
        "https://www.duckdns.org/update?domains={}&token={}&ip={}",
        domain, entry.token, ip
    );
    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    let text = resp.text().await.unwrap_or_default();
    if text.trim() == "OK" {
        Ok(())
    } else {
        Err(format!("DuckDNS update failed: {}", text))
    }
}

async fn update_noip(
    client: &reqwest::Client,
    entry: &DdnsEntry,
    ip: &str,
) -> Result<(), String> {
    let url = format!(
        "https://dynupdate.no-ip.com/nic/update?hostname={}&myip={}",
        entry.hostname, ip
    );
    let resp = client
        .get(&url)
        .basic_auth(&entry.username, Some(&entry.token))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let text = resp.text().await.unwrap_or_default();
    if text.starts_with("good") || text.starts_with("nochg") {
        Ok(())
    } else {
        Err(format!("No-IP update failed: {}", text))
    }
}

async fn update_dynu(
    client: &reqwest::Client,
    entry: &DdnsEntry,
    ip: &str,
) -> Result<(), String> {
    let url = format!(
        "https://api.dynu.com/nic/update?hostname={}&myip={}",
        entry.hostname, ip
    );
    let resp = client
        .get(&url)
        .basic_auth(&entry.username, Some(&entry.token))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let text = resp.text().await.unwrap_or_default();
    if text.starts_with("good") || text.starts_with("nochg") {
        Ok(())
    } else {
        Err(format!("Dynu update failed: {}", text))
    }
}

async fn update_custom(
    client: &reqwest::Client,
    entry: &DdnsEntry,
    ip: &str,
) -> Result<(), String> {
    if entry.custom_url.is_empty() {
        return Err("Custom URL is empty".to_string());
    }
    // Replace placeholders in URL
    let url = entry
        .custom_url
        .replace("{ip}", ip)
        .replace("{hostname}", &entry.hostname);
    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(())
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!("Custom DDNS update failed: {}", text))
    }
}

/// Background loop for a single DDNS entry.
async fn update_loop(state: Arc<AppState>, entry: DdnsEntry) {
    let client = reqwest::Client::new();
    let interval = std::time::Duration::from_secs(entry.interval_secs.max(60));
    let status_key = format!("{}{}", STATUS_PREFIX, entry.id);

    // Load existing status
    let mut status = state
        .db
        .get::<DdnsStatus>(&status_key)
        .await
        .ok()
        .flatten()
        .unwrap_or(DdnsStatus {
            id: entry.id,
            ..Default::default()
        });

    loop {
        // Detect WAN IP
        match detect_wan_ip(&client).await {
            Ok(ip) => {
                let ip_changed = status.current_ip.as_deref() != Some(&ip);
                if ip_changed {
                    info!("DDNS [{}]: IP changed to {}", entry.hostname, ip);
                    match update_provider(&client, &entry, &ip).await {
                        Ok(()) => {
                            status.current_ip = Some(ip);
                            status.last_update =
                                Some(chrono::Utc::now().to_rfc3339());
                            status.last_error = None;
                            status.update_count += 1;
                            info!("DDNS [{}]: updated successfully", entry.hostname);
                        }
                        Err(e) => {
                            warn!("DDNS [{}]: update failed: {}", entry.hostname, e);
                            status.last_error = Some(e);
                        }
                    }
                    // Persist status
                    let _ = state.db.put(&status_key, &status).await;
                    // Notify UI
                    let _ = state.event_tx.send(WsEvent::DdnsStatusChanged(
                        serde_json::to_value(&status).unwrap_or_default(),
                    ));
                }
            }
            Err(e) => {
                warn!("DDNS [{}]: IP detection failed: {}", entry.hostname, e);
                status.last_error = Some(e);
                let _ = state.db.put(&status_key, &status).await;
            }
        }

        tokio::time::sleep(interval).await;
    }
}

/// Force an immediate update for a DDNS entry.
pub async fn force_update(state: &Arc<AppState>, entry: &DdnsEntry) -> Result<DdnsStatus, String> {
    let client = reqwest::Client::new();
    let status_key = format!("{}{}", STATUS_PREFIX, entry.id);

    let ip = detect_wan_ip(&client).await?;

    update_provider(&client, entry, &ip).await?;

    let mut status = state
        .db
        .get::<DdnsStatus>(&status_key)
        .await
        .ok()
        .flatten()
        .unwrap_or(DdnsStatus {
            id: entry.id,
            ..Default::default()
        });

    status.current_ip = Some(ip);
    status.last_update = Some(chrono::Utc::now().to_rfc3339());
    status.last_error = None;
    status.update_count += 1;

    let _ = state.db.put(&status_key, &status).await;
    let _ = state.event_tx.send(WsEvent::DdnsStatusChanged(
        serde_json::to_value(&status).unwrap_or_default(),
    ));

    Ok(status)
}

/// Load status for all DDNS entries.
pub async fn load_all_status(state: &Arc<AppState>) -> Vec<DdnsStatus> {
    state
        .db
        .scan_prefix::<DdnsStatus>(STATUS_PREFIX)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|(_, s)| s)
        .collect()
}
