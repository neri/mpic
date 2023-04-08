use crate::{chunk::UNCOMPRESSED_SIZE, color::*, *};
use heapless::Vec;

pub struct Encoder;

impl Encoder {
    pub fn encode<F>(data: &[u8], width: u32, height: u32, mut writer: F) -> Result<(), EncodeError>
    where
        F: FnMut(&[u8]),
    {
        let header = FileHeader::new(width, height);

        if !header.is_valid() || data.len() < (width as usize * height as usize * 3) {
            return Err(EncodeError::InvalidInput);
        }
        writer(header.bytes());

        for y8 in (0..height).step_by(8) {
            for x8 in (0..width).step_by(8) {
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
                    let yuv = Yuv666::from_rgb(Rgb { r, g, b });
                    buf_y[index] = yuv.y;
                    buf_u[index] = yuv.u;
                    buf_v[index] = yuv.v;
                }
                let block = Self::encode_chunk(&mut buf_y, &buf_u, &buf_v);
                writer(&[block.len() as u8]);
                writer(block.as_slice());
            }
        }
        Ok(())
    }

    pub fn encode_chunk(buf_y: &mut [u8; 64], buf_u: &[u8; 64], buf_v: &[u8; 64]) -> Vec<u8, 128> {
        dc2ac(buf_y);

        let mut buf_u = mosaic_uv(buf_u);
        dc2ac(&mut buf_u);

        let mut buf_v = mosaic_uv(buf_v);
        dc2ac(&mut buf_v);

        let mut buf = [0; UNCOMPRESSED_SIZE];
        for i in 0..64 {
            buf[i] = buf_y[i];
        }
        for i in 0..16 {
            buf[64 + i] = buf_u[i];
        }
        for i in 0..16 {
            buf[80 + i] = buf_v[i];
        }

        let mut vec = Vec::<u8, 128>::new();
        chunk::compress(&buf, &mut vec);

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
pub(crate) fn mosaic_uv(data: &[u8; 64]) -> [u8; 16] {
    let mut buf = [0u8; 16];
    for y in 0..4 {
        for x in 0..4 {
            let base = y * 16 + x * 2;
            if false {
                buf[y * 4 + x] = data[base];
            } else {
                let p0 = data[base] as usize;
                let p1 = data[base + 1] as usize;
                let p2 = data[base + 8] as usize;
                let p3 = data[base + 9] as usize;
                buf[y * 4 + x] = ((p0 + p1 + p2 + p3) / 4) as u8;
            }
        }
    }
    buf
}

/// Encode DC array to AC array
fn dc2ac(data: &mut [u8]) {
    let mut acc = data[0];
    for p in data.iter_mut().skip(1) {
        let v = *p;
        *p = v.wrapping_sub(acc) & 0x3F;
        acc = v;
    }
}
