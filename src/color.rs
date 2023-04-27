#[cfg(feature = "embedded")]
use embedded_graphics::pixelcolor::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MpicRgb888 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MpicRgb666 {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MpicYuv666 {
    pub y: u8,
    pub u: u8,
    pub v: u8,
}

impl MpicRgb888 {
    #[inline]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    #[inline]
    pub fn from_yuv(yuv: MpicYuv666) -> Self {
        MpicRgb666::from_yuv(yuv).into_rgb888()
    }
}

impl MpicRgb666 {
    #[inline]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    #[inline]
    pub fn from_yuv(yuv: MpicYuv666) -> Self {
        let y = u6_to_u8(yuv.y.wrapping_sub(4)) as i32;
        let u = (u6_to_u8(yuv.u) as i32).wrapping_sub(128);
        let v = (u6_to_u8(yuv.v) as i32).wrapping_sub(128);

        let r = ((298 * y + 409 * v + 128).wrapping_shr(10)).max(0).min(63);
        let g = ((298 * y - 100 * u - 208 * v + 128).wrapping_shr(10))
            .max(0)
            .min(63);
        let b = ((298 * y + 516 * u + 128).wrapping_shr(10)).max(0).min(63);

        Self {
            r: r as u8,
            g: g as u8,
            b: b as u8,
        }
    }

    #[inline]
    pub const fn into_rgb888(self) -> MpicRgb888 {
        MpicRgb888 {
            r: self.r8(),
            g: self.g8(),
            b: self.b8(),
        }
    }

    #[inline]
    pub const fn r8(&self) -> u8 {
        u6_to_u8(self.r)
    }

    #[inline]
    pub const fn g8(&self) -> u8 {
        u6_to_u8(self.g)
    }

    #[inline]
    pub const fn b8(&self) -> u8 {
        u6_to_u8(self.b)
    }
}

impl MpicYuv666 {
    #[inline]
    pub const fn new(y: u8, u: u8, v: u8) -> Self {
        Self { y, u, v }
    }

    #[inline]
    pub const fn from_rgb(rgb: MpicRgb888) -> Self {
        let r = rgb.r as i32;
        let g = rgb.g as i32;
        let b = rgb.b as i32;

        let y = ((66 * r + 129 * g + 25 * b + 128).wrapping_shr(10)).wrapping_add(4);
        let u = ((-38 * r - 74 * g + 112 * b + 128) / 256) + 128;
        let v = ((112 * r - 94 * g - 18 * b + 128) / 256) + 128;

        Self {
            y: y as u8,
            u: u.wrapping_shr(2) as u8,
            v: v.wrapping_shr(2) as u8,
        }
    }
}

impl From<MpicRgb888> for MpicYuv666 {
    #[inline]
    fn from(rgb: MpicRgb888) -> Self {
        Self::from_rgb(rgb)
    }
}

impl From<MpicYuv666> for MpicRgb666 {
    #[inline]
    fn from(yuv: MpicYuv666) -> Self {
        Self::from_yuv(yuv)
    }
}

impl From<MpicRgb666> for MpicRgb888 {
    #[inline]
    fn from(value: MpicRgb666) -> Self {
        value.into_rgb888()
    }
}

macro_rules! from_rgb {
    ($ident:ident, $shift_r:expr, $shift_g:expr, $shift_b:expr) => {
        #[cfg(feature = "embedded")]
        impl From<MpicRgb666> for $ident {
            #[inline]
            fn from(rgb: MpicRgb666) -> Self {
                Self::new(
                    rgb.r.wrapping_shr($shift_r - 2),
                    rgb.g.wrapping_shr($shift_g - 2),
                    rgb.b.wrapping_shr($shift_b - 2),
                )
            }
        }
    };
    ($ident:ident) => {
        #[cfg(feature = "embedded")]
        impl From<MpicRgb666> for $ident {
            #[inline]
            fn from(rgb: MpicRgb666) -> Self {
                Self::new(u6_to_u8(rgb.r), u6_to_u8(rgb.g), u6_to_u8(rgb.b))
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
from_rgb!(Rgb888);
from_rgb!(Bgr888);

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

#[test]
fn rgb_yuv() {
    let allowed_rgb_error = 12;
    let allowed_yuv_error = 1;

    let mut max_y = u8::MIN;
    let mut min_y = u8::MAX;
    let mut max_u = u8::MIN;
    let mut min_u = u8::MAX;
    let mut max_v = u8::MIN;
    let mut min_v = u8::MAX;

    fn rgb_error(lhs: MpicRgb888, rhs: MpicRgb888) -> isize {
        let l1 = lhs.r as isize;
        let l2 = lhs.g as isize;
        let l3 = lhs.b as isize;
        let r1 = rhs.r as isize;
        let r2 = rhs.g as isize;
        let r3 = rhs.b as isize;

        let diff1 = (l1 - r1).abs();
        let diff2 = (l2 - r2).abs();
        let diff3 = (l3 - r3).abs();

        diff1.max(diff2).max(diff3)
    }

    fn yuv_error(lhs: MpicYuv666, rhs: MpicYuv666) -> isize {
        let l1 = lhs.y as isize;
        let l2 = lhs.u as isize;
        let l3 = lhs.v as isize;
        let r1 = rhs.y as isize;
        let r2 = rhs.u as isize;
        let r3 = rhs.v as isize;

        let diff1 = (l1 - r1).abs();
        let diff2 = (l2 - r2).abs();
        let diff3 = (l3 - r3).abs();

        diff1.max(diff2).max(diff3)
    }

    for r in 0..64 {
        for g in 0..64 {
            for b in 0..64 {
                let r = u6_to_u8(r);
                let g = u6_to_u8(g);
                let b = u6_to_u8(b);
                let rgb = MpicRgb888 { r, g, b };
                let yuv = MpicYuv666::from_rgb(rgb);
                max_y = max_y.max(yuv.y);
                min_y = min_y.min(yuv.y);
                max_u = max_u.max(yuv.u);
                min_u = min_u.min(yuv.u);
                max_v = max_v.max(yuv.v);
                min_v = min_v.min(yuv.v);

                let rgb2 = MpicRgb888::from_yuv(yuv);
                let yuv2 = MpicYuv666::from_rgb(rgb2);

                let rgb_error = rgb_error(rgb, rgb2);
                let yuv_error = yuv_error(yuv, yuv2);

                assert!(
                    rgb_error <= allowed_rgb_error,
                    "RGB Error exceeded limit: {} > {} RGB {:?} => YUV {:?} => {:?} ",
                    rgb_error,
                    allowed_rgb_error,
                    (rgb.r, rgb.g, rgb.b),
                    (yuv.y, yuv.u, yuv.v),
                    (rgb2.r, rgb2.g, rgb2.b),
                );

                assert!(
                    yuv_error <= allowed_yuv_error,
                    "YUV Error exceeded limit: {} > {} RGB {:?} => YUV {:?} => {:?} => {:?}",
                    yuv_error,
                    allowed_yuv_error,
                    (rgb.r, rgb.g, rgb.b),
                    (yuv.y, yuv.u, yuv.v),
                    (rgb2.r, rgb2.g, rgb2.b),
                    (yuv2.y, yuv2.u, yuv2.v),
                );
            }
        }
    }

    assert_eq!(
        (min_y, min_u, min_v, max_y, max_u, max_v),
        (4, 4, 4, 58, 60, 60)
    );
}
