//! mPic - Simple Lossy Compression Image Format for Embedded Platforms
//!
//! ## Features
//!
//! - Simple.
//! - Lossy compression.
//! - Small memory footprint, only a few hundred bytes of stack memory required for decoding.
//! - Designed for 16bpp color images and supports `embedded-graphics`; add `features = ["embedded"]` to Cargo.toml.
//! - Support for `no_std`, No `alloc` is needed for decoding.

#![cfg_attr(not(test), no_std)]

use core::{mem::size_of, slice};
#[cfg(feature = "embedded")]
use embedded_graphics::prelude::Size;

#[cfg(feature = "alloc")]
extern crate alloc;

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
    width: u16,
    height: u16,
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
    pub const fn new(width: u32, height: u32) -> Option<Self> {
        if width == 0
            || width >= 0x10000
            || (width & 7) != 0
            || height == 0
            || height >= 0x10000
            || (height & 7) != 0
        {
            return None;
        }
        Some(Self {
            magic: Self::MAGIC,
            version: Self::VER_CURRENT,
            width: (width as u16).to_le(),
            height: (height as u16).to_le(),
        })
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
    pub fn from_bytes<'a>(blob: &'a [u8]) -> Option<&'a Self> {
        if blob.len() < Self::MINIMAL_SIZE {
            return None;
        }
        let header = unsafe { &*(blob.as_ptr() as *const FileHeader) };
        header.is_valid().then(|| header)
    }

    #[inline]
    pub fn bytes<'a>(&'a self) -> &'a [u8] {
        unsafe { slice::from_raw_parts(self as *const _ as *const u8, size_of::<Self>()) }
    }

    #[inline]
    pub fn info(&self) -> ImageInfo {
        ImageInfo {
            width: self.width.to_le(),
            height: self.height.to_le(),
        }
    }
}
