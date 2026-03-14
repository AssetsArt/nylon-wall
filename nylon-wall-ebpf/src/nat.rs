use crate::common::*;
use nylon_wall_common::conntrack::ConntrackKey;
use nylon_wall_common::nat::EbpfNatEntry;

/// Maximum NAT entries to evaluate per packet
pub const MAX_NAT_ENTRIES: u32 = 64;

/// Incrementally update an IP checksum when replacing old_val with new_val.
/// Uses the algorithm from RFC 1624.
#[inline(always)]
pub fn update_csum(old_csum: u16, old_val: u32, new_val: u32) -> u16 {
    let old_h = (old_val >> 16) as u32;
    let old_l = (old_val & 0xFFFF) as u32;
    let new_h = (new_val >> 16) as u32;
    let new_l = (new_val & 0xFFFF) as u32;

    let mut csum = !old_csum as u32 & 0xFFFF;
    csum = csum.wrapping_add(!old_h & 0xFFFF);
    csum = csum.wrapping_add(!old_l & 0xFFFF);
    csum = csum.wrapping_add(new_h);
    csum = csum.wrapping_add(new_l);

    // Fold carry (bounded — at most 3 iterations needed for u32 accumulator)
    csum = (csum & 0xFFFF) + (csum >> 16);
    csum = (csum & 0xFFFF) + (csum >> 16);
    csum = (csum & 0xFFFF) + (csum >> 16);

    !csum as u16
}

/// Update L4 (TCP/UDP) checksum for IP address change.
#[inline(always)]
pub fn update_l4_csum(old_csum: u16, old_ip: u32, new_ip: u32, old_port: u16, new_port: u16) -> u16 {
    let mut csum = update_csum(old_csum, old_ip, new_ip);
    csum = update_csum(csum, old_port as u32, new_port as u32);
    csum
}

/// Try to apply DNAT on ingress. Returns true if NAT was applied.
/// Rewrites destination IP and port in the packet.
#[inline(always)]
pub fn try_dnat_ingress(data: usize, data_end: usize, pkt: &PacketInfo) -> bool {
    let nat_count = unsafe {
        crate::NAT_ENTRY_COUNT
            .get(0)
            .copied()
            .unwrap_or(0)
    };

    if nat_count == 0 {
        return false;
    }

    for i in 0..MAX_NAT_ENTRIES {
        if i >= nat_count {
            break;
        }

        let entry: &EbpfNatEntry = match unsafe { crate::NAT_TABLE.get(i) } {
            Some(e) => e,
            None => break,
        };

        if entry.enabled == 0 {
            continue;
        }

        // DNAT applies on ingress (nat_type == 1)
        if entry.nat_type != 1 {
            continue;
        }

        // Protocol match
        if entry.protocol != 0 && entry.protocol != pkt.protocol {
            continue;
        }

        // Destination match (match the original destination)
        if !ip_match(pkt.dst_ip, entry.dst_ip, entry.dst_mask) {
            continue;
        }

        // Port match
        if !port_match(pkt.dst_port, entry.dst_port_start, entry.dst_port_end) {
            continue;
        }

        // Apply DNAT: rewrite destination IP and port
        let new_dst_ip = entry.translate_ip;
        let new_dst_port = if entry.translate_port_start != 0 {
            // Calculate offset within source port range and apply to translate range
            if entry.dst_port_start != 0 && entry.translate_port_end > entry.translate_port_start && pkt.dst_port >= entry.dst_port_start {
                let offset = pkt.dst_port - entry.dst_port_start;
                let range_size = entry.translate_port_end - entry.translate_port_start;
                if offset <= range_size {
                    entry.translate_port_start + offset
                } else {
                    entry.translate_port_start
                }
            } else {
                entry.translate_port_start
            }
        } else {
            pkt.dst_port
        };

        // Rewrite IP header
        let ip_base = data + ETH_HDR_LEN;
        if ip_base + IPV4_HDR_LEN > data_end {
            return false;
        }

        // Update IP checksum
        let ip_csum_offset = ip_base + 10;
        if ip_csum_offset + 2 > data_end {
            return false;
        }
        let old_csum = unsafe { u16::from_be_bytes(*((ip_csum_offset) as *const [u8; 2])) };
        let new_csum = update_csum(old_csum, pkt.dst_ip, new_dst_ip);

        unsafe {
            // Write new destination IP
            *((ip_base + 16) as *mut u32) = new_dst_ip;
            // Write new IP checksum
            *((ip_csum_offset) as *mut [u8; 2]) = new_csum.to_be_bytes();
        }

        // Update L4 header (port + checksum)
        let ihl = unsafe { (*((ip_base) as *const u8) & 0x0F) as usize * 4 };
        let transport_base = ip_base + ihl;

        if pkt.protocol == IPPROTO_TCP && transport_base + TCP_HDR_LEN <= data_end {
            unsafe {
                let l4_csum_offset = transport_base + 16;
                if l4_csum_offset + 2 <= data_end {
                    let old_l4_csum = u16::from_be_bytes(*((l4_csum_offset) as *const [u8; 2]));
                    let new_l4_csum = update_l4_csum(old_l4_csum, pkt.dst_ip, new_dst_ip, pkt.dst_port, new_dst_port);
                    *((transport_base + 2) as *mut [u8; 2]) = new_dst_port.to_be_bytes();
                    *((l4_csum_offset) as *mut [u8; 2]) = new_l4_csum.to_be_bytes();
                }
            }
        } else if pkt.protocol == IPPROTO_UDP && transport_base + UDP_HDR_LEN <= data_end {
            unsafe {
                let l4_csum_offset = transport_base + 6;
                if l4_csum_offset + 2 <= data_end {
                    let old_l4_csum = u16::from_be_bytes(*((l4_csum_offset) as *const [u8; 2]));
                    if old_l4_csum != 0 {
                        // UDP checksum is optional; only update if non-zero
                        let new_l4_csum = update_l4_csum(old_l4_csum, pkt.dst_ip, new_dst_ip, pkt.dst_port, new_dst_port);
                        *((l4_csum_offset) as *mut [u8; 2]) = new_l4_csum.to_be_bytes();
                    }
                    *((transport_base + 2) as *mut [u8; 2]) = new_dst_port.to_be_bytes();
                }
            }
        }

        // Store NAT state for return traffic
        let nat_key = ConntrackKey {
            src_ip: pkt.src_ip,
            dst_ip: new_dst_ip,
            src_port: pkt.src_port,
            dst_port: new_dst_port,
            protocol: pkt.protocol,
            _pad: [0; 3],
        };
        let nat_state = nylon_wall_common::nat::EbpfNatState {
            original_ip: pkt.dst_ip,
            original_port: pkt.dst_port,
            translated_ip: new_dst_ip,
            translated_port: new_dst_port,
            nat_type: 1, // DNAT
            _pad: [0; 3],
        };
        let _ = crate::NAT_CONNTRACK.insert(&nat_key, &nat_state, 0);

        return true;
    }

    false
}

/// Try to apply SNAT/Masquerade on egress. Returns true if NAT was applied.
/// Rewrites source IP and port in the packet.
#[inline(always)]
pub fn try_snat_egress(data: usize, data_end: usize, pkt: &PacketInfo) -> bool {
    let nat_count = unsafe {
        crate::NAT_ENTRY_COUNT
            .get(0)
            .copied()
            .unwrap_or(0)
    };

    if nat_count == 0 {
        return false;
    }

    for i in 0..MAX_NAT_ENTRIES {
        if i >= nat_count {
            break;
        }

        let entry: &EbpfNatEntry = match unsafe { crate::NAT_TABLE.get(i) } {
            Some(e) => e,
            None => break,
        };

        if entry.enabled == 0 {
            continue;
        }

        // SNAT (0) or Masquerade (2) applies on egress
        if entry.nat_type != 0 && entry.nat_type != 2 {
            continue;
        }

        // Protocol match
        if entry.protocol != 0 && entry.protocol != pkt.protocol {
            continue;
        }

        // Source match
        if !ip_match(pkt.src_ip, entry.src_ip, entry.src_mask) {
            continue;
        }

        // For masquerade, use the interface IP from MASQUERADE_IP map
        let new_src_ip = if entry.nat_type == 2 {
            unsafe {
                crate::MASQUERADE_IP
                    .get(0)
                    .copied()
                    .unwrap_or(entry.translate_ip)
            }
        } else {
            entry.translate_ip
        };

        if new_src_ip == 0 {
            continue;
        }

        let new_src_port = if entry.translate_port_start != 0 {
            // Calculate offset within source port range and apply to translate range
            if entry.dst_port_start != 0 && entry.translate_port_end > entry.translate_port_start && pkt.src_port >= entry.dst_port_start {
                let offset = pkt.src_port - entry.dst_port_start;
                let range_size = entry.translate_port_end - entry.translate_port_start;
                if offset <= range_size {
                    entry.translate_port_start + offset
                } else {
                    entry.translate_port_start
                }
            } else {
                entry.translate_port_start
            }
        } else {
            pkt.src_port
        };

        // Rewrite IP header
        let ip_base = data + ETH_HDR_LEN;
        if ip_base + IPV4_HDR_LEN > data_end {
            return false;
        }

        let ip_csum_offset = ip_base + 10;
        if ip_csum_offset + 2 > data_end {
            return false;
        }
        let old_csum = unsafe { u16::from_be_bytes(*((ip_csum_offset) as *const [u8; 2])) };
        let new_csum = update_csum(old_csum, pkt.src_ip, new_src_ip);

        unsafe {
            // Write new source IP
            *((ip_base + 12) as *mut u32) = new_src_ip;
            // Write new IP checksum
            *((ip_csum_offset) as *mut [u8; 2]) = new_csum.to_be_bytes();
        }

        // Update L4 header
        let ihl = unsafe { (*((ip_base) as *const u8) & 0x0F) as usize * 4 };
        let transport_base = ip_base + ihl;

        if pkt.protocol == IPPROTO_TCP && transport_base + TCP_HDR_LEN <= data_end {
            unsafe {
                let l4_csum_offset = transport_base + 16;
                if l4_csum_offset + 2 <= data_end {
                    let old_l4_csum = u16::from_be_bytes(*((l4_csum_offset) as *const [u8; 2]));
                    let new_l4_csum = update_l4_csum(old_l4_csum, pkt.src_ip, new_src_ip, pkt.src_port, new_src_port);
                    *((transport_base) as *mut [u8; 2]) = new_src_port.to_be_bytes();
                    *((l4_csum_offset) as *mut [u8; 2]) = new_l4_csum.to_be_bytes();
                }
            }
        } else if pkt.protocol == IPPROTO_UDP && transport_base + UDP_HDR_LEN <= data_end {
            unsafe {
                let l4_csum_offset = transport_base + 6;
                if l4_csum_offset + 2 <= data_end {
                    let old_l4_csum = u16::from_be_bytes(*((l4_csum_offset) as *const [u8; 2]));
                    if old_l4_csum != 0 {
                        let new_l4_csum = update_l4_csum(old_l4_csum, pkt.src_ip, new_src_ip, pkt.src_port, new_src_port);
                        *((l4_csum_offset) as *mut [u8; 2]) = new_l4_csum.to_be_bytes();
                    }
                    *((transport_base) as *mut [u8; 2]) = new_src_port.to_be_bytes();
                }
            }
        }

        // Store NAT state for return traffic
        let nat_key = ConntrackKey {
            src_ip: new_src_ip,
            dst_ip: pkt.dst_ip,
            src_port: new_src_port,
            dst_port: pkt.dst_port,
            protocol: pkt.protocol,
            _pad: [0; 3],
        };
        let nat_state = nylon_wall_common::nat::EbpfNatState {
            original_ip: pkt.src_ip,
            original_port: pkt.src_port,
            translated_ip: new_src_ip,
            translated_port: new_src_port,
            nat_type: 0, // SNAT
            _pad: [0; 3],
        };
        let _ = crate::NAT_CONNTRACK.insert(&nat_key, &nat_state, 0);

        return true;
    }

    false
}

/// Reverse NAT for return traffic on ingress (undo SNAT).
/// If a packet's destination matches a SNAT translation, rewrite dst back to original.
#[inline(always)]
pub fn try_reverse_nat_ingress(data: usize, data_end: usize, pkt: &PacketInfo) -> bool {
    // Look up by the packet's actual addresses (translated connection)
    let nat_key = ConntrackKey {
        src_ip: pkt.dst_ip,  // Reversed: this was the SNAT'd source
        dst_ip: pkt.src_ip,
        src_port: pkt.dst_port,
        dst_port: pkt.src_port,
        protocol: pkt.protocol,
        _pad: [0; 3],
    };

    let state = match unsafe { crate::NAT_CONNTRACK.get(&nat_key) } {
        Some(s) => *s,
        None => return false,
    };

    // This is a return packet for SNAT - rewrite destination back to original
    if state.nat_type == 0 {
        let ip_base = data + ETH_HDR_LEN;
        if ip_base + IPV4_HDR_LEN > data_end {
            return false;
        }

        let ip_csum_offset = ip_base + 10;
        if ip_csum_offset + 2 > data_end {
            return false;
        }
        let old_csum = unsafe { u16::from_be_bytes(*((ip_csum_offset) as *const [u8; 2])) };
        let new_csum = update_csum(old_csum, pkt.dst_ip, state.original_ip);

        unsafe {
            *((ip_base + 16) as *mut u32) = state.original_ip;
            *((ip_csum_offset) as *mut [u8; 2]) = new_csum.to_be_bytes();
        }

        // Update L4
        let ihl = unsafe { (*((ip_base) as *const u8) & 0x0F) as usize * 4 };
        let transport_base = ip_base + ihl;

        if pkt.protocol == IPPROTO_TCP && transport_base + TCP_HDR_LEN <= data_end {
            unsafe {
                let l4_csum_offset = transport_base + 16;
                if l4_csum_offset + 2 <= data_end {
                    let old_l4_csum = u16::from_be_bytes(*((l4_csum_offset) as *const [u8; 2]));
                    let new_l4_csum = update_l4_csum(old_l4_csum, pkt.dst_ip, state.original_ip, pkt.dst_port, state.original_port);
                    *((transport_base + 2) as *mut [u8; 2]) = state.original_port.to_be_bytes();
                    *((l4_csum_offset) as *mut [u8; 2]) = new_l4_csum.to_be_bytes();
                }
            }
        } else if pkt.protocol == IPPROTO_UDP && transport_base + UDP_HDR_LEN <= data_end {
            unsafe {
                let l4_csum_offset = transport_base + 6;
                if l4_csum_offset + 2 <= data_end {
                    let old_l4_csum = u16::from_be_bytes(*((l4_csum_offset) as *const [u8; 2]));
                    if old_l4_csum != 0 {
                        let new_l4_csum = update_l4_csum(old_l4_csum, pkt.dst_ip, state.original_ip, pkt.dst_port, state.original_port);
                        *((l4_csum_offset) as *mut [u8; 2]) = new_l4_csum.to_be_bytes();
                    }
                    *((transport_base + 2) as *mut [u8; 2]) = state.original_port.to_be_bytes();
                }
            }
        }

        return true;
    }

    false
}

/// Reverse NAT for return traffic on egress (undo DNAT).
#[inline(always)]
pub fn try_reverse_nat_egress(data: usize, data_end: usize, pkt: &PacketInfo) -> bool {
    let nat_key = ConntrackKey {
        src_ip: pkt.dst_ip,
        dst_ip: pkt.src_ip,
        src_port: pkt.dst_port,
        dst_port: pkt.src_port,
        protocol: pkt.protocol,
        _pad: [0; 3],
    };

    let state = match unsafe { crate::NAT_CONNTRACK.get(&nat_key) } {
        Some(s) => *s,
        None => return false,
    };

    // This is a return packet for DNAT - rewrite source back to original
    if state.nat_type == 1 {
        let ip_base = data + ETH_HDR_LEN;
        if ip_base + IPV4_HDR_LEN > data_end {
            return false;
        }

        let ip_csum_offset = ip_base + 10;
        if ip_csum_offset + 2 > data_end {
            return false;
        }
        let old_csum = unsafe { u16::from_be_bytes(*((ip_csum_offset) as *const [u8; 2])) };
        let new_csum = update_csum(old_csum, pkt.src_ip, state.original_ip);

        unsafe {
            *((ip_base + 12) as *mut u32) = state.original_ip;
            *((ip_csum_offset) as *mut [u8; 2]) = new_csum.to_be_bytes();
        }

        let ihl = unsafe { (*((ip_base) as *const u8) & 0x0F) as usize * 4 };
        let transport_base = ip_base + ihl;

        if pkt.protocol == IPPROTO_TCP && transport_base + TCP_HDR_LEN <= data_end {
            unsafe {
                let l4_csum_offset = transport_base + 16;
                if l4_csum_offset + 2 <= data_end {
                    let old_l4_csum = u16::from_be_bytes(*((l4_csum_offset) as *const [u8; 2]));
                    let new_l4_csum = update_l4_csum(old_l4_csum, pkt.src_ip, state.original_ip, pkt.src_port, state.original_port);
                    *((transport_base) as *mut [u8; 2]) = state.original_port.to_be_bytes();
                    *((l4_csum_offset) as *mut [u8; 2]) = new_l4_csum.to_be_bytes();
                }
            }
        } else if pkt.protocol == IPPROTO_UDP && transport_base + UDP_HDR_LEN <= data_end {
            unsafe {
                let l4_csum_offset = transport_base + 6;
                if l4_csum_offset + 2 <= data_end {
                    let old_l4_csum = u16::from_be_bytes(*((l4_csum_offset) as *const [u8; 2]));
                    if old_l4_csum != 0 {
                        let new_l4_csum = update_l4_csum(old_l4_csum, pkt.src_ip, state.original_ip, pkt.src_port, state.original_port);
                        *((l4_csum_offset) as *mut [u8; 2]) = new_l4_csum.to_be_bytes();
                    }
                    *((transport_base) as *mut [u8; 2]) = state.original_port.to_be_bytes();
                }
            }
        }

        return true;
    }

    false
}
