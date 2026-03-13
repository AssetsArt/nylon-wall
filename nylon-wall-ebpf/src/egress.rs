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
/// Uses the TC (Traffic Control) classifier hook.
/// Steps mirror ingress but check EGRESS_RULES and update conntrack for outbound.
pub fn process_egress(ctx: &TcContext) -> Result<i32, ()> {
    let data = ctx.data();
    let data_end = ctx.data_end();

    let pkt = match parse_packet(data, data_end) {
        Some(p) => p,
        None => return Ok(TC_ACT_OK),
    };

    let fwd_key = ConntrackKey {
        src_ip: pkt.src_ip,
        dst_ip: pkt.dst_ip,
        src_port: pkt.src_port,
        dst_port: pkt.dst_port,
        protocol: pkt.protocol,
        _pad: [0; 3],
    };

    let rev_key = ConntrackKey {
        src_ip: pkt.dst_ip,
        dst_ip: pkt.src_ip,
        src_port: pkt.dst_port,
        dst_port: pkt.src_port,
        protocol: pkt.protocol,
        _pad: [0; 3],
    };

    let now = unsafe { aya_ebpf::helpers::bpf_ktime_get_ns() };

    // Check conntrack: if reply to established inbound connection, pass
    if let Some(entry) = unsafe { crate::CONNTRACK.get(&rev_key) } {
        if entry.state == 1 || entry.state == 0 {
            if let Some(entry_mut) = unsafe { crate::CONNTRACK.get_ptr_mut(&rev_key) } {
                unsafe {
                    if pkt.tcp_flags & (TCP_FIN | TCP_RST) != 0 {
                        (*entry_mut).state = 4; // Closing
                    } else {
                        (*entry_mut).state = 1;
                    }
                    (*entry_mut).packets_out += 1;
                    (*entry_mut).bytes_out += pkt.pkt_len as u64;
                    (*entry_mut).last_seen = now;
                }
            }
            return Ok(TC_ACT_OK);
        }
    }

    let existing_state = unsafe {
        crate::CONNTRACK
            .get(&fwd_key)
            .map(|e| e.state)
            .unwrap_or(255)
    };

    // Evaluate egress rules
    let rule_count = unsafe {
        crate::EGRESS_RULE_COUNT
            .get(0)
            .copied()
            .unwrap_or(0)
    };

    let mut matched_action: u8 = 0; // Default: Allow
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

        if rule.enabled == 0 {
            continue;
        }

        // Direction check: 1 = Egress
        if rule.direction != 1 {
            continue;
        }

        if rule.protocol != 0 && rule.protocol != pkt.protocol {
            continue;
        }

        if !ip_match(pkt.src_ip, rule.src_ip, rule.src_mask) {
            continue;
        }
        if !ip_match(pkt.dst_ip, rule.dst_ip, rule.dst_mask) {
            continue;
        }

        if !port_match(pkt.src_port, rule.src_port_start, rule.src_port_end) {
            continue;
        }
        if !port_match(pkt.dst_port, rule.dst_port_start, rule.dst_port_end) {
            continue;
        }

        matched_action = rule.action;
        matched_rule_id = rule.id;
        should_log = rule.action == 1 || rule.action == 2;

        if let Some(count) = unsafe { crate::RULE_HITS.get_ptr_mut(&rule.id) } {
            unsafe { *count += 1 };
        } else {
            let one: u64 = 1;
            let _ = crate::RULE_HITS.insert(&rule.id, &one, 0);
        }

        break;
    }

    let action = match matched_action {
        1 => TC_ACT_SHOT,
        _ => TC_ACT_OK,
    };

    // Update conntrack for allowed outbound traffic
    if action == TC_ACT_OK {
        if existing_state == 255 {
            let entry = ConntrackEntry {
                state: 0,
                _pad: [0; 3],
                packets_in: 0,
                packets_out: 1,
                bytes_in: 0,
                bytes_out: pkt.pkt_len as u64,
                last_seen: now,
                timeout: 300,
                _pad2: 0,
            };
            let _ = crate::CONNTRACK.insert(&fwd_key, &entry, 0);
        } else if let Some(entry_mut) = unsafe { crate::CONNTRACK.get_ptr_mut(&fwd_key) } {
            unsafe {
                if pkt.tcp_flags & (TCP_FIN | TCP_RST) != 0 {
                    (*entry_mut).state = 4; // Closing
                }
                (*entry_mut).packets_out += 1;
                (*entry_mut).bytes_out += pkt.pkt_len as u64;
                (*entry_mut).last_seen = now;
            }
        }
    }

    // Emit perf event
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
            ifindex: 0, // TC doesn't have direct ifindex access in the same way
            bytes: pkt.pkt_len,
        };
        let _ = unsafe { crate::EVENTS.output(ctx, &event, 0) };
    }

    Ok(action)
}
