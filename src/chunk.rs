use heapless::Vec;

/// 8bit uncompressed chunk size
pub const UNCOMPRESSED_SIZE: usize = 96;
/// 6bit compacted chunk size
pub const COMPACTED_SIZE: usize = 72;
/// Theoretical Minimum Compressed Data: ANY VALUE + (SLIDE * 2) = 5
pub const MINIMAL_COMPRESSED_SIZE: usize = 5;

pub(crate) fn compress(src: &[u8; UNCOMPRESSED_SIZE], output: &mut Vec<u8, 128>) {
    output.clear();

    // Attempt LZ compression
    let mut cursor = 0;
    while let Some(current) = src.get(cursor) {
        cursor += {
            let min_len = 3;
            let max_len = 64;
            let max_slide = 64;
            let mut matches = (0, 0);
            for slide in 1..=cursor.min(max_slide) {
                let len = check_len(src, cursor, cursor - slide).min(max_len);
                if matches.0 < len {
                    matches = (len, slide);
                }
            }
            if matches.0 >= min_len {
                let len = matches.0;
                let slide = matches.1;
                output.push(0x40 | (len - min_len) as u8).unwrap();
                output.push(slide as u8 - 1).unwrap();
                len
            } else {
                output.push(*current).unwrap();
                1
            }
        }
    }

    // If compression does not reduce size much, switch to compaction
    if output.len() < COMPACTED_SIZE {
        return;
    }
    output.clear();

    // 6bit compaction
    for chunk in src.chunks(4) {
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
                    // raw value
                    output.push(data & 0x3F).ok()?;
                }
                0b0100_0000..=0b0111_1111 => {
                    // slide
                    let slen = (data & 0x3F) as usize + 3;
                    let slide = *src.get(cursor + 1)?;
                    let slide = (slide & 0x7F) as usize + 1;
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
                    // reserved
                    return None;
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

#[inline]
fn check_len(src: &[u8], lhs: usize, rhs: usize) -> usize {
    let mut len = 0;
    loop {
        let lhs = match src.get(lhs + len) {
            Some(v) => *v,
            None => return len,
        };
        let rhs = match src.get(rhs + len) {
            Some(v) => *v,
            None => return len,
        };
        if lhs != rhs {
            return len;
        }
        len += 1;
    }
}
