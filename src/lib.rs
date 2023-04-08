//! mPic - Simple Lossy Compression Image Format for Embedded Platforms

#![cfg_attr(not(test), no_std)]
#![feature(const_trait_impl)]
#![feature(maybe_uninit_uninit_array)]

use core::{mem::size_of, slice};
#[cfg(feature = "embedded")]
use embedded_graphics::prelude::Size;

mod decode;
pub use decode::*;

mod encode;
pub use encode::*;

mod chunk;
pub mod color;

#[cfg(test)]
mod test;

pub const PREFERRED_FILE_EXT: &str = "mpic";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EncodeError {
    InvalidInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DecodeError {
    InvalidInput,
    InvalidData,
}

#[repr(C, packed)]
pub struct FileHeader {
    magic: [u8; 4],
    width: u16,
    height: u16,
    version: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImageInfo {
    pub width: u16,
    pub height: u16,
}

impl ImageInfo {
    #[inline]
    pub fn width(&self) -> u32 {
        self.width as u32
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height as u32
    }
}

#[cfg(feature = "embedded")]
impl From<ImageInfo> for Size {
    #[inline]
    fn from(info: ImageInfo) -> Self {
        Self::new(info.width as u32, info.height as u32)
    }
}

impl FileHeader {
    pub const MINIMAL_SIZE: usize = size_of::<Self>();

    pub const MAGIC: [u8; 4] = *b"\x00mpi";

    pub const VER_CURRENT: u8 = 0;

    #[inline]
    pub fn from_bytes<'a>(blob: &'a [u8]) -> Option<&'a Self> {
        if blob.len() < Self::MINIMAL_SIZE {
            return None;
        }
        Some(unsafe { &*(blob.as_ptr() as *const FileHeader) })
    }

    #[inline]
    pub fn bytes<'a>(&'a self) -> &'a [u8] {
        unsafe { slice::from_raw_parts(self as *const _ as *const u8, size_of::<Self>()) }
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        let width = self.width.to_le();
        let height = self.height.to_le();
        self.magic == Self::MAGIC
            && self.version == Self::VER_CURRENT
            && width > 0
            && (width & 7) == 0
            && height > 0
            && (height & 7) == 0
    }

    #[inline]
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::VER_CURRENT,
            width: if width < 0xFFFF {
                (width as u16).to_le()
            } else {
                0
            },
            height: if height < 0xFFFF {
                (height as u16).to_le()
            } else {
                0
            },
        }
    }

    #[inline]
    pub fn info(&self) -> ImageInfo {
        ImageInfo {
            width: self.width.to_le(),
            height: self.height.to_le(),
        }
    }
}
