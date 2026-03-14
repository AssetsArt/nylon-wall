use nylon_wall_common::tls::{EbpfSniEvent, SNI_MAX_DOMAIN_LEN};

use crate::common::*;

// TLS constants
const TLS_HANDSHAKE: u8 = 0x16;
const TLS_CLIENT_HELLO: u8 = 0x01;
const SNI_EXTENSION_TYPE: u16 = 0x0000;
const SNI_HOST_NAME_TYPE: u8 = 0x00;

// Max iterations for eBPF verifier bounded loops
const MAX_EXTENSIONS: u32 = 64;
const MAX_DOMAIN_SCAN: u32 = SNI_MAX_DOMAIN_LEN as u32;

/// FNV-1a hash for domain name lookup.
/// Must match the hash used by daemon when populating the map.
#[inline(always)]
fn fnv1a_hash_bytes(data: usize, len: usize, data_end: usize) -> u32 {
    let mut hash: u32 = 2166136261;
    for i in 0..MAX_DOMAIN_SCAN {
        if i as usize >= len {
            break;
        }
        let ptr = data + i as usize;
        if ptr + 1 > data_end {
            break;
        }
        let mut byte = unsafe { *(ptr as *const u8) };
        // Lowercase for case-insensitive matching
        if byte >= b'A' && byte <= b'Z' {
            byte += 32;
        }
        hash ^= byte as u32;
        hash = hash.wrapping_mul(16777619);
    }
    hash
}

/// Check if a TCP packet to port 443 contains a TLS ClientHello with SNI,
/// and apply SNI policy. Returns Some(action) if SNI was matched, None otherwise.
///
/// `transport_base` is the start of the TCP header.
/// `pkt` contains parsed IP/port info.
#[inline(always)]
pub fn check_sni(
    data: usize,
    data_end: usize,
    pkt: &PacketInfo,
    transport_base: usize,
) -> Option<SniResult> {
    // Only check TCP to port 443
    if pkt.protocol != IPPROTO_TCP || pkt.dst_port != 443 {
        return None;
    }

    // TCP header length (data offset field: upper 4 bits of byte 12, in 32-bit words)
    if transport_base + 13 > data_end {
        return None;
    }
    let tcp_data_offset = unsafe {
        ((*((transport_base + 12) as *const u8)) >> 4) as usize * 4
    };
    if tcp_data_offset < TCP_HDR_LEN {
        return None;
    }

    let tls_base = transport_base + tcp_data_offset;

    // Need at least TLS record header (5 bytes) + handshake header (4 bytes)
    if tls_base + 9 > data_end {
        return None;
    }

    // Check TLS record type = Handshake (0x16)
    let content_type = unsafe { *(tls_base as *const u8) };
    if content_type != TLS_HANDSHAKE {
        return None;
    }

    // TLS record length
    let tls_record_len = unsafe {
        u16::from_be_bytes(*((tls_base + 3) as *const [u8; 2])) as usize
    };

    let handshake_base = tls_base + 5;

    // Check handshake type = ClientHello (0x01)
    let handshake_type = unsafe { *(handshake_base as *const u8) };
    if handshake_type != TLS_CLIENT_HELLO {
        return None;
    }

    // ClientHello starts at handshake_base + 4 (skip type + 3-byte length)
    let ch_base = handshake_base + 4;

    // Skip: client_version (2) + random (32) = 34 bytes
    let mut pos = ch_base + 34;
    if pos + 1 > data_end {
        return None;
    }

    // Skip session_id (variable: 1-byte length + data)
    let session_id_len = unsafe { *(pos as *const u8) } as usize;
    pos += 1 + session_id_len;
    if pos + 2 > data_end {
        return None;
    }

    // Skip cipher_suites (variable: 2-byte length + data)
    let cipher_suites_len = unsafe {
        u16::from_be_bytes(*((pos) as *const [u8; 2])) as usize
    };
    pos += 2 + cipher_suites_len;
    if pos + 1 > data_end {
        return None;
    }

    // Skip compression_methods (variable: 1-byte length + data)
    let compression_len = unsafe { *(pos as *const u8) } as usize;
    pos += 1 + compression_len;
    if pos + 2 > data_end {
        return None;
    }

    // Extensions length
    let extensions_len = unsafe {
        u16::from_be_bytes(*((pos) as *const [u8; 2])) as usize
    };
    pos += 2;

    let extensions_end = pos + extensions_len;
    // Bound to data_end and TLS record boundary
    let bound = if extensions_end < data_end { extensions_end } else { data_end };

    // Walk extensions to find SNI
    for _ in 0..MAX_EXTENSIONS {
        if pos + 4 > bound {
            break;
        }

        let ext_type = unsafe {
            u16::from_be_bytes(*((pos) as *const [u8; 2]))
        };
        let ext_len = unsafe {
            u16::from_be_bytes(*((pos + 2) as *const [u8; 2])) as usize
        };

        if ext_type == SNI_EXTENSION_TYPE {
            // Found SNI extension
            // SNI list: 2-byte list_length, then entries
            let sni_data = pos + 4;
            if sni_data + 5 > bound {
                return None;
            }

            // Skip list length (2 bytes), read name_type (1 byte)
            let name_type = unsafe { *((sni_data + 2) as *const u8) };
            if name_type != SNI_HOST_NAME_TYPE {
                return None;
            }

            // Name length (2 bytes)
            let name_len = unsafe {
                u16::from_be_bytes(*((sni_data + 3) as *const [u8; 2])) as usize
            };
            let name_start = sni_data + 5;

            if name_start + name_len > data_end || name_len == 0 {
                return None;
            }

            // Hash the domain name
            let domain_hash = fnv1a_hash_bytes(name_start, name_len, data_end);

            // Check SNI policy map
            let action = match unsafe { crate::SNI_POLICY.get(&domain_hash) } {
                Some(a) => *a,
                None => {
                    // Check wildcard: hash parent domains
                    // e.g. "www.facebook.com" → also check "facebook.com" and "com"
                    match check_wildcard(name_start, name_len, data_end) {
                        Some(a) => a,
                        None => return Some(SniResult {
                            action: 0, // default allow
                            domain_hash,
                            domain_start: name_start,
                            domain_len: name_len,
                        }),
                    }
                }
            };

            return Some(SniResult {
                action,
                domain_hash,
                domain_start: name_start,
                domain_len: name_len,
            });
        }

        pos += 4 + ext_len;
    }

    None // No SNI extension found
}

/// Check wildcard domain matches by hashing parent domains.
/// e.g. for "www.facebook.com", also check "facebook.com" and "com"
#[inline(always)]
fn check_wildcard(name_start: usize, name_len: usize, data_end: usize) -> Option<u8> {
    let mut offset: usize = 0;

    for _ in 0..MAX_DOMAIN_SCAN {
        if offset >= name_len {
            break;
        }
        let ptr = name_start + offset;
        if ptr + 1 > data_end {
            break;
        }
        let byte = unsafe { *(ptr as *const u8) };
        if byte == b'.' {
            // Hash the remaining part after this dot
            let parent_start = name_start + offset + 1;
            let parent_len = name_len - offset - 1;
            if parent_len > 0 {
                let parent_hash = fnv1a_hash_bytes(parent_start, parent_len, data_end);
                if let Some(a) = unsafe { crate::SNI_POLICY.get(&parent_hash) } {
                    return Some(*a);
                }
            }
        }
        offset += 1;
    }

    None
}

/// Result of SNI inspection
pub struct SniResult {
    pub action: u8, // 0=allow, 1=block, 2=log
    pub domain_hash: u32,
    pub domain_start: usize,
    pub domain_len: usize,
}

/// Build and emit an SNI perf event
#[inline(always)]
pub fn emit_sni_event(
    ctx: &impl SniEventContext,
    pkt: &PacketInfo,
    result: &SniResult,
    data_end: usize,
) {
    let now = unsafe { aya_ebpf::helpers::bpf_ktime_get_ns() };

    let mut event = EbpfSniEvent {
        timestamp: now,
        src_ip: pkt.src_ip,
        dst_ip: pkt.dst_ip,
        src_port: pkt.src_port,
        dst_port: pkt.dst_port,
        domain_hash: result.domain_hash,
        action: result.action,
        domain_len: 0,
        _pad: [0; 2],
        domain: [0u8; SNI_MAX_DOMAIN_LEN],
    };

    // Copy domain name (up to SNI_MAX_DOMAIN_LEN bytes)
    let copy_len = if result.domain_len < SNI_MAX_DOMAIN_LEN {
        result.domain_len
    } else {
        SNI_MAX_DOMAIN_LEN
    };
    event.domain_len = copy_len as u8;

    for i in 0..MAX_DOMAIN_SCAN {
        if i as usize >= copy_len {
            break;
        }
        let ptr = result.domain_start + i as usize;
        if ptr + 1 > data_end {
            break;
        }
        event.domain[i as usize] = unsafe { *(ptr as *const u8) };
    }

    ctx.output_sni_event(&event);
}

/// Trait to abstract over XdpContext and TcContext for perf event output
pub trait SniEventContext {
    fn output_sni_event(&self, event: &EbpfSniEvent);
}

impl SniEventContext for aya_ebpf::programs::XdpContext {
    fn output_sni_event(&self, event: &EbpfSniEvent) {
        unsafe { crate::SNI_EVENTS.output(self, event, 0); }
    }
}

impl SniEventContext for aya_ebpf::programs::TcContext {
    fn output_sni_event(&self, event: &EbpfSniEvent) {
        let _ = unsafe { crate::SNI_EVENTS.output(self, event, 0) };
    }
}
