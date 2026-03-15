use aya_ebpf::programs::XdpContext;
use aya_ebpf::bindings::xdp_action;

use nylon_wall_common::conntrack::{ConntrackEntry, ConntrackKey};
use nylon_wall_common::log::EbpfPacketEvent;
use nylon_wall_common::rule::EbpfRule;

use crate::common::*;
use crate::scratchpad::read_scratch;

const XDP_PASS: u32 = xdp_action::XDP_PASS;
const XDP_DROP: u32 = xdp_action::XDP_DROP;

/// Ingress rules stage (XDP tail call target, terminal).
///
/// Handles: zone policies, conntrack, firewall rules, rate limiting,
/// conntrack updates, and perf event emission.
pub fn process(ctx: &XdpContext) -> Result<u32, ()> {
    let scratch = match read_scratch() {
        Some(s) => s,
        None => return Ok(XDP_PASS),
    };

    // If already decided (e.g. by SNI stage), apply that decision
    let decided = unsafe { (*scratch).decided };
    if decided != 0 {
        let action = unsafe { (*scratch).action };
        return Ok(if action == 1 { XDP_DROP } else { XDP_PASS });
    }

    // Read scratch fields
    let src_ip = unsafe { (*scratch).src_ip };
    let dst_ip = unsafe { (*scratch).dst_ip };
    let src_port = unsafe { (*scratch).src_port };
    let dst_port = unsafe { (*scratch).dst_port };
    let protocol = unsafe { (*scratch).protocol };
    let tcp_flags = unsafe { (*scratch).tcp_flags };
    let pkt_len = unsafe { (*scratch).pkt_len };
    let ifindex = unsafe { (*scratch).ifindex };
    let now = unsafe { (*scratch).timestamp };

    // Zone-based policy check
    if let Some(src_zone) = unsafe { crate::ZONE_MAP.get(&ifindex) } {
        let dst_zone: u32 = 0;
        if *src_zone != dst_zone {
            let policy_key = (*src_zone << 16) | dst_zone;
            if let Some(policy) = unsafe { crate::POLICY_MAP.get(&policy_key) } {
                if policy.action == 1 {
                    if let Some(m) = unsafe { crate::METRICS.get_ptr_mut(0) } {
                        unsafe { (*m).packets_dropped += 1 };
                    }
                    let event = EbpfPacketEvent {
                        timestamp: now, src_ip, dst_ip,
                        src_port, dst_port,
                        protocol, action: 1, rule_id: 0,
                        ifindex, bytes: pkt_len,
                    };
                    unsafe { crate::EVENTS.output(ctx, &event, 0); }
                    return Ok(XDP_DROP);
                }
            }
        }
    }

    // Build conntrack keys
    let fwd_key = ConntrackKey {
        src_ip, dst_ip, src_port, dst_port,
        protocol, _pad: [0; 3],
    };
    let rev_key = ConntrackKey {
        src_ip: dst_ip, dst_ip: src_ip,
        src_port: dst_port, dst_port: src_port,
        protocol, _pad: [0; 3],
    };

    // Check conntrack: reply to established outgoing connection → pass
    if let Some(entry) = unsafe { crate::CONNTRACK.get(&rev_key) } {
        if entry.state == 1 || entry.state == 0 {
            if let Some(entry_mut) = unsafe { crate::CONNTRACK.get_ptr_mut(&rev_key) } {
                unsafe {
                    if tcp_flags & (TCP_FIN | TCP_RST) != 0 {
                        (*entry_mut).state = 4;
                    } else {
                        (*entry_mut).state = 1;
                    }
                    (*entry_mut).packets_in += 1;
                    (*entry_mut).bytes_in += pkt_len as u64;
                    (*entry_mut).last_seen = now;
                }
            }
            if let Some(m) = unsafe { crate::METRICS.get_ptr_mut(0) } {
                unsafe { (*m).packets_allowed += 1 };
            }
            return Ok(XDP_PASS);
        }
    }

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
        if i >= rule_count { break; }

        let rule: &EbpfRule = match unsafe { crate::INGRESS_RULES.get(i) } {
            Some(r) => r,
            None => break,
        };

        if rule.enabled == 0 { continue; }
        if rule.direction != 0 { continue; }
        if rule.protocol != 0 && rule.protocol != protocol { continue; }
        if !ip_match(src_ip, rule.src_ip, rule.src_mask) { continue; }
        if !ip_match(dst_ip, rule.dst_ip, rule.dst_mask) { continue; }
        if !port_match(src_port, rule.src_port_start, rule.src_port_end) { continue; }
        if !port_match(dst_port, rule.dst_port_start, rule.dst_port_end) { continue; }

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
                bytes_in: pkt_len as u64, bytes_out: 0,
                last_seen: now, timeout: 300, _pad2: 0,
            };
            let _ = crate::CONNTRACK.insert(&fwd_key, &entry, 0);
        } else if let Some(entry_mut) = unsafe { crate::CONNTRACK.get_ptr_mut(&fwd_key) } {
            unsafe {
                if tcp_flags & (TCP_FIN | TCP_RST) != 0 { (*entry_mut).state = 4; }
                (*entry_mut).packets_in += 1;
                (*entry_mut).bytes_in += pkt_len as u64;
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
            timestamp: now, src_ip, dst_ip,
            src_port, dst_port,
            protocol, action: matched_action,
            rule_id: matched_rule_id, ifindex, bytes: pkt_len,
        };
        unsafe { crate::EVENTS.output(ctx, &event, 0); }
    }

    Ok(action)
}
