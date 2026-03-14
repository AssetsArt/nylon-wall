use aya_ebpf::programs::TcContext;

use nylon_wall_common::conntrack::{ConntrackEntry, ConntrackKey};
use nylon_wall_common::log::EbpfPacketEvent;
use nylon_wall_common::rule::EbpfRule;

use crate::common::*;

// TC action constants
const TC_ACT_OK: i32 = 0;    // Pass
const TC_ACT_SHOT: i32 = 2;  // Drop

/// Process an egress (outgoing) packet through the firewall.
///
/// Note: NAT (SNAT/Masquerade/reverse-DNAT) is handled via separate eBPF
/// programs chained after this one. See nat.rs for NAT processing logic.
pub fn process_egress(ctx: &TcContext) -> Result<i32, ()> {
    let data = ctx.data();
    let data_end = ctx.data_end();

    let pkt = match parse_packet(data, data_end) {
        Some(p) => p,
        None => return Ok(TC_ACT_OK),
    };

    let now = unsafe { aya_ebpf::helpers::bpf_ktime_get_ns() };

    // Save original packet info for conntrack BEFORE NAT modifies the packet.
    // This is critical: ingress stores conntrack with post-DNAT addresses
    // (e.g. client:port → server:9450), so egress must look up conntrack
    // with pre-reverse-NAT addresses (server:9450 → client:port) to match.
    let orig_pkt = pkt;

    // Apply NAT:
    // 1. Reverse DNAT for return traffic (undo inbound DNAT)
    // 2. SNAT/Masquerade for outbound traffic
    let _nat_applied = crate::nat::try_reverse_nat_egress(data, data_end, &pkt)
        || crate::nat::try_snat_egress(data, data_end, &pkt);

    // Re-parse packet if NAT was applied (for firewall rules and wire output)
    let pkt = if _nat_applied {
        match parse_packet(data, data_end) {
            Some(p) => p,
            None => return Ok(TC_ACT_OK),
        }
    } else {
        pkt
    };

    // Update global metrics
    if let Some(m) = unsafe { crate::METRICS.get_ptr_mut(0) } {
        unsafe {
            (*m).packets_total += 1;
            (*m).bytes_total += pkt.pkt_len as u64;
        }
    }

    // SNI filtering: check TLS ClientHello for blocked domains
    let sni_enabled = unsafe { crate::SNI_ENABLED.get(0).copied().unwrap_or(0) };
    if sni_enabled == 1 && pkt.protocol == IPPROTO_TCP && pkt.dst_port == 443 {
        // TC skb may have TCP payload in non-linear fragments.
        // Pull first 512 bytes into the linear buffer so we can parse
        // the TLS ClientHello up to the SNI extension via direct access.
        let _ = ctx.pull_data(512);
        // Re-read data pointers — pull_data may reallocate the buffer.
        let data = ctx.data();
        let data_end = ctx.data_end();

        let ip_base = data + ETH_HDR_LEN;
        if ip_base + 1 > data_end {
            return Ok(TC_ACT_OK);
        }
        let ihl = unsafe { (*((ip_base) as *const u8) & 0x0F) as usize * 4 };
        let transport_base = ip_base + ihl;

        if crate::tls::check_sni_block(data, data_end, transport_base) {
            if let Some(m) = unsafe { crate::METRICS.get_ptr_mut(0) } {
                unsafe { (*m).packets_dropped += 1 };
            }
            return Ok(TC_ACT_SHOT);
        }
    }

    // Use ORIGINAL (pre-reverse-NAT) addresses for conntrack keys so they
    // match the ingress post-DNAT conntrack entries.
    let fwd_key = ConntrackKey {
        src_ip: orig_pkt.src_ip, dst_ip: orig_pkt.dst_ip,
        src_port: orig_pkt.src_port, dst_port: orig_pkt.dst_port,
        protocol: orig_pkt.protocol, _pad: [0; 3],
    };
    let rev_key = ConntrackKey {
        src_ip: orig_pkt.dst_ip, dst_ip: orig_pkt.src_ip,
        src_port: orig_pkt.dst_port, dst_port: orig_pkt.src_port,
        protocol: orig_pkt.protocol, _pad: [0; 3],
    };

    // Check conntrack: if reply to established inbound connection, pass
    if let Some(entry) = unsafe { crate::CONNTRACK.get(&rev_key) } {
        if entry.state == 1 || entry.state == 0 {
            if let Some(entry_mut) = unsafe { crate::CONNTRACK.get_ptr_mut(&rev_key) } {
                unsafe {
                    if pkt.tcp_flags & (TCP_FIN | TCP_RST) != 0 {
                        (*entry_mut).state = 4;
                    } else {
                        (*entry_mut).state = 1;
                    }
                    (*entry_mut).packets_out += 1;
                    (*entry_mut).bytes_out += pkt.pkt_len as u64;
                    (*entry_mut).last_seen = now;
                }
            }
            if let Some(m) = unsafe { crate::METRICS.get_ptr_mut(0) } {
                unsafe { (*m).packets_allowed += 1 };
            }
            return Ok(TC_ACT_OK);
        }
    }

    let existing_state = unsafe {
        crate::CONNTRACK.get(&fwd_key).map(|e| e.state).unwrap_or(255)
    };

    // Evaluate egress rules
    let rule_count = unsafe {
        crate::EGRESS_RULE_COUNT.get(0).copied().unwrap_or(0)
    };

    let mut matched_action: u8 = 0;
    let mut matched_rule_id: u32 = 0;
    let mut should_log = false;

    for i in 0..MAX_RULES {
        if i >= rule_count {
            break;
        }

        let rule: &EbpfRule = match unsafe { crate::EGRESS_RULES.get(i) } {
            Some(r) => r,
            None => break,
        };

        if rule.enabled == 0 { continue; }
        if rule.direction != 1 { continue; }
        if rule.protocol != 0 && rule.protocol != pkt.protocol { continue; }
        if !ip_match(pkt.src_ip, rule.src_ip, rule.src_mask) { continue; }
        if !ip_match(pkt.dst_ip, rule.dst_ip, rule.dst_mask) { continue; }
        if !port_match(pkt.src_port, rule.src_port_start, rule.src_port_end) { continue; }
        if !port_match(pkt.dst_port, rule.dst_port_start, rule.dst_port_end) { continue; }

        matched_action = rule.action;
        matched_rule_id = rule.id;
        should_log = rule.action == 1 || rule.action == 2;

        // Rate limiting (token bucket)
        if rule.action == 3 && rule.rate_limit_pps > 0 {
            let rate = rule.rate_limit_pps as u64;
            let ns_per_token = 1_000_000_000 / rate.max(1);

            match unsafe { crate::RATE_LIMIT.get_ptr_mut(&rule.id) } {
                Some(state) => unsafe {
                    let elapsed = now.saturating_sub((*state).last_update);
                    let new_tokens = elapsed / ns_per_token;
                    let tokens = ((*state).tokens + new_tokens).min(rate * 2);
                    if tokens > 0 {
                        (*state).tokens = tokens - 1;
                        (*state).last_update = now;
                        matched_action = 0;
                    } else {
                        matched_action = 1;
                        should_log = true;
                    }
                },
                None => {
                    let state = nylon_wall_common::log::EbpfRateState {
                        tokens: rate, last_update: now,
                    };
                    let _ = crate::RATE_LIMIT.insert(&rule.id, &state, 0);
                    matched_action = 0;
                }
            }
        }

        if let Some(count) = unsafe { crate::RULE_HITS.get_ptr_mut(&rule.id) } {
            unsafe { *count += 1 };
        } else {
            let one: u64 = 1;
            let _ = crate::RULE_HITS.insert(&rule.id, &one, 0);
        }

        break;
    }

    let action = if matched_action == 1 { TC_ACT_SHOT } else { TC_ACT_OK };

    // Update conntrack
    if action == TC_ACT_OK {
        if let Some(m) = unsafe { crate::METRICS.get_ptr_mut(0) } {
            unsafe { (*m).packets_allowed += 1 };
        }
        if existing_state == 255 {
            let entry = ConntrackEntry {
                state: 0, _pad: [0; 3],
                packets_in: 0, packets_out: 1,
                bytes_in: 0, bytes_out: pkt.pkt_len as u64,
                last_seen: now, timeout: 300, _pad2: 0,
            };
            let _ = crate::CONNTRACK.insert(&fwd_key, &entry, 0);
        } else if let Some(entry_mut) = unsafe { crate::CONNTRACK.get_ptr_mut(&fwd_key) } {
            unsafe {
                if pkt.tcp_flags & (TCP_FIN | TCP_RST) != 0 { (*entry_mut).state = 4; }
                (*entry_mut).packets_out += 1;
                (*entry_mut).bytes_out += pkt.pkt_len as u64;
                (*entry_mut).last_seen = now;
            }
        }
    } else {
        if let Some(m) = unsafe { crate::METRICS.get_ptr_mut(0) } {
            unsafe { (*m).packets_dropped += 1 };
        }
    }

    // Emit perf event
    if should_log || matched_action == 1 {
        let event = EbpfPacketEvent {
            timestamp: now, src_ip: pkt.src_ip, dst_ip: pkt.dst_ip,
            src_port: pkt.src_port, dst_port: pkt.dst_port,
            protocol: pkt.protocol, action: matched_action,
            rule_id: matched_rule_id, ifindex: 0, bytes: pkt.pkt_len,
        };
        let _ = unsafe { crate::EVENTS.output(ctx, &event, 0) };
    }

    Ok(action)
}
