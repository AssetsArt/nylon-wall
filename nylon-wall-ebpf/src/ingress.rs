use aya_ebpf::{bindings::xdp_action, helpers::bpf_ktime_get_ns, programs::XdpContext};

use nylon_wall_common::conntrack::{ConntrackEntry, ConntrackKey};
use nylon_wall_common::log::EbpfPacketEvent;
use nylon_wall_common::rule::EbpfRule;

use crate::common::*;

// TC action constants used in conntrack state for return traffic
const XDP_PASS: u32 = xdp_action::XDP_PASS;
const XDP_DROP: u32 = xdp_action::XDP_DROP;

/// Process an ingress (incoming) packet through the firewall.
///
/// Steps:
/// 1. Parse Ethernet → IPv4 → TCP/UDP headers
/// 2. Check connection tracking (ESTABLISHED connections pass immediately)
/// 3. Evaluate ingress rules in priority order
/// 4. Update conntrack state
/// 5. Emit perf event for logging if needed
pub fn process_ingress(ctx: &XdpContext) -> Result<u32, ()> {
    let data = ctx.data();
    let data_end = ctx.data_end();

    // Parse packet headers
    let pkt = match parse_packet(data, data_end) {
        Some(p) => p,
        None => return Ok(XDP_PASS), // Non-IPv4 or malformed → pass
    };

    // Build conntrack key for the forward direction
    let fwd_key = ConntrackKey {
        src_ip: pkt.src_ip,
        dst_ip: pkt.dst_ip,
        src_port: pkt.src_port,
        dst_port: pkt.dst_port,
        protocol: pkt.protocol,
        _pad: [0; 3],
    };

    // Build reverse key to check if this is a reply to an outgoing connection
    let rev_key = ConntrackKey {
        src_ip: pkt.dst_ip,
        dst_ip: pkt.src_ip,
        src_port: pkt.dst_port,
        dst_port: pkt.src_port,
        protocol: pkt.protocol,
        _pad: [0; 3],
    };

    let now = unsafe { bpf_ktime_get_ns() };

    // Check conntrack: if this is a reply to an established outgoing connection, pass it
    if let Some(entry) = unsafe { crate::CONNTRACK.get(&rev_key) } {
        if entry.state == 1 || entry.state == 0 {
            // ESTABLISHED or NEW (reply makes it established)
            // Update the reverse entry's counters
            if let Some(entry_mut) = unsafe { crate::CONNTRACK.get_ptr_mut(&rev_key) } {
                unsafe {
                    // Mark closing if FIN or RST seen
                    if pkt.tcp_flags & (TCP_FIN | TCP_RST) != 0 {
                        (*entry_mut).state = 4; // Closing
                    } else {
                        (*entry_mut).state = 1; // Established
                    }
                    (*entry_mut).packets_in += 1;
                    (*entry_mut).bytes_in += pkt.pkt_len as u64;
                    (*entry_mut).last_seen = now;
                }
            }
            return Ok(XDP_PASS);
        }
    }

    // Check if there's an existing forward conntrack entry
    let existing_state = unsafe {
        crate::CONNTRACK
            .get(&fwd_key)
            .map(|e| e.state)
            .unwrap_or(255) // 255 = no entry
    };

    // Evaluate ingress rules
    let rule_count = unsafe {
        crate::INGRESS_RULE_COUNT
            .get(0)
            .copied()
            .unwrap_or(0)
    };

    let mut matched_action: u8 = 0; // Default: Allow
    let mut matched_rule_id: u32 = 0;
    let mut should_log = false;

    // Bounded loop required by eBPF verifier
    for i in 0..MAX_RULES {
        if i >= rule_count {
            break;
        }

        let rule: &EbpfRule = match unsafe { crate::INGRESS_RULES.get(i) } {
            Some(r) => r,
            None => break,
        };

        // Skip disabled rules
        if rule.enabled == 0 {
            continue;
        }

        // Direction check: 0 = Ingress
        if rule.direction != 0 {
            continue;
        }

        // Protocol match (0 = any)
        if rule.protocol != 0 && rule.protocol != pkt.protocol {
            continue;
        }

        // IP matching
        if !ip_match(pkt.src_ip, rule.src_ip, rule.src_mask) {
            continue;
        }
        if !ip_match(pkt.dst_ip, rule.dst_ip, rule.dst_mask) {
            continue;
        }

        // Port matching
        if !port_match(pkt.src_port, rule.src_port_start, rule.src_port_end) {
            continue;
        }
        if !port_match(pkt.dst_port, rule.dst_port_start, rule.dst_port_end) {
            continue;
        }

        // Rule matches!
        matched_action = rule.action;
        matched_rule_id = rule.id;
        should_log = rule.action == 1 || rule.action == 2; // DROP or LOG

        // Increment hit counter
        if let Some(count) = unsafe { crate::RULE_HITS.get_ptr_mut(&rule.id) } {
            unsafe { *count += 1 };
        } else {
            let one: u64 = 1;
            let _ = crate::RULE_HITS.insert(&rule.id, &one, 0);
        }

        break; // First match wins (rules are sorted by priority)
    }

    // Determine XDP action
    let action = match matched_action {
        1 => XDP_DROP,  // Drop
        _ => XDP_PASS,  // Allow (0), Log (2), RateLimit (3) all pass
    };

    // Update conntrack for allowed traffic
    if action == XDP_PASS {
        if existing_state == 255 {
            // New connection
            let entry = ConntrackEntry {
                state: 0, // New
                _pad: [0; 3],
                packets_in: 1,
                packets_out: 0,
                bytes_in: pkt.pkt_len as u64,
                bytes_out: 0,
                last_seen: now,
                timeout: 300, // 5 minutes default
                _pad2: 0,
            };
            let _ = crate::CONNTRACK.insert(&fwd_key, &entry, 0);
        } else if let Some(entry_mut) = unsafe { crate::CONNTRACK.get_ptr_mut(&fwd_key) } {
            unsafe {
                if pkt.tcp_flags & (TCP_FIN | TCP_RST) != 0 {
                    (*entry_mut).state = 4; // Closing
                }
                (*entry_mut).packets_in += 1;
                (*entry_mut).bytes_in += pkt.pkt_len as u64;
                (*entry_mut).last_seen = now;
            }
        }
    }

    // Emit perf event for logging
    if should_log || matched_action == 1 {
        let event = EbpfPacketEvent {
            timestamp: now,
            src_ip: pkt.src_ip,
            dst_ip: pkt.dst_ip,
            src_port: pkt.src_port,
            dst_port: pkt.dst_port,
            protocol: pkt.protocol,
            action: matched_action,
            rule_id: matched_rule_id,
            ifindex: unsafe { (*ctx.ctx).ingress_ifindex },
            bytes: pkt.pkt_len,
        };
        unsafe {
            crate::EVENTS.output(ctx, &event, 0);
        }
    }

    Ok(action)
}
