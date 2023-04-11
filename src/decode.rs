use crate::{chunk::UNCOMPRESSED_SIZE, color::*, *};
use core::marker::PhantomData;
use heapless::Vec;

#[cfg(feature = "embedded")]
use embedded_graphics::{prelude::*, primitives::Rectangle};

pub struct Decoder<'a, T> {
    blob: &'a [u8],
    info: ImageInfo,
    _phantom: PhantomData<T>,
}

impl<'a, T> Decoder<'a, T> {
    #[inline]
    pub fn new(blob: &'a [u8]) -> Option<Self> {
        let header = FileHeader::from_bytes(blob)?;
        if !header.is_valid() {
            return None;
        }
        let info = header.info();
        Some(Self {
            blob,
            info,
            _phantom: PhantomData,
        })
    }

    #[inline]
    pub fn info(&self) -> ImageInfo {
        self.info
    }

    pub fn decode<F>(&self, mut draw_pixel: F) -> Result<(), DecodeError>
    where
        F: FnMut(u32, u32, Rgb),
    {
        let mut cursor = size_of::<FileHeader>();
        let width = self.info().width as u32;
        let height = self.info().height as u32;
        for y8 in (0..height).step_by(8) {
            for x8 in (0..width).step_by(8) {
                let len = *self.blob.get(cursor).ok_or(DecodeError::InvalidData)? as usize;
                let src = self
                    .blob
                    .get(cursor + 1..cursor + len + 1)
                    .ok_or(DecodeError::InvalidData)?;
                Self::decode_chunk(src).map(|(buf_y, buf_u, buf_v)| {
                    for y7 in 0..8 {
                        for x7 in 0..8 {
                            let y = buf_y[(y7 * 8 + x7) as usize];
                            let u = buf_u[(y7 * 8 + x7) as usize];
                            let v = buf_v[(y7 * 8 + x7) as usize];
                            let rgb = Rgb::from_yuv(Yuv666::new(y, u, v));
                            draw_pixel(x8 + x7 as u32, y8 + y7 as u32, rgb);
                        }
                    }
                })?;
                cursor += len + 1;
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn decode_block<F>(&self, mut draw_block: F) -> Result<(), DecodeError>
    where
        F: FnMut(u32, u32, &[u8; 64], &[u8; 64], &[u8; 64]),
    {
        let mut cursor = size_of::<FileHeader>();
        let width = self.info().width as u32;
        let height = self.info().height as u32;
        for y8 in (0..height).step_by(8) {
            for x8 in (0..width).step_by(8) {
                let len = *self.blob.get(cursor).ok_or(DecodeError::InvalidData)? as usize;
                let src = self
                    .blob
                    .get(cursor + 1..cursor + len + 1)
                    .ok_or(DecodeError::InvalidData)?;
                Self::decode_chunk(src).map(|(buf_y, buf_u, buf_v)| {
                    draw_block(x8, y8, &buf_y, &buf_u, &buf_v);
                })?;
                cursor += len + 1;
            }
        }
        Ok(())
    }

    pub fn decode_chunk(src: &[u8]) -> Result<([u8; 64], [u8; 64], [u8; 64]), DecodeError> {
        let mut vec = Vec::<u8, UNCOMPRESSED_SIZE>::new();
        chunk::decompress(src, &mut vec).ok_or(DecodeError::InvalidData)?;

        // let slice: [u8; UNCOMPRESSED_SIZE] =
        //     vec.into_array().map_err(|_| DecodeError::InvalidData)?;

        let buf_y: &[u8; 64] = &vec[0..64]
            .try_into()
            .map_err(|_| DecodeError::InvalidData)?;

        let buf_u = demosaic_uv(
            &vec[64..80]
                .try_into()
                .map_err(|_| DecodeError::InvalidData)?,
        );

        let buf_v = demosaic_uv(
            &vec[80..96]
                .try_into()
                .map_err(|_| DecodeError::InvalidData)?,
        );

        Ok((*buf_y, buf_u, buf_v))
    }
}

#[cfg(feature = "embedded")]
impl<T> OriginDimensions for Decoder<'_, T> {
    #[inline]
    fn size(&self) -> Size {
        self.info().into()
    }
}

#[cfg(feature = "embedded")]
impl<T: PixelColor + From<Rgb>> ImageDrawable for Decoder<'_, T> {
    type Color = T;

    #[inline]
    fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let mut result = Ok(());
        let _ = self.decode_block(|x8, y8, buf_y, buf_u, buf_v| {
            let mut colors = [T::from(Rgb::new(0, 0, 0)); 64];
            for index in 0..64 {
                let rgb = Rgb::from(Yuv666::new(buf_y[index], buf_u[index], buf_v[index]));
                colors[index] = rgb.into();
            }
            match target.fill_contiguous(
                &Rectangle::new(Point::new(x8 as i32, y8 as i32), Size::new(8, 8)),
                colors,
            ) {
                Ok(_) => (),
                Err(e) => result = Err(e),
            }
        });
        result
    }

    fn draw_sub_image<D>(&self, target: &mut D, area: &Rectangle) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        // TODO:
        self.draw(&mut target.translated(-area.top_left).clipped(area))
    }
}

/// Unmosaic the U and V channels
#[inline]
pub(crate) fn demosaic_uv(data: &[u8; 16]) -> [u8; 64] {
    let mut buf = [0u8; 64];
    for y in 0..4 {
        for x in 0..4 {
            let base = y * 16 + x * 2;
            let p = data[y * 4 + x];
            buf[base] = p;
            buf[base + 1] = p;
            buf[base + 8] = p;
            buf[base + 9] = p;
        }
    }
    buf
}
