use crate::{chunk::UNCOMPRESSED_SIZE, color::*, *};
use heapless::Vec;

pub struct Encoder;

impl Encoder {
    #[cfg(feature = "alloc")]
    pub fn encode(
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<alloc::vec::Vec<u8>, EncodeError> {
        let mut vec = alloc::vec::Vec::new();
        Self::encode_to_writer(data, width, height, |v| vec.extend_from_slice(v)).map(|_| vec)
    }

    pub fn encode_to_writer<F>(
        data: &[u8],
        width: u32,
        height: u32,
        mut writer: F,
    ) -> Result<(), EncodeError>
    where
        F: FnMut(&[u8]),
    {
        if data.len() < (width as usize * height as usize * 3) {
            return Err(EncodeError::InvalidInput);
        }
        let header = FileHeader::new(width, height).ok_or(EncodeError::InvalidInput)?;
        writer(header.bytes());

        let w8 = width & !7;
        let h8 = height & !7;
        let w7 = width & 7;
        let h7 = height & 7;

        for y8 in (0..h8).step_by(8) {
            for x8 in (0..w8).step_by(8) {
                let mut buf_y = [0u8; 64];
                let mut buf_u = [0u8; 64];
                let mut buf_v = [0u8; 64];
                for index in 0..64 {
                    let x7 = index & 7;
                    let y7 = index >> 3;
                    let offset = ((y8 as usize + y7) * width as usize + x8 as usize + x7) * 3;

                    let r = data[offset + 0];
                    let g = data[offset + 1];
                    let b = data[offset + 2];
                    let yuv = MpicYuv666::from_rgb(MpicRgb888::new(r, g, b));
                    buf_y[index] = yuv.y;
                    buf_u[index] = yuv.u;
                    buf_v[index] = yuv.v;
                }
                let block = Self::encode_chunk(&buf_y, &buf_u, &buf_v);
                writer(&[block.len() as u8]);
                writer(block.as_slice());
            }
            if w7 > 0 {
                let block = Self::_encode_edge(data, width, w8, y8, w7, 8);
                writer(&[block.len() as u8]);
                writer(block.as_slice());
            }
        }
        if h7 > 0 {
            for x8 in (0..w8).step_by(8) {
                let block = Self::_encode_edge(data, width, x8, h8, 8, h7);
                writer(&[block.len() as u8]);
                writer(block.as_slice());
            }
            if w7 > 0 {
                let block = Self::_encode_edge(data, width, w8, h8, w7, h7);
                writer(&[block.len() as u8]);
                writer(block.as_slice());
            }
        }

        Ok(())
    }

    #[inline]
    fn _encode_edge(data: &[u8], width: u32, x8: u32, y8: u32, w7: u32, h7: u32) -> Vec<u8, 128> {
        let w7 = w7 as usize;
        let h7 = h7 as usize;
        assert!(w7 > 0 && w7 <= 8);
        assert!(h7 > 0 && h7 <= 8);

        let w1 = w7 & 1;
        let h1 = h7 & 1;

        let mut buf_y = [0; 64];
        let mut buf_u = [0; 64];
        let mut buf_v = [0; 64];
        for y7 in 0..h7 {
            for x7 in 0..w7 {
                let index = y7 * 8 + x7;
                let offset = ((y8 as usize + y7) * width as usize + x8 as usize + x7) * 3;
                let r = data[offset + 0];
                let g = data[offset + 1];
                let b = data[offset + 2];
                let yuv = MpicYuv666::from_rgb(MpicRgb888::new(r, g, b));
                buf_y[index] = yuv.y;
                buf_u[index] = yuv.u;
                buf_v[index] = yuv.v;
            }
            if w1 > 0 {
                let index = y7 * 8 + w7;
                buf_y[index] = buf_y[index - 1];
                buf_u[index] = buf_u[index - 1];
                buf_v[index] = buf_v[index - 1];
            }
            for x7 in w7 + w1..8 {
                let index = y7 * 8 + x7;
                buf_y[index] = buf_y[0];
                buf_u[index] = buf_u[0];
                buf_v[index] = buf_v[0];
            }
        }
        if h1 > 0 {
            for x7 in 0..8 {
                let index_l = h7 * 8 + x7;
                let index_r = index_l - 8;
                buf_y[index_l] = buf_y[index_r];
                buf_u[index_l] = buf_u[index_r];
                buf_v[index_l] = buf_v[index_r];
            }
        }
        for y7 in h7 + h1..8 {
            for x7 in 0..8 {
                let index = y7 * 8 + x7;
                buf_y[index] = buf_y[0];
                buf_u[index] = buf_u[0];
                buf_v[index] = buf_v[0];
            }
        }

        Self::encode_chunk(&buf_y, &buf_u, &buf_v)
    }

    pub fn encode_chunk(buf_y: &[u8; 64], buf_u: &[u8; 64], buf_v: &[u8; 64]) -> Vec<u8, 128> {
        let mut buf = [0; UNCOMPRESSED_SIZE];
        for i in 0..64 {
            buf[i] = buf_y[i];
        }

        let (buf_u, buf_v) = mosaic_uv(buf_u, buf_v);

        for i in 0..16 {
            buf[64 + i] = buf_u[i];
        }
        for i in 0..16 {
            buf[80 + i] = buf_v[i];
        }

        let mut vec = Vec::<u8, 128>::new();
        chunk::compress(&buf, &mut vec);
        // vec.extend_from_slice(&buf).unwrap();

        #[cfg(test)]
        {
            let mut unpacked = Vec::<u8, UNCOMPRESSED_SIZE>::new();
            let result = chunk::decompress(vec.as_slice(), &mut unpacked);
            if result.is_none() || unpacked.as_slice() != buf.as_slice() {
                panic!(
                    "DECODE FAILED.\nEXPECTED:\n{:02x?}\nPACKED:\n{:02x?}\nUNPACKED:\n{:02x?}\n",
                    buf.as_slice(),
                    vec.as_slice(),
                    unpacked.as_slice(),
                );
            }
        }

        vec
    }
}

/// Mosaic the U and V channels.
pub(crate) fn mosaic_uv(buf_u: &[u8; 64], buf_v: &[u8; 64]) -> ([u8; 16], [u8; 16]) {
    let mut out_u = [0u8; 16];
    let mut out_v = [0u8; 16];

    for y in 0..4 {
        for x in 0..4 {
            let base = y * 16 + x * 2;

            let u0 = buf_u[base] as usize;
            let u1 = buf_u[base + 1] as usize;
            let u2 = buf_u[base + 8] as usize;
            let u3 = buf_u[base + 9] as usize;

            let v0 = buf_v[base] as usize;
            let v1 = buf_v[base + 1] as usize;
            let v2 = buf_v[base + 8] as usize;
            let v3 = buf_v[base + 9] as usize;

            let base = y * 4 + x;
            out_u[base] = ((u0 + u1 + u2 + u3) / 4) as u8;
            out_v[base] = ((v0 + v1 + v2 + v3) / 4) as u8;
        }
    }

    (out_u, out_v)
}
