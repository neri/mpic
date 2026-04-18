//! # mPic
//!
//! Simple Lossy Compression Image Format for Embedded Platforms
//!
//! ## Features
//!
//! - Simple.
//! - Lossy compression.
//! - A typical image compression ratio is somewhere between PNG and JPG.
//! - Small memory footprint, only a few hundred bytes of stack memory required for decoding.
//! - Designed for 16bpp color images and supports `embedded-graphics`; add `features = ["embedded"]` to Cargo.toml.
//! - Support for `no_std`, No `alloc` is needed for decoding.
//!
//! ### Suitability
//!
//! - Good for:
//!   - Photographic images
//! - Not recommended for:
//!   - Pixel art
//!   - Grayscale images
//!
//! ### MSRV
//!
//! - The latest version is recommended whenever possible.
//!
//! ## More information
//!
//! - See detail: <https://github.com/neri/mpic>
//!

#![cfg_attr(not(test), no_std)]

use core::{mem::size_of, slice};
#[cfg(feature = "embedded")]
use embedded_graphics::prelude::Size;
use heapless::Vec;

#[cfg(feature = "alloc")]
extern crate alloc;

mod decode;
pub use decode::*;

mod encode;
pub use encode::*;

mod chunk;
pub mod color;

#[path = "lz/lz.rs"]
pub mod lz;

#[cfg(test)]
mod test;

/// Preferred file extension for MPIC format. (`"mpic"`)
pub const PREFERRED_FILE_EXT: &str = "mpic";

/// Errors that can occur during encoding.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EncodeError {
    /// Invalid input data, such as incorrect length or invalid format.
    InvalidInput,
}

/// Errors that can occur during decoding.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DecodeError {
    /// Invalid input data, such as incorrect length.
    InvalidInput,
    /// Data is corrupted or cannot be decoded.
    InvalidData,
}

/// File header for MPIC format.
#[repr(C, packed)]
pub struct FileHeader {
    magic: [u8; 4],
    width: u16,
    height: u16,
    version: Version,
}

/// Image information extracted from the file header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImageInfo {
    width: u16,
    height: u16,
}

impl ImageInfo {
    /// Return the width of the image in pixels.
    #[inline]
    pub fn width(&self) -> u32 {
        self.width as u32
    }

    /// Return the height of the image in pixels.
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
    /// Minimal valid file size
    pub const MINIMAL_SIZE: usize = size_of::<Self>();

    /// Magic number (`b"\x00mpi"`)
    pub const MAGIC: [u8; 4] = *b"\x00mpi";

    /// Create a new file header with the given width and height.
    #[inline]
    pub const fn new(width: u32, height: u32) -> Option<Self> {
        if width == 0 || width > 0xFFFF || height == 0 || height > 0xFFFF {
            return None;
        }
        let version = if (width & 7) == 0 && (height & 7) == 0 {
            Version::V0
        } else {
            Version::V1
        };
        Some(Self {
            magic: Self::MAGIC,
            version,
            width: (width as u16).to_le(),
            height: (height as u16).to_le(),
        })
    }

    /// Returns true if the file header is valid, false otherwise.
    #[inline]
    pub fn is_valid(&self) -> bool {
        let width = self.width.to_le();
        let height = self.height.to_le();
        self.magic == Self::MAGIC
            && (Version::V0..=Version::CURRENT).contains(&self.version)
            && width > 0
            && height > 0
    }

    /// Create a file header from a byte slice.
    #[inline]
    pub fn from_bytes<'a>(blob: &'a [u8]) -> Option<&'a Self> {
        if blob.len() < Self::MINIMAL_SIZE {
            return None;
        }
        let header = unsafe { &*(blob.as_ptr() as *const FileHeader) };
        header.is_valid().then(|| header)
    }

    /// Return the raw bytes of the file header.
    #[inline]
    pub fn bytes<'a>(&'a self) -> &'a [u8] {
        unsafe { slice::from_raw_parts(self as *const _ as *const u8, size_of::<Self>()) }
    }

    /// Return the image information from the file header.
    #[inline]
    pub fn info(&self) -> ImageInfo {
        ImageInfo {
            width: self.width.to_le(),
            height: self.height.to_le(),
        }
    }

    /// Return the version of the file header.
    #[inline]
    pub const fn version(&self) -> Version {
        self.version
    }
}

/// Format version
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub u8);

impl Version {
    /// Current version
    pub const CURRENT: Self = Self::V1;

    /// Version 0: Only supports images with width and height that are multiples of 8.
    pub const V0: Self = Self(0);
    /// Version 1: Current version
    pub const V1: Self = Self(1);
}
