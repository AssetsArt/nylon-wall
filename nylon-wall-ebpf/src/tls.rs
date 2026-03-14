use crate::common::*;

// TLS constants
const TLS_HANDSHAKE: u8 = 0x16;
const TLS_CLIENT_HELLO: u8 = 0x01;
const SNI_EXTENSION_TYPE: u16 = 0x0000;
const SNI_HOST_NAME_TYPE: u8 = 0x00;

// Bounded loop limits for the eBPF verifier
const MAX_EXTENSIONS: u32 = 16; // SNI is typically the first extension
const MAX_DOMAIN_SCAN: u32 = 64;

// Bitmasks for variable-length TLS fields.
// The eBPF verifier tracks `var_off` (tnum) for each register. Comparisons
// like `> MAX` narrow smin/smax but do NOT narrow var_off. Only bitwise AND
// narrows var_off. Without AND-masking, accumulated variable offsets from
// read_be16() create packet pointers with huge var_off, causing `r=0`
// (no validated range) on subsequent packet accesses.
const SESSION_ID_MASK: usize = 0xFF;       // max 255 bytes (spec: max 32, but be safe)
const CIPHER_SUITES_MASK: usize = 0x1FF;   // max 511 bytes (plenty for real-world)
const COMPRESS_MASK: usize = 0xFF;         // max 255 bytes
const EXT_LEN_MASK: usize = 0x7FF;         // max 2047 bytes per extension

/// Read a big-endian u16 from two consecutive bytes in packet memory.
/// Avoids the BPF `be16` instruction which destroys verifier range tracking.
#[inline(always)]
fn read_be16(ptr: usize) -> usize {
    let hi = unsafe { *(ptr as *const u8) } as usize;
    let lo = unsafe { *((ptr + 1) as *const u8) } as usize;
    (hi << 8) | lo
}

/// FNV-1a hash from packet memory with per-byte bounds checks.
/// Must match the hash used by daemon when populating the map.
#[inline(always)]
fn fnv1a_hash_pkt(data: usize, len: usize, data_end: usize) -> u32 {
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
        if byte >= b'A' && byte <= b'Z' {
            byte += 32;
        }
        hash ^= byte as u32;
        hash = hash.wrapping_mul(16777619);
    }
    hash
}

/// Check if a TCP packet contains a TLS ClientHello with a blocked SNI domain.
/// Returns true if the packet should be dropped.
///
/// This is the minimal eBPF-side implementation: parse TLS → hash domain →
/// single map lookup. No domain copying, no wildcard matching, no event
/// emission. The daemon handles wildcard rules by pre-populating parent
/// domain hashes in the SNI_POLICY map.
///
/// IMPORTANT (TC egress callers): The linear buffer of TC skbs may not
/// include the TCP payload. Call `ctx.pull_data(512)` and re-read
/// `ctx.data()`/`ctx.data_end()` before calling this function.
#[inline(always)]
pub fn check_sni_block(
    data: usize,
    data_end: usize,
    transport_base: usize,
) -> bool {
    // TCP header length (data offset field)
    if transport_base + 13 > data_end {
        return false;
    }
    // TCP data offset is top 4 bits × 4, max 60 bytes. AND-mask to 0x3F
    // to keep var_off bounded for the verifier.
    let tcp_data_offset = (unsafe {
        ((*((transport_base + 12) as *const u8)) >> 4) as usize * 4
    }) & 0x3F;
    if tcp_data_offset < TCP_HDR_LEN {
        return false;
    }

    let tls_base = transport_base + tcp_data_offset;

    // TLS record header (5 bytes) + handshake header (4 bytes)
    if tls_base + 9 > data_end {
        return false;
    }

    if unsafe { *(tls_base as *const u8) } != TLS_HANDSHAKE {
        return false;
    }

    let handshake_base = tls_base + 5;
    if unsafe { *(handshake_base as *const u8) } != TLS_CLIENT_HELLO {
        return false;
    }

    // ClientHello: skip type(1) + length(3) + version(2) + random(32) = 38
    let mut pos = handshake_base + 38;
    if pos + 1 > data_end {
        return false;
    }

    // Skip session_id (AND-mask to keep var_off small for verifier)
    let session_id_len = (unsafe { *(pos as *const u8) } as usize) & SESSION_ID_MASK;
    pos += 1 + session_id_len;
    if pos + 2 > data_end {
        return false;
    }

    // Skip cipher_suites (AND-mask narrows var_off from 0xffff to 0x1ff)
    let cipher_suites_len = read_be16(pos) & CIPHER_SUITES_MASK;
    pos += 2 + cipher_suites_len;
    if pos + 1 > data_end {
        return false;
    }

    // Skip compression_methods (AND-mask to keep var_off small)
    let compression_len = (unsafe { *(pos as *const u8) } as usize) & COMPRESS_MASK;
    pos += 1 + compression_len;
    if pos + 2 > data_end {
        return false;
    }

    // Extensions
    // NOTE: All bounds checks use `data_end` directly, NOT a derived variable
    // like `extensions_end`. The eBPF verifier only recognises comparisons
    // against the original data_end register (PKT_END) as valid packet
    // bounds checks. Comparisons against any other register — even one
    // logically equal to or smaller than data_end — produce r=0 (no
    // validated range), causing "invalid access to packet" rejections.
    let _extensions_len = read_be16(pos);
    pos += 2;

    for _ in 0..MAX_EXTENSIONS {
        if pos + 4 > data_end {
            break;
        }

        let ext_type = read_be16(pos) as u16;
        let ext_len = read_be16(pos + 2) & EXT_LEN_MASK;

        if ext_type == SNI_EXTENSION_TYPE {
            let sni_data = pos + 4;
            if sni_data + 5 > data_end {
                return false;
            }

            let name_type = unsafe { *((sni_data + 2) as *const u8) };
            if name_type != SNI_HOST_NAME_TYPE {
                return false;
            }

            // AND-mask name_len to 0x3F (max 63) — keeps var_off tiny
            let name_len = read_be16(sni_data + 3) & 0x3F;
            let name_start = sni_data + 5;

            if name_len == 0 {
                return false;
            }

            // Hash domain directly from packet memory and lookup
            let hash = fnv1a_hash_pkt(name_start, name_len, data_end);
            return unsafe { crate::SNI_POLICY.get(&hash) }
                .map(|a| *a == 1) // 1 = block
                .unwrap_or(false);
        }

        pos += 4 + ext_len;
    }

    false
}
