use std::net::Ipv4Addr;
#[cfg(target_os = "linux")]
use std::sync::Arc;

use nylon_wall_common::mdns::MdnsConfig;
use tokio::sync::Mutex;
#[cfg(target_os = "linux")]
use tracing::warn;
use tracing::info;

use crate::AppState;

const MDNS_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MDNS_PORT: u16 = 5353;
const MDNS_CONFIG_KEY: &str = "mdns_config";

/// mDNS reflector manager — holds the running task handle.
pub struct MdnsReflector {
    task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl MdnsReflector {
    pub fn new() -> Self {
        Self {
            task: Mutex::new(None),
        }
    }

    /// Start the reflector with the given config.
    pub async fn start(&self, config: MdnsConfig) {
        self.stop().await;
        if !config.enabled || config.interfaces.len() < 2 {
            return;
        }
        #[cfg(target_os = "linux")]
        {
            let handle = tokio::spawn(reflector_loop(config));
            *self.task.lock().await = Some(handle);
        }
        #[cfg(not(target_os = "linux"))]
        {
            info!("mDNS reflector: skipped (not Linux)");
        }
    }

    /// Stop the reflector.
    pub async fn stop(&self) {
        if let Some(handle) = self.task.lock().await.take() {
            handle.abort();
        }
    }

    /// Restart with the given config (stop + start).
    pub async fn restart(&self, config: MdnsConfig) {
        self.stop().await;
        self.start(config).await;
    }
}

/// Load mDNS config from the database.
pub async fn load_config(state: &AppState) -> MdnsConfig {
    state
        .db
        .get::<MdnsConfig>(MDNS_CONFIG_KEY)
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// Save mDNS config to the database.
pub async fn save_config(state: &AppState, config: &MdnsConfig) -> Result<(), String> {
    state
        .db
        .put(MDNS_CONFIG_KEY, config)
        .await
        .map_err(|e| e.to_string())
}

/// The main reflector loop (Linux only).
/// Binds to 224.0.0.251:5353 on each interface, receives mDNS packets,
/// and re-sends them on all other configured interfaces.
#[cfg(target_os = "linux")]
async fn reflector_loop(config: MdnsConfig) {
    use std::net::SocketAddrV4;

    info!(
        "mDNS reflector starting on interfaces: {:?}",
        config.interfaces
    );

    let socket = match create_mdns_socket(&config.interfaces) {
        Ok(s) => s,
        Err(e) => {
            warn!("mDNS reflector: failed to create socket: {}", e);
            return;
        }
    };

    let udp = match tokio::net::UdpSocket::from_std(socket) {
        Ok(u) => Arc::new(u),
        Err(e) => {
            warn!("mDNS reflector: failed to convert socket: {}", e);
            return;
        }
    };

    let mut buf = vec![0u8; 9000];
    let dest = SocketAddrV4::new(MDNS_ADDR, MDNS_PORT);

    loop {
        match udp.recv_from(&mut buf).await {
            Ok((len, _src)) => {
                if len == 0 {
                    continue;
                }
                // Re-send to multicast group
                if let Err(e) = udp.send_to(&buf[..len], dest).await {
                    warn!("mDNS reflector: send error: {}", e);
                }
            }
            Err(e) => {
                warn!("mDNS reflector: recv error: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }
}

/// Create a UDP socket bound to 0.0.0.0:5353 and join the mDNS multicast group
/// on each specified interface.
#[cfg(target_os = "linux")]
fn create_mdns_socket(
    interfaces: &[String],
) -> Result<std::net::UdpSocket, Box<dyn std::error::Error>> {
    use socket2::{Domain, Protocol, SockAddr, Socket, Type};
    use std::net::SocketAddrV4;

    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    socket.set_reuse_port(true)?;
    socket.set_nonblocking(true)?;
    socket.set_multicast_loop_v4(false)?;

    let bind_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, MDNS_PORT);
    socket.bind(&SockAddr::from(bind_addr))?;

    for iface_name in interfaces {
        if let Some(ip) = get_interface_ipv4(iface_name) {
            if let Err(e) = socket.join_multicast_v4(&MDNS_ADDR, &ip) {
                warn!(
                    "mDNS reflector: failed to join multicast on {} ({}): {}",
                    iface_name, ip, e
                );
            } else {
                info!("mDNS reflector: joined multicast on {} ({})", iface_name, ip);
            }
        } else {
            warn!(
                "mDNS reflector: could not find IPv4 address for interface {}",
                iface_name
            );
        }
    }

    Ok(socket.into())
}

/// Get the first IPv4 address of a network interface by name.
#[cfg(target_os = "linux")]
fn get_interface_ipv4(name: &str) -> Option<Ipv4Addr> {
    if let Ok(output) = std::process::Command::new("ip")
        .args(["-4", "-o", "addr", "show", name])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Format: "2: eth0    inet 192.168.1.1/24 brd ..."
        for part in stdout.split_whitespace() {
            if let Some(ip_str) = part.split('/').next() {
                if let Ok(ip) = ip_str.parse::<Ipv4Addr>() {
                    return Some(ip);
                }
            }
        }
    }
    None
}
