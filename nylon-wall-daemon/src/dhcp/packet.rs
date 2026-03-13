use std::net::Ipv4Addr;
use dhcproto::{Decodable, Encodable};
use dhcproto::v4::{self, DhcpOption, Flags, Message, MessageType, Opcode};
use nylon_wall_common::dhcp::DhcpPool;

/// Parsed DHCP message with convenience accessors.
pub struct DhcpMessage {
    inner: Message,
}

impl DhcpMessage {
    /// Parse raw bytes into a DhcpMessage.
    pub fn parse(buf: &[u8]) -> anyhow::Result<Self> {
        let msg = Message::from_bytes(buf)?;
        Ok(Self { inner: msg })
    }

    /// Get the DHCP message type (DISCOVER, OFFER, REQUEST, etc.).
    pub fn message_type(&self) -> Option<MessageType> {
        self.inner.opts().msg_type()
    }

    /// Get the client MAC address as a formatted string.
    pub fn client_mac(&self) -> String {
        let chaddr = self.inner.chaddr();
        super::mac_to_string(&chaddr[..6])
    }

    /// Get the raw client hardware address bytes.
    pub fn client_mac_bytes(&self) -> [u8; 6] {
        let chaddr = self.inner.chaddr();
        let mut mac = [0u8; 6];
        mac.copy_from_slice(&chaddr[..6]);
        mac
    }

    /// Get the transaction ID.
    pub fn xid(&self) -> u32 {
        self.inner.xid()
    }

    /// Get the requested IP address (option 50).
    pub fn requested_ip(&self) -> Option<Ipv4Addr> {
        self.inner
            .opts()
            .get(v4::OptionCode::RequestedIpAddress)
            .and_then(|opt| {
                if let DhcpOption::RequestedIpAddress(ip) = opt {
                    Some(*ip)
                } else {
                    None
                }
            })
    }

    /// Get the server identifier (option 54).
    pub fn server_identifier(&self) -> Option<Ipv4Addr> {
        self.inner
            .opts()
            .get(v4::OptionCode::ServerIdentifier)
            .and_then(|opt| {
                if let DhcpOption::ServerIdentifier(ip) = opt {
                    Some(*ip)
                } else {
                    None
                }
            })
    }

    /// Get the client hostname (option 12).
    pub fn hostname(&self) -> Option<String> {
        self.inner
            .opts()
            .get(v4::OptionCode::Hostname)
            .and_then(|opt| {
                if let DhcpOption::Hostname(name) = opt {
                    Some(name.clone())
                } else {
                    None
                }
            })
    }

    /// Get the client's existing IP (ciaddr) if it has one.
    pub fn ciaddr(&self) -> Ipv4Addr {
        self.inner.ciaddr()
    }

    /// Get the gateway/relay agent IP (giaddr).
    pub fn giaddr(&self) -> Ipv4Addr {
        self.inner.giaddr()
    }

    /// Get the inner message flags.
    pub fn flags(&self) -> Flags {
        self.inner.flags()
    }

    /// Get the offered/assigned IP (yiaddr).
    pub fn inner_yiaddr(&self) -> u32 {
        u32::from(self.inner.yiaddr())
    }

    /// Get subnet mask from options.
    pub fn subnet_mask(&self) -> Option<Ipv4Addr> {
        self.inner
            .opts()
            .get(v4::OptionCode::SubnetMask)
            .and_then(|opt| {
                if let DhcpOption::SubnetMask(mask) = opt {
                    Some(*mask)
                } else {
                    None
                }
            })
    }

    /// Get the first router/gateway from options.
    pub fn router(&self) -> Option<Ipv4Addr> {
        self.inner
            .opts()
            .get(v4::OptionCode::Router)
            .and_then(|opt| {
                if let DhcpOption::Router(routers) = opt {
                    routers.first().copied()
                } else {
                    None
                }
            })
    }

    /// Get DNS servers from options.
    pub fn dns_servers(&self) -> Vec<Ipv4Addr> {
        self.inner
            .opts()
            .get(v4::OptionCode::DomainNameServer)
            .map(|opt| {
                if let DhcpOption::DomainNameServer(servers) = opt {
                    servers.clone()
                } else {
                    Vec::new()
                }
            })
            .unwrap_or_default()
    }

    /// Get lease time from options.
    pub fn lease_time(&self) -> Option<u32> {
        self.inner
            .opts()
            .get(v4::OptionCode::AddressLeaseTime)
            .and_then(|opt| {
                if let DhcpOption::AddressLeaseTime(time) = opt {
                    Some(*time)
                } else {
                    None
                }
            })
    }
}

/// Build a DHCPOFFER response.
pub fn build_offer(
    request: &DhcpMessage,
    offer_ip: Ipv4Addr,
    pool: &DhcpPool,
    server_ip: Ipv4Addr,
) -> Vec<u8> {
    let mut msg = Message::default();
    msg.set_opcode(Opcode::BootReply)
        .set_xid(request.xid())
        .set_flags(request.flags())
        .set_yiaddr(offer_ip)
        .set_siaddr(server_ip)
        .set_giaddr(request.giaddr())
        .set_chaddr(&request.inner.chaddr());

    let opts = msg.opts_mut();
    opts.insert(DhcpOption::MessageType(MessageType::Offer));
    opts.insert(DhcpOption::ServerIdentifier(server_ip));
    opts.insert(DhcpOption::AddressLeaseTime(pool.lease_time));

    // Subnet mask from CIDR
    if let Some(mask) = subnet_mask_from_cidr(&pool.subnet) {
        opts.insert(DhcpOption::SubnetMask(mask));
    }

    // Gateway
    if let Some(ref gw) = pool.gateway {
        if let Ok(gw_ip) = gw.parse::<Ipv4Addr>() {
            opts.insert(DhcpOption::Router(vec![gw_ip]));
        }
    }

    // DNS servers
    let dns_ips: Vec<Ipv4Addr> = pool
        .dns_servers
        .iter()
        .filter_map(|s| s.parse::<Ipv4Addr>().ok())
        .collect();
    if !dns_ips.is_empty() {
        opts.insert(DhcpOption::DomainNameServer(dns_ips));
    }

    // Domain name
    if let Some(ref domain) = pool.domain_name {
        opts.insert(DhcpOption::DomainName(domain.clone()));
    }

    msg.to_vec().unwrap_or_default()
}

/// Build a DHCPACK response.
pub fn build_ack(
    request: &DhcpMessage,
    assigned_ip: Ipv4Addr,
    pool: &DhcpPool,
    server_ip: Ipv4Addr,
) -> Vec<u8> {
    let mut msg = Message::default();
    msg.set_opcode(Opcode::BootReply)
        .set_xid(request.xid())
        .set_flags(request.flags())
        .set_yiaddr(assigned_ip)
        .set_siaddr(server_ip)
        .set_giaddr(request.giaddr())
        .set_chaddr(&request.inner.chaddr());

    let opts = msg.opts_mut();
    opts.insert(DhcpOption::MessageType(MessageType::Ack));
    opts.insert(DhcpOption::ServerIdentifier(server_ip));
    opts.insert(DhcpOption::AddressLeaseTime(pool.lease_time));

    if let Some(mask) = subnet_mask_from_cidr(&pool.subnet) {
        opts.insert(DhcpOption::SubnetMask(mask));
    }
    if let Some(ref gw) = pool.gateway {
        if let Ok(gw_ip) = gw.parse::<Ipv4Addr>() {
            opts.insert(DhcpOption::Router(vec![gw_ip]));
        }
    }
    let dns_ips: Vec<Ipv4Addr> = pool
        .dns_servers
        .iter()
        .filter_map(|s| s.parse::<Ipv4Addr>().ok())
        .collect();
    if !dns_ips.is_empty() {
        opts.insert(DhcpOption::DomainNameServer(dns_ips));
    }
    if let Some(ref domain) = pool.domain_name {
        opts.insert(DhcpOption::DomainName(domain.clone()));
    }

    msg.to_vec().unwrap_or_default()
}

/// Build a DHCPNAK response.
pub fn build_nak(request: &DhcpMessage, server_ip: Ipv4Addr) -> Vec<u8> {
    let mut msg = Message::default();
    msg.set_opcode(Opcode::BootReply)
        .set_xid(request.xid())
        .set_flags(request.flags())
        .set_giaddr(request.giaddr())
        .set_chaddr(&request.inner.chaddr());

    let opts = msg.opts_mut();
    opts.insert(DhcpOption::MessageType(MessageType::Nak));
    opts.insert(DhcpOption::ServerIdentifier(server_ip));

    msg.to_vec().unwrap_or_default()
}

/// Build a DHCPDISCOVER packet (for client).
pub fn build_discover(mac: &[u8; 6], xid: u32, hostname: Option<&str>) -> Vec<u8> {
    let mut msg = Message::default();
    let mut chaddr = [0u8; 16];
    chaddr[..6].copy_from_slice(mac);
    msg.set_opcode(Opcode::BootRequest)
        .set_xid(xid)
        .set_flags(Flags::from(0x8000u16)) // broadcast flag
        .set_chaddr(&chaddr);

    let opts = msg.opts_mut();
    opts.insert(DhcpOption::MessageType(MessageType::Discover));
    if let Some(name) = hostname {
        opts.insert(DhcpOption::Hostname(name.to_string()));
    }
    // Request common options
    opts.insert(DhcpOption::ParameterRequestList(vec![
        v4::OptionCode::SubnetMask,
        v4::OptionCode::Router,
        v4::OptionCode::DomainNameServer,
        v4::OptionCode::DomainName,
    ]));

    msg.to_vec().unwrap_or_default()
}

/// Build a DHCPREQUEST packet (for client).
pub fn build_request(
    mac: &[u8; 6],
    xid: u32,
    server_ip: Ipv4Addr,
    offered_ip: Ipv4Addr,
) -> Vec<u8> {
    let mut msg = Message::default();
    let mut chaddr = [0u8; 16];
    chaddr[..6].copy_from_slice(mac);
    msg.set_opcode(Opcode::BootRequest)
        .set_xid(xid)
        .set_flags(Flags::from(0x8000u16))
        .set_chaddr(&chaddr);

    let opts = msg.opts_mut();
    opts.insert(DhcpOption::MessageType(MessageType::Request));
    opts.insert(DhcpOption::ServerIdentifier(server_ip));
    opts.insert(DhcpOption::RequestedIpAddress(offered_ip));
    opts.insert(DhcpOption::ParameterRequestList(vec![
        v4::OptionCode::SubnetMask,
        v4::OptionCode::Router,
        v4::OptionCode::DomainNameServer,
        v4::OptionCode::DomainName,
    ]));

    msg.to_vec().unwrap_or_default()
}

/// Build a DHCPRELEASE packet (for client).
pub fn build_release(
    mac: &[u8; 6],
    xid: u32,
    client_ip: Ipv4Addr,
    server_ip: Ipv4Addr,
) -> Vec<u8> {
    let mut msg = Message::default();
    let mut chaddr = [0u8; 16];
    chaddr[..6].copy_from_slice(mac);
    msg.set_opcode(Opcode::BootRequest)
        .set_xid(xid)
        .set_ciaddr(client_ip)
        .set_chaddr(&chaddr);

    let opts = msg.opts_mut();
    opts.insert(DhcpOption::MessageType(MessageType::Release));
    opts.insert(DhcpOption::ServerIdentifier(server_ip));

    msg.to_vec().unwrap_or_default()
}

/// Extract subnet mask from CIDR notation (e.g., "192.168.1.0/24" -> 255.255.255.0).
fn subnet_mask_from_cidr(cidr: &str) -> Option<Ipv4Addr> {
    let parts: Vec<&str> = cidr.split('/').collect();
    if parts.len() != 2 {
        return None;
    }
    let prefix_len: u32 = parts[1].parse().ok()?;
    if prefix_len > 32 {
        return None;
    }
    let mask = if prefix_len == 0 {
        0u32
    } else {
        !0u32 << (32 - prefix_len)
    };
    Some(Ipv4Addr::from(mask))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subnet_mask_from_cidr() {
        assert_eq!(
            subnet_mask_from_cidr("192.168.1.0/24"),
            Some(Ipv4Addr::new(255, 255, 255, 0))
        );
        assert_eq!(
            subnet_mask_from_cidr("10.0.0.0/8"),
            Some(Ipv4Addr::new(255, 0, 0, 0))
        );
        assert_eq!(
            subnet_mask_from_cidr("172.16.0.0/16"),
            Some(Ipv4Addr::new(255, 255, 0, 0))
        );
    }
}
