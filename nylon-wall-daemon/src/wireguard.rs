use nylon_wall_common::wireguard::{WgPeer, WgPeerStatus, WgServer};
#[cfg(target_os = "linux")]
use tracing::info;
#[cfg(target_os = "linux")]
use tracing::warn;

use crate::db::Database;

const SERVER_KEY: &str = "wg:server";

/// Load WireGuard server config from DB.
pub async fn load_server(db: &Database) -> WgServer {
    db.get::<WgServer>(SERVER_KEY)
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// Save WireGuard server config to DB.
pub async fn save_server(db: &Database, server: &WgServer) -> Result<(), String> {
    db.put(SERVER_KEY, server)
        .await
        .map_err(|e| e.to_string())
}

/// Generate a WireGuard keypair using `wg genkey` + `wg pubkey`.
pub fn generate_keypair() -> Result<(String, String), String> {
    #[cfg(target_os = "linux")]
    {
        let genkey = std::process::Command::new("wg")
            .arg("genkey")
            .output()
            .map_err(|e| format!("wg genkey failed: {}", e))?;
        let private_key = String::from_utf8_lossy(&genkey.stdout).trim().to_string();

        let pubkey = std::process::Command::new("wg")
            .arg("pubkey")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child.stdin.take().unwrap().write_all(private_key.as_bytes())?;
                child.wait_with_output()
            })
            .map_err(|e| format!("wg pubkey failed: {}", e))?;
        let public_key = String::from_utf8_lossy(&pubkey.stdout).trim().to_string();

        Ok((private_key, public_key))
    }

    #[cfg(not(target_os = "linux"))]
    {
        // Demo mode: generate fake keys
        use rand::RngExt;
        let mut rng = rand::rng();
        let priv_bytes: [u8; 32] = rng.random();
        let pub_bytes: [u8; 32] = rng.random();
        use base64::Engine;
        let engine = base64::engine::general_purpose::STANDARD;
        Ok((engine.encode(priv_bytes), engine.encode(pub_bytes)))
    }
}

/// Generate a preshared key.
pub fn generate_psk() -> Result<String, String> {
    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("wg")
            .arg("genpsk")
            .output()
            .map_err(|e| format!("wg genpsk failed: {}", e))?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    #[cfg(not(target_os = "linux"))]
    {
        use rand::RngExt;
        let bytes: [u8; 32] = rand::rng().random();
        use base64::Engine;
        Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
    }
}

/// Apply WireGuard configuration to the system (Linux only).
#[cfg(target_os = "linux")]
pub async fn apply_config(server: &WgServer, peers: &[WgPeer]) -> Result<(), String> {
    let iface = &server.interface;

    // Create interface if it doesn't exist
    let _ = std::process::Command::new("ip")
        .args(["link", "add", iface, "type", "wireguard"])
        .output();

    // Set private key via temp file
    let key_path = format!("/tmp/wg_{}_key", iface);
    std::fs::write(&key_path, &server.private_key)
        .map_err(|e| format!("Failed to write key: {}", e))?;

    let mut cmd = std::process::Command::new("wg");
    cmd.args([
        "set",
        iface,
        "listen-port",
        &server.listen_port.to_string(),
        "private-key",
        &key_path,
    ]);
    let output = cmd.output().map_err(|e| format!("wg set failed: {}", e))?;
    let _ = std::fs::remove_file(&key_path);

    if !output.status.success() {
        return Err(format!(
            "wg set failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // Set IP address
    let _ = std::process::Command::new("ip")
        .args(["addr", "flush", "dev", iface])
        .output();
    let add_addr = std::process::Command::new("ip")
        .args(["addr", "add", &server.address, "dev", iface])
        .output()
        .map_err(|e| format!("ip addr add failed: {}", e))?;
    if !add_addr.status.success() {
        warn!(
            "ip addr add: {}",
            String::from_utf8_lossy(&add_addr.stderr)
        );
    }

    // Bring interface up
    let _ = std::process::Command::new("ip")
        .args(["link", "set", iface, "up"])
        .output();

    // Add peers
    for peer in peers {
        if !peer.enabled {
            continue;
        }
        let mut args = vec![
            "set".to_string(),
            iface.to_string(),
            "peer".to_string(),
            peer.public_key.clone(),
            "allowed-ips".to_string(),
            peer.allowed_ips.clone(),
        ];
        if !peer.preshared_key.is_empty() {
            let psk_path = format!("/tmp/wg_{}_psk_{}", iface, peer.id);
            let _ = std::fs::write(&psk_path, &peer.preshared_key);
            args.push("preshared-key".to_string());
            args.push(psk_path.clone());
        }
        if peer.persistent_keepalive > 0 {
            args.push("persistent-keepalive".to_string());
            args.push(peer.persistent_keepalive.to_string());
        }
        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let _ = std::process::Command::new("wg").args(&arg_refs).output();
    }

    info!("WireGuard config applied to {}", iface);
    Ok(())
}

/// Remove WireGuard interface (Linux only).
#[cfg(target_os = "linux")]
pub fn remove_interface(iface: &str) {
    let _ = std::process::Command::new("ip")
        .args(["link", "delete", iface])
        .output();
}

/// Get live peer status from `wg show`.
pub fn get_peer_status(iface: &str) -> Vec<WgPeerStatus> {
    #[cfg(target_os = "linux")]
    {
        let output = match std::process::Command::new("wg")
            .args(["show", iface, "dump"])
            .output()
        {
            Ok(o) if o.status.success() => {
                String::from_utf8_lossy(&o.stdout).to_string()
            }
            _ => return Vec::new(),
        };

        // wg show dump format (tab-separated):
        // Line 1: interface private-key public-key listen-port fwmark
        // Line 2+: peer public-key preshared-key endpoint allowed-ips latest-handshake transfer-rx transfer-tx persistent-keepalive
        output
            .lines()
            .skip(1) // skip interface line
            .filter_map(|line| {
                let fields: Vec<&str> = line.split('\t').collect();
                if fields.len() >= 8 {
                    Some(WgPeerStatus {
                        public_key: fields[0].to_string(),
                        endpoint: fields[3].to_string(),
                        allowed_ips: fields[4].to_string(),
                        last_handshake: fields[5].to_string(),
                        transfer_rx: fields[6].parse().unwrap_or(0),
                        transfer_tx: fields[7].parse().unwrap_or(0),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = iface;
        Vec::new()
    }
}

/// Build a peer .conf file content for download.
pub fn build_peer_config(server: &WgServer, peer: &WgPeer) -> String {
    let mut conf = format!(
        "[Interface]\nPrivateKey = {}\nAddress = {}\n",
        peer.private_key, peer.allowed_ips
    );

    if !server.dns.is_empty() {
        conf.push_str(&format!("DNS = {}\n", server.dns.join(", ")));
    }

    conf.push_str(&format!(
        "\n[Peer]\nPublicKey = {}\n",
        server.public_key
    ));

    if !peer.preshared_key.is_empty() {
        conf.push_str(&format!("PresharedKey = {}\n", peer.preshared_key));
    }

    // AllowedIPs for the client — route all traffic through VPN
    conf.push_str("AllowedIPs = 0.0.0.0/0, ::/0\n");

    if !server.endpoint.is_empty() {
        conf.push_str(&format!("Endpoint = {}:{}\n", server.endpoint, server.listen_port));
    }

    if peer.persistent_keepalive > 0 {
        conf.push_str(&format!(
            "PersistentKeepalive = {}\n",
            peer.persistent_keepalive
        ));
    }

    conf
}
