#[cfg(feature = "embedded")]
use embedded_graphics::pixelcolor::{
    Bgr555, Bgr565, Bgr666, Bgr888, Rgb555, Rgb565, Rgb666, Rgb888,
};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Yuv666 {
    pub y: u8,
    pub u: u8,
    pub v: u8,
}

impl Rgb {
    #[inline]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

impl Yuv666 {
    #[inline]
    pub const fn new(y: u8, u: u8, v: u8) -> Self {
        Self { y, u, v }
    }

    #[inline]
    pub const fn from_rgb(rgb: Rgb) -> Self {
        let r = rgb.r as i32;
        let g = rgb.g as i32;
        let b = rgb.b as i32;

        let y = ((66 * r + 129 * g + 25 * b + 128) / 256) + 16;
        let u = ((-38 * r - 74 * g + 112 * b + 128) / 256) + 128;
        let v = ((112 * r - 94 * g - 18 * b + 128) / 256) + 128;

        let y2 = (y >> 2) as u8;
        let u2 = (u >> 2) as u8;
        let v2 = (v >> 2) as u8;

        Self {
            y: y2,
            u: u2,
            v: v2,
        }
    }
}

impl Rgb {
    #[inline]
    pub const fn from_yuv(yuv: Yuv666) -> Rgb {
        let y = (u6_to_u8(yuv.y) as i32) - 16;
        let u = (u6_to_u8(yuv.u) as i32) - 128;
        let v = (u6_to_u8(yuv.v) as i32) - 128;

        let r = ((298 * y + 409 * v + 128) >> 8).max(0).min(255);
        let g = ((298 * y - 100 * u - 208 * v + 128) >> 8).max(0).min(255);
        let b = ((298 * y + 516 * u + 128) >> 8).max(0).min(255);

        Rgb {
            r: r as u8,
            g: g as u8,
            b: b as u8,
        }
    }
}

impl const From<Rgb> for Yuv666 {
    #[inline]
    fn from(rgb: Rgb) -> Self {
        Self::from_rgb(rgb)
    }
}

impl const From<Yuv666> for Rgb {
    #[inline]
    fn from(yuv: Yuv666) -> Self {
        Self::from_yuv(yuv)
    }
}

macro_rules! from_rgb {
    ($ident:ident, $shift_r:expr, $shift_g:expr, $shift_b:expr) => {
        #[cfg(feature = "embedded")]
        impl const From<Rgb> for $ident {
            #[inline]
            fn from(rgb: Rgb) -> Self {
                Self::new(
                    rgb.r.wrapping_shr($shift_r),
                    rgb.g.wrapping_shr($shift_g),
                    rgb.b.wrapping_shr($shift_b),
                )
            }
        }
    };
}

from_rgb!(Rgb555, 3, 3, 3);
from_rgb!(Bgr555, 3, 3, 3);
from_rgb!(Rgb565, 3, 2, 3);
from_rgb!(Bgr565, 3, 2, 3);
from_rgb!(Rgb666, 2, 2, 2);
from_rgb!(Bgr666, 2, 2, 2);
from_rgb!(Rgb888, 0, 0, 0);
from_rgb!(Bgr888, 0, 0, 0);

/// expand 6bit value to 8bit
#[inline]
#[allow(dead_code)]
pub(crate) const fn u6_to_u8(val: u8) -> u8 {
    let val = val.wrapping_shl(2);
    val | val.wrapping_shr(6)
}

/// expand 4bit value to 8bit
#[inline]
#[allow(dead_code)]
pub(crate) const fn u4_to_u8(val: u8) -> u8 {
    val | val.wrapping_shl(4)
}
