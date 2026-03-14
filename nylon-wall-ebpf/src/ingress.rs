use aya_ebpf::{bindings::xdp_action, helpers::bpf_ktime_get_ns, programs::XdpContext};

use nylon_wall_common::conntrack::{ConntrackEntry, ConntrackKey};
use nylon_wall_common::log::EbpfPacketEvent;
use nylon_wall_common::rule::EbpfRule;

use crate::common::*;

// XDP action constants
const XDP_PASS: u32 = xdp_action::XDP_PASS;
const XDP_DROP: u32 = xdp_action::XDP_DROP;

/// Process an ingress (incoming) packet through the firewall.
///
/// Steps:
/// 1. Parse Ethernet → IPv4 → TCP/UDP headers
/// 2. Update global metrics
/// 3. Check zone-based policies
/// 4. Check connection tracking (ESTABLISHED connections pass immediately)
/// 5. Evaluate ingress rules in priority order
/// 6. Apply rate limiting if needed
/// 7. Update conntrack state
/// 8. Emit perf event for logging if needed
///
/// Note: NAT (DNAT/reverse-SNAT) is handled via separate eBPF programs
/// chained before this one. See nat.rs for NAT processing logic.
pub fn process_ingress(ctx: &XdpContext) -> Result<u32, ()> {
    let data = ctx.data();
    let data_end = ctx.data_end();

    // Parse packet headers
    let pkt = match parse_packet(data, data_end) {
        Some(p) => p,
        None => return Ok(XDP_PASS), // Non-IPv4 or malformed → pass
    };

    let now = unsafe { bpf_ktime_get_ns() };

    // Apply NAT before firewall rules:
    // 1. Reverse SNAT for return traffic (undo outbound SNAT)
    // 2. DNAT for inbound port forwarding
    let _nat_applied = crate::nat::try_reverse_nat_ingress(data, data_end, &pkt)
        || crate::nat::try_dnat_ingress(data, data_end, &pkt);

    // Re-parse packet if NAT was applied (IP/port may have changed)
    let pkt = if _nat_applied {
        match parse_packet(data, data_end) {
            Some(p) => p,
            None => return Ok(XDP_PASS),
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

    // SNI filtering: check inbound TLS ClientHello for blocked domains
    let sni_enabled = unsafe { crate::SNI_ENABLED.get(0).copied().unwrap_or(0) };
    if sni_enabled == 1 && pkt.protocol == IPPROTO_TCP && pkt.dst_port == 443 {
        let ip_base = data + ETH_HDR_LEN;
        let ihl = unsafe { (*((ip_base) as *const u8) & 0x0F) as usize * 4 };
        let transport_base = ip_base + ihl;

        if let Some(result) = crate::tls::check_sni(data, data_end, &pkt, transport_base) {
            if result.action == 1 {
                crate::tls::emit_sni_event(ctx, &pkt, &result, data_end);
                if let Some(m) = unsafe { crate::METRICS.get_ptr_mut(0) } {
                    unsafe { (*m).packets_dropped += 1 };
                }
                return Ok(XDP_DROP);
            } else if result.action == 2 {
                crate::tls::emit_sni_event(ctx, &pkt, &result, data_end);
            }
        }
    }

    // Zone-based policy check
    let ifindex = unsafe { (*ctx.ctx).ingress_ifindex };
    if let Some(src_zone) = unsafe { crate::ZONE_MAP.get(&ifindex) } {
        let dst_zone: u32 = 0; // Local zone
        if *src_zone != dst_zone {
            let policy_key = (*src_zone << 16) | dst_zone;
            if let Some(policy) = unsafe { crate::POLICY_MAP.get(&policy_key) } {
                if policy.action == 1 {
                    // Zone policy says DROP
                    if let Some(m) = unsafe { crate::METRICS.get_ptr_mut(0) } {
                        unsafe { (*m).packets_dropped += 1 };
                    }
                    let event = EbpfPacketEvent {
                        timestamp: now, src_ip: pkt.src_ip, dst_ip: pkt.dst_ip,
                        src_port: pkt.src_port, dst_port: pkt.dst_port,
                        protocol: pkt.protocol, action: 1, rule_id: 0,
                        ifindex, bytes: pkt.pkt_len,
                    };
                    unsafe { crate::EVENTS.output(ctx, &event, 0); }
                    return Ok(XDP_DROP);
                }
            }
        }
    }

    // Build conntrack keys
    let fwd_key = ConntrackKey {
        src_ip: pkt.src_ip, dst_ip: pkt.dst_ip,
        src_port: pkt.src_port, dst_port: pkt.dst_port,
        protocol: pkt.protocol, _pad: [0; 3],
    };
    let rev_key = ConntrackKey {
        src_ip: pkt.dst_ip, dst_ip: pkt.src_ip,
        src_port: pkt.dst_port, dst_port: pkt.src_port,
        protocol: pkt.protocol, _pad: [0; 3],
    };

    // Check conntrack: if reply to established outgoing connection, pass
    if let Some(entry) = unsafe { crate::CONNTRACK.get(&rev_key) } {
        if entry.state == 1 || entry.state == 0 {
            if let Some(entry_mut) = unsafe { crate::CONNTRACK.get_ptr_mut(&rev_key) } {
                unsafe {
                    if pkt.tcp_flags & (TCP_FIN | TCP_RST) != 0 {
                        (*entry_mut).state = 4;
                    } else {
                        (*entry_mut).state = 1;
                    }
                    (*entry_mut).packets_in += 1;
                    (*entry_mut).bytes_in += pkt.pkt_len as u64;
                    (*entry_mut).last_seen = now;
                }
            }
            if let Some(m) = unsafe { crate::METRICS.get_ptr_mut(0) } {
                unsafe { (*m).packets_allowed += 1 };
            }
            return Ok(XDP_PASS);
        }
    }

    // Check existing forward conntrack entry
    let existing_state = unsafe {
        crate::CONNTRACK.get(&fwd_key).map(|e| e.state).unwrap_or(255)
    };

    // Evaluate ingress rules
    let rule_count = unsafe {
        crate::INGRESS_RULE_COUNT.get(0).copied().unwrap_or(0)
    };

    let mut matched_action: u8 = 0;
    let mut matched_rule_id: u32 = 0;
    let mut should_log = false;

    for i in 0..MAX_RULES {
        if i >= rule_count {
            break;
        }

        let rule: &EbpfRule = match unsafe { crate::INGRESS_RULES.get(i) } {
            Some(r) => r,
            None => break,
        };

        if rule.enabled == 0 { continue; }
        if rule.direction != 0 { continue; }
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
                        matched_action = 0; // Allow
                    } else {
                        matched_action = 1; // Drop (over rate)
                        should_log = true;
                    }
                },
                None => {
                    let state = nylon_wall_common::log::EbpfRateState {
                        tokens: rate, last_update: now,
                    };
                    let _ = crate::RATE_LIMIT.insert(&rule.id, &state, 0);
                    matched_action = 0; // Allow (initial)
                }
            }
        }

        // Increment hit counter
        if let Some(count) = unsafe { crate::RULE_HITS.get_ptr_mut(&rule.id) } {
            unsafe { *count += 1 };
        } else {
            let one: u64 = 1;
            let _ = crate::RULE_HITS.insert(&rule.id, &one, 0);
        }

        break;
    }

    let action = if matched_action == 1 { XDP_DROP } else { XDP_PASS };

    // Update conntrack for allowed traffic
    if action == XDP_PASS {
        if let Some(m) = unsafe { crate::METRICS.get_ptr_mut(0) } {
            unsafe { (*m).packets_allowed += 1 };
        }
        if existing_state == 255 {
            let entry = ConntrackEntry {
                state: 0, _pad: [0; 3],
                packets_in: 1, packets_out: 0,
                bytes_in: pkt.pkt_len as u64, bytes_out: 0,
                last_seen: now, timeout: 300, _pad2: 0,
            };
            let _ = crate::CONNTRACK.insert(&fwd_key, &entry, 0);
        } else if let Some(entry_mut) = unsafe { crate::CONNTRACK.get_ptr_mut(&fwd_key) } {
            unsafe {
                if pkt.tcp_flags & (TCP_FIN | TCP_RST) != 0 { (*entry_mut).state = 4; }
                (*entry_mut).packets_in += 1;
                (*entry_mut).bytes_in += pkt.pkt_len as u64;
                (*entry_mut).last_seen = now;
            }
        }
    } else {
        if let Some(m) = unsafe { crate::METRICS.get_ptr_mut(0) } {
            unsafe { (*m).packets_dropped += 1 };
        }
    }

    // Emit perf event for logging
    if should_log || matched_action == 1 {
        let event = EbpfPacketEvent {
            timestamp: now, src_ip: pkt.src_ip, dst_ip: pkt.dst_ip,
            src_port: pkt.src_port, dst_port: pkt.dst_port,
            protocol: pkt.protocol, action: matched_action,
            rule_id: matched_rule_id, ifindex, bytes: pkt.pkt_len,
        };
        unsafe { crate::EVENTS.output(ctx, &event, 0); }
    }

    Ok(action)
}
