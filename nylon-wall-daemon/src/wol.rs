use std::net::UdpSocket;

/// Parse a MAC address string "aa:bb:cc:dd:ee:ff" or "aa-bb-cc-dd-ee-ff" into 6 bytes.
pub fn parse_mac(mac: &str) -> Result<[u8; 6], String> {
    let parts: Vec<&str> = mac.split(|c| c == ':' || c == '-').collect();
    if parts.len() != 6 {
        return Err(format!("Invalid MAC address: {}", mac));
    }
    let mut bytes = [0u8; 6];
    for (i, part) in parts.iter().enumerate() {
        bytes[i] = u8::from_str_radix(part, 16)
            .map_err(|_| format!("Invalid hex in MAC: {}", part))?;
    }
    Ok(bytes)
}

/// Build a Wake-on-LAN magic packet: 6 bytes of 0xFF followed by the MAC repeated 16 times.
fn build_magic_packet(mac: &[u8; 6]) -> Vec<u8> {
    let mut packet = Vec::with_capacity(102);
    // 6 bytes of 0xFF
    packet.extend_from_slice(&[0xFF; 6]);
    // MAC repeated 16 times
    for _ in 0..16 {
        packet.extend_from_slice(mac);
    }
    packet
}

/// Send a WOL magic packet to the broadcast address on port 9.
pub fn send_magic_packet(mac_str: &str) -> Result<(), String> {
    let mac = parse_mac(mac_str)?;
    let packet = build_magic_packet(&mac);

    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    socket.set_broadcast(true).map_err(|e| e.to_string())?;
    socket
        .send_to(&packet, "255.255.255.255:9")
        .map_err(|e| e.to_string())?;

    tracing::info!("WOL magic packet sent to {}", mac_str);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mac() {
        let mac = parse_mac("aa:bb:cc:dd:ee:ff").unwrap();
        assert_eq!(mac, [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
    }

    #[test]
    fn test_parse_mac_dash() {
        let mac = parse_mac("AA-BB-CC-DD-EE-FF").unwrap();
        assert_eq!(mac, [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
    }

    #[test]
    fn test_parse_mac_invalid() {
        assert!(parse_mac("invalid").is_err());
        assert!(parse_mac("aa:bb:cc:dd:ee").is_err());
        assert!(parse_mac("aa:bb:cc:dd:ee:gg").is_err());
    }

    #[test]
    fn test_magic_packet_length() {
        let mac = [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff];
        let packet = build_magic_packet(&mac);
        assert_eq!(packet.len(), 102); // 6 + 16*6
        assert_eq!(&packet[..6], &[0xFF; 6]);
        assert_eq!(&packet[6..12], &mac);
        assert_eq!(&packet[96..102], &mac);
    }
}
