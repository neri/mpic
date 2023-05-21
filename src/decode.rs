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

    #[cfg(feature = "alloc")]
    pub fn decode(&self) -> Result<alloc::vec::Vec<u8>, DecodeError> {
        let width = self.info().width() as usize;
        let height = self.info().height() as usize;
        let vec_size = width * height * 3;
        let mut vec = alloc::vec::Vec::with_capacity(vec_size);
        vec.resize(vec_size, 0);
        self.decode_to_slice(vec.as_mut()).map(|_| vec)
    }

    pub fn decode_to_slice(&self, output: &mut [u8]) -> Result<(), DecodeError> {
        let width = self.info().width() as usize;
        let height = self.info().height() as usize;
        let vec_size = width * height * 3;
        if output.len() < vec_size {
            return Err(DecodeError::InvalidInput);
        }

        let mut cursor = size_of::<FileHeader>();
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
                            let rgb = MpicRgb666::from_yuv(MpicYuv666::new(y, u, v));

                            let index = (x8 + x7 + (y8 + y7) * width) * 3;
                            output[index] = rgb.r8();
                            output[index + 1] = rgb.g8();
                            output[index + 2] = rgb.b8();
                        }
                    }
                })?;
                cursor += len + 1;
            }
        }
        Ok(())
    }

    #[cfg(feature = "alloc")]
    pub fn decode_rgba(&self) -> Result<alloc::vec::Vec<u8>, DecodeError> {
        let width = self.info().width() as usize;
        let height = self.info().height() as usize;
        let vec_size = width * height * 4;
        let mut vec = alloc::vec::Vec::with_capacity(vec_size);
        vec.resize(vec_size, 0);

        let mut cursor = size_of::<FileHeader>();
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
                            let rgb = MpicRgb666::from_yuv(MpicYuv666::new(y, u, v));

                            let index = (x8 + x7 + (y8 + y7) * width) * 4;
                            vec[index] = rgb.r8();
                            vec[index + 1] = rgb.g8();
                            vec[index + 2] = rgb.b8();
                            vec[index + 3] = u8::MAX;
                        }
                    }
                })?;
                cursor += len + 1;
            }
        }
        Ok(vec)
    }

    #[allow(dead_code)]
    fn decode_sub_image<F, E>(
        &self,
        left: i32,
        top: i32,
        width: u32,
        height: u32,
        mut draw_block: F,
    ) -> Result<(), E>
    where
        F: FnMut(u32, u32, &[u8; 64], &[u8; 64], &[u8; 64]) -> Result<(), E>,
    {
        let mut cursor = size_of::<FileHeader>();
        let image_width = ceil_7(self.info().width as u32);
        let image_height = ceil_7(self.info().height as u32);

        let mut left = left;
        let mut top = top;
        let mut right = left + width as i32;
        let mut bottom = top + height as i32;
        if left < 0 {
            right += left;
            left = 0;
        }
        if top < 0 {
            bottom += top;
            top = 0;
        }
        let block_left = (left as u32 / 8) * 8;
        let block_top = (top as u32 / 8) * 8;
        let block_right = ceil_7(image_width.min(right as u32));
        let block_bottom = ceil_7(image_height.min(bottom as u32));

        let mut result = Ok(());
        for y8 in (0..block_bottom).step_by(8) {
            for x8 in (0..image_width).step_by(8) {
                let len = match self.blob.get(cursor) {
                    Some(v) => *v as usize,
                    None => return result,
                };
                if y8 >= block_top && x8 >= block_left && x8 <= block_right {
                    let src = match self.blob.get(cursor + 1..cursor + len + 1) {
                        Some(v) => v,
                        None => return result,
                    };
                    match Self::decode_chunk(src).map(|(buf_y, buf_u, buf_v)| {
                        match draw_block(x8, y8, &buf_y, &buf_u, &buf_v) {
                            Ok(_) => (),
                            Err(e) => result = Err(e),
                        }
                    }) {
                        Ok(_) => (),
                        Err(_) => break,
                    }
                    if result.is_err() {
                        break;
                    }
                }
                cursor += len + 1;
            }
        }
        result
    }

    pub fn decode_chunk(src: &[u8]) -> Result<([u8; 64], [u8; 64], [u8; 64]), DecodeError> {
        let mut vec = Vec::<u8, UNCOMPRESSED_SIZE>::new();
        chunk::decompress(src, &mut vec).ok_or(DecodeError::InvalidData)?;

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
impl<T: PixelColor + From<MpicRgb666>> ImageDrawable for Decoder<'_, T> {
    type Color = T;

    fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let rect = target.bounding_box();
        self.decode_sub_image(
            rect.top_left.x,
            rect.top_left.y,
            rect.size.width,
            rect.size.height,
            |x8, y8, buf_y, buf_u, buf_v| {
                let mut colors = [T::from(MpicRgb666::new(0, 0, 0)); 64];
                for index in 0..64 {
                    let rgb =
                        MpicRgb666::from(MpicYuv666::new(buf_y[index], buf_u[index], buf_v[index]));
                    colors[index] = rgb.into();
                }
                target.fill_contiguous(
                    &Rectangle::new(Point::new(x8 as i32, y8 as i32), Size::new(8, 8)),
                    colors,
                )
            },
        )
    }

    fn draw_sub_image<D>(&self, target: &mut D, area: &Rectangle) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
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

#[inline]
const fn ceil_7(v: u32) -> u32 {
    ((v + 7) / 8) * 8
}
