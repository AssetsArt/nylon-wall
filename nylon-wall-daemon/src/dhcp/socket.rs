use std::net::SocketAddr;
use tokio::net::UdpSocket;

/// Create a UDP socket for the DHCP server (port 67) bound to a specific interface.
pub async fn create_server_socket(interface: &str) -> anyhow::Result<UdpSocket> {
    create_dhcp_socket(67, interface).await
}

/// Create a UDP socket for the DHCP client (port 68) bound to a specific interface.
pub async fn create_client_socket(interface: &str) -> anyhow::Result<UdpSocket> {
    create_dhcp_socket(68, interface).await
}

async fn create_dhcp_socket(port: u16, interface: &str) -> anyhow::Result<UdpSocket> {
    use socket2::{Domain, Protocol, Socket, Type};

    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    socket.set_broadcast(true)?;
    socket.set_nonblocking(true)?;

    // Bind to specific interface using SO_BINDTODEVICE
    socket.bind_device(Some(interface.as_bytes()))?;

    // Bind to 0.0.0.0:{port}
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    socket.bind(&addr.into())?;

    let std_socket: std::net::UdpSocket = socket.into();
    let tokio_socket = UdpSocket::from_std(std_socket)?;

    Ok(tokio_socket)
}
