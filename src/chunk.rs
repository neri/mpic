use crate::lz::lazy::*;
use crate::lz::*;
use crate::*;
use core::convert::Infallible;

/// 8bit uncompressed chunk size
pub const UNCOMPRESSED_SIZE: usize = 96;
/// 6bit compacted chunk size
pub const COMPACTED_SIZE: usize = 72;
/// Theoretical Minimum Compressed Data: ANY VALUE + (SLIDE * 2) = 5
pub const MINIMAL_COMPRESSED_SIZE: usize = 5;

const MIN_LEN_SHORT: usize = 2;
const MAX_LEN_SHORT: usize = 3 + MIN_LEN_SHORT;
const MAX_DIST_SHORT: u8 = 32;
const MIN_LEN_LONG: usize = 3;
const MAX_LEN_LONG: usize = 63 + MIN_LEN_LONG;
const MAX_DIST: usize = 64;

/// Compress a chunk of data.
pub(crate) fn compress(
    src: &[u8; UNCOMPRESSED_SIZE],
    output: &mut Vec<u8, 128>,
    level: CompressionLevel,
) {
    // if true {
    //     output.extend_from_slice(src).unwrap();
    //     return;
    // }

    if cfg!(feature = "alloc") && level != CompressionLevel::Fast {
        compress_lazy(src, output, level);
    } else {
        compress_fast(src, output);
    }

    // If compression does not reduce size much, switch to compaction
    if output.len() < COMPACTED_SIZE {
        return;
    }

    compact(src, output);
}

/// 6bit compaction
///
/// `(00aa_aaaa 00bb_bbbb 00cc_cccc 00dd_dddd) -> (bbaa_aaaa cccc_bbbb dddd_ddcc)`
#[inline]
pub(crate) fn compact(src: &[u8; UNCOMPRESSED_SIZE], output: &mut Vec<u8, 128>) {
    output.clear();

    for chunk in src.chunks_exact(4) {
        let s1 = (chunk[0] & 0x3F) as u32;
        let s2 = (chunk[1] & 0x3F) as u32;
        let s3 = (chunk[2] & 0x3F) as u32;
        let s4 = (chunk[3] & 0x3F) as u32;

        let d0 = s1 | s2.wrapping_shl(6) | s3.wrapping_shl(12) | s4.wrapping_shl(18);

        output.push(d0 as u8).unwrap();
        output.push(d0.wrapping_shr(8) as u8).unwrap();
        output.push(d0.wrapping_shr(16) as u8).unwrap();
    }
}

/// Simple LZ compression
#[inline]
pub(crate) fn compress_fast(src: &[u8; UNCOMPRESSED_SIZE], output: &mut Vec<u8, 128>) {
    output.clear();

    let mut current = 0;
    while let Some(&literal) = src.get(current) {
        let count = {
            let mut best_match = BestMatch::Empty;

            // In mpic format, size of src is very small (96 bytes), so we can afford to check all possible matches.
            for distance in 1..=current.min(MAX_DIST) {
                let len = matching_len(src, current, current - distance, MAX_LEN_LONG);
                if len >= MIN_LEN_SHORT && best_match.len() < len {
                    best_match = BestMatch::new(len, distance);
                }
            }

            match best_match {
                BestMatch::Found(matches) => match emit_match(output, matches) {
                    Some(len) => len,
                    None => {
                        output.push(literal).unwrap();
                        1
                    }
                },
                BestMatch::Empty => {
                    output.push(literal).unwrap();
                    1
                }
            }
        };
        current += count;
    }
}

/// Compress using lazy matching
#[cfg(feature = "alloc")]
#[inline]
pub(crate) fn compress_lazy(
    src: &[u8; UNCOMPRESSED_SIZE],
    output: &mut Vec<u8, 128>,
    level: CompressionLevel,
) {
    output.clear();

    let mut lazy_match = LazyMatchSolver::new();
    for (current, &literal) in src.iter().enumerate() {
        let mut best_match = BestMatch::Empty;

        // In mpic format, size of src is very small (96 bytes), so we can afford to check all possible matches.
        for distance in 1..=current.min(MAX_DIST) {
            let len = matching_len(src, current, current - distance, MAX_LEN_LONG);
            if len >= MIN_LEN_SHORT && best_match.len() < len {
                best_match = BestMatch::new(len, distance);
            }
        }

        let mut weight = match best_match {
            BestMatch::Found(m) => weight(&m),
            BestMatch::Empty => 0,
        };
        if weight == 0 {
            best_match = BestMatch::Empty;
        } else {
            let position_weight = src.len() - current;
            weight = weight + position_weight;
        }

        lazy_match.push(LazyLzssItem::new(literal, best_match, weight));
    }

    let mut fast = Vec::<u8, 128>::new();
    lazy_match
        .solve(CompressionLevel::Fast, |item| {
            match item {
                LZSS::Literal(v) => {
                    fast.push(v).unwrap();
                }
                LZSS::Match(m) => {
                    emit_match(&mut fast, m).unwrap();
                }
            }
            Result::<(), Infallible>::Ok(())
        })
        .unwrap();

    if matches!(level, CompressionLevel::Fast) {
        output.extend_from_slice(&fast).unwrap();
        return;
    }

    let mut lazy = Vec::<u8, 128>::new();
    lazy_match
        .solve(CompressionLevel::Default, |item| {
            match item {
                LZSS::Literal(v) => {
                    lazy.push(v).unwrap();
                }
                LZSS::Match(m) => {
                    emit_match(&mut lazy, m).unwrap();
                }
            }
            Result::<(), Infallible>::Ok(())
        })
        .unwrap();

    if matches!(level, CompressionLevel::Default) {
        output.extend_from_slice(&best_size(&[fast, lazy])).unwrap();
        return;
    }

    let mut best = Vec::<u8, 128>::new();
    lazy_match
        .solve(CompressionLevel::Best, |item| {
            match item {
                LZSS::Literal(v) => {
                    best.push(v).unwrap();
                }
                LZSS::Match(m) => {
                    emit_match(&mut best, m).unwrap();
                }
            }
            Result::<(), Infallible>::Ok(())
        })
        .unwrap();

    output
        .extend_from_slice(&best_size(&[fast, lazy, best]))
        .unwrap();
}

/// Returns the best compressed data among candidates.
#[inline]
fn best_size(candidates: &[Vec<u8, 128>]) -> Vec<u8, 128> {
    candidates
        .iter()
        .min_by_key(|data| data.len())
        .cloned()
        .unwrap_or_default()
}

#[inline]
fn emit_match(output: &mut Vec<u8, 128>, matches: Match) -> Option<usize> {
    let len = matches.len.get();
    let encoded_slide = matches.distance.get() as u8;
    if len <= MAX_LEN_SHORT && encoded_slide <= MAX_DIST_SHORT {
        output
            .push(0x80 | (((len - MIN_LEN_SHORT) as u8) << 5) | (encoded_slide - 1))
            .unwrap();
        Some(len)
    } else if len >= MIN_LEN_LONG {
        output.push(0x40 | (len - MIN_LEN_LONG) as u8).unwrap();
        output.push(encoded_slide - 1).unwrap();
        Some(len)
    } else {
        None
    }
}

#[inline]
fn weight(matches: &Match) -> usize {
    let len = matches.len.get();
    let encoded_slide = matches.distance.get() as u8;
    if len >= MIN_LEN_SHORT && len <= MAX_LEN_SHORT && encoded_slide <= MAX_DIST_SHORT {
        len
    } else if len >= MIN_LEN_LONG {
        len - 1
    } else {
        0
    }
}

pub(crate) fn decompress(src: &[u8], output: &mut Vec<u8, UNCOMPRESSED_SIZE>) -> Option<()> {
    let len = src.len();
    output.clear();
    if len == UNCOMPRESSED_SIZE {
        // 8bit uncompressed
        output.extend_from_slice(src).ok()
    } else if len == COMPACTED_SIZE {
        // 6bit compacted
        let mut src = src.iter();
        for _ in 0..24 {
            let b1 = *src.next()? as u32;
            let b2 = *src.next()? as u32;
            let b3 = *src.next()? as u32;
            let d0 = b1 | b2.wrapping_shl(8) | b3.wrapping_shl(16);
            output.push((d0 & 0x3F) as u8).ok()?;
            output.push((d0.wrapping_shr(6) & 0x3F) as u8).ok()?;
            output.push((d0.wrapping_shr(12) & 0x3F) as u8).ok()?;
            output.push((d0.wrapping_shr(18) & 0x3F) as u8).ok()?;
        }
        Some(())
    } else if is_valid_compressed_size(len) {
        // compressed
        let mut cursor = 0;
        while cursor < len {
            let data = *src.get(cursor)?;
            match data {
                0b0000_0000..=0b0011_1111 => {
                    // 00vv_vvvv raw value
                    output.push(data & 0x3F).ok()?;
                }
                0b0100_0000..=0b0111_1111 => {
                    // 01nn_nnnn 00mm_mmmm slide long
                    let slen = (data & 0x3F) as usize + 3;
                    let slide = *src.get(cursor + 1)?;
                    if (slide & 0xC0) != 0 {
                        // RESERVED
                        return None;
                    }
                    let slide = slide as usize + 1;
                    if output.len() < slide || output.len() + slen > UNCOMPRESSED_SIZE {
                        return None;
                    }
                    let base = output.len() - slide;
                    for i in 0..slen {
                        let v = *output.get(base + i)?;
                        output.push(v).ok()?;
                    }
                    cursor += 1;
                }
                0b1000_0000..=0b1111_1111 => {
                    // 1nnm_mmmm slide short
                    let slen = 2 + ((data & 0x60) as usize >> 5);
                    let slide = (data & 0x1F) as usize + 1;
                    if output.len() < slide || output.len() + slen > UNCOMPRESSED_SIZE {
                        return None;
                    }
                    let base = output.len() - slide;
                    for i in 0..slen {
                        let v = *output.get(base + i)?;
                        output.push(v).ok()?;
                    }
                }
            }
            cursor += 1;
        }
        (output.len() == UNCOMPRESSED_SIZE).then(|| ())
    } else {
        // reserved
        None
    }
}

#[inline]
pub(crate) fn is_valid_compressed_size(size: usize) -> bool {
    size >= MINIMAL_COMPRESSED_SIZE && size < COMPACTED_SIZE
}
