use crate::{chunk, color::*, demosaic_uv, mosaic_uv};
use heapless::Vec;

#[test]
fn rgb_yuv() {
    let allowed_error = 11;

    let mut max_y = u8::MIN;
    let mut min_y = u8::MAX;
    let mut max_u = u8::MIN;
    let mut min_u = u8::MAX;
    let mut max_v = u8::MIN;
    let mut min_v = u8::MAX;

    fn rgb_error(lhs: Rgb, rhs: Rgb) -> isize {
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

    for r in 0..64 {
        for g in 0..64 {
            for b in 0..64 {
                let r = u6_to_u8(r);
                let g = u6_to_u8(g);
                let b = u6_to_u8(b);
                let rgb = Rgb { r, g, b };
                let yuv = Yuv666::from_rgb(rgb);
                max_y = max_y.max(yuv.y);
                min_y = min_y.min(yuv.y);
                max_u = max_u.max(yuv.u);
                min_u = min_u.min(yuv.u);
                max_v = max_v.max(yuv.v);
                min_v = min_v.min(yuv.v);

                let rgb2 = Rgb::from_yuv(yuv);
                let error = rgb_error(rgb, rgb2);

                assert!(
                    error <= allowed_error,
                    "MAX_ERROR {} RGB {:?} => YUV {:?} => {:?} ",
                    error,
                    (rgb.r, rgb.g, rgb.b),
                    (yuv.y, yuv.u, yuv.v),
                    (rgb2.r, rgb2.g, rgb2.b),
                );
            }
        }
    }

    assert_eq!(
        (min_y, min_u, min_v, max_y, max_u, max_v),
        (4, 4, 4, 58, 60, 60)
    );
}

#[test]
fn mosaic() {
    #[rustfmt::skip]
    let original: [u8; 64] = [
        0x89, 0x89, 0x1b, 0x1b, 0x78, 0x78, 0xbb, 0xbb,
        0x89, 0x89, 0x1b, 0x1b, 0x78, 0x78, 0xbb, 0xbb,
        0x80, 0x80, 0xc8, 0xc8, 0x67, 0x67, 0xcd, 0xcd,
        0x80, 0x80, 0xc8, 0xc8, 0x67, 0x67, 0xcd, 0xcd,
        0x68, 0x68, 0xd8, 0xd8, 0xe5, 0xe5, 0x4e, 0x4e,
        0x68, 0x68, 0xd8, 0xd8, 0xe5, 0xe5, 0x4e, 0x4e,
        0x21, 0x21, 0x6b, 0x6b, 0x1a, 0x1a, 0x97, 0x97,
        0x21, 0x21, 0x6b, 0x6b, 0x1a, 0x1a, 0x97, 0x97,
    ];
    #[rustfmt::skip]
    let mosaiced_expectation: [u8; 16] = [
        0x89, 0x1b, 0x78, 0xbb,
        0x80, 0xc8, 0x67, 0xcd,
        0x68, 0xd8, 0xe5, 0x4e,
        0x21, 0x6b, 0x1a, 0x97,
    ];

    let mosaiced = mosaic_uv(&original);
    assert_eq!(mosaiced, mosaiced_expectation);

    let demosaiced = demosaic_uv(&mosaiced);
    assert_eq!(demosaiced, original);
}

macro_rules! test_compress {
    ($ident:ident, $source:expr, $size_is_compressed:ident) => {
        #[test]
        fn $ident() {
            let source = $source;
            assert_eq!(source.len(), 96);
            let source = source.into_array().unwrap();

            let mut vec1 = Vec::new();
            chunk::compress(&source, &mut vec1);

            $size_is_compressed(vec1.len());

            let mut vec2 = Vec::new();
            chunk::decompress(&vec1, &mut vec2).unwrap();

            assert_eq!(&source, vec2.as_slice());
        }
    };
    ($ident:ident, $source:expr) => {
        test_compress!($ident, $source, is_compressed);
    };
}

fn is_compressed(len: usize) {
    assert!(
        chunk::is_valid_compressed_size(len),
        "is_compressed failed: {}",
        len
    )
}

fn is_not_compressed(len: usize) {
    assert!(
        !chunk::is_valid_compressed_size(len),
        "is_not_compressed failed: {}",
        len
    )
}

test_compress!(compress_all_zero, {
    let mut source = Vec::<u8, 96>::new();
    for _ in 0..96 {
        source.push(0).unwrap();
    }
    source
});

test_compress!(compress_all_max, {
    let mut source = Vec::<u8, 96>::new();
    for _ in 0..96 {
        source.push(0x3F).unwrap();
    }
    source
});

test_compress!(compress_all_test_2a, {
    let mut source = Vec::<u8, 96>::new();
    for _ in 0..96 {
        source.push(0x2A).unwrap();
    }
    source
});

test_compress!(compress_all_test_15, {
    let mut source = Vec::<u8, 96>::new();
    for _ in 0..96 {
        source.push(0x15).unwrap();
    }
    source
});

test_compress!(
    compress_ordered_not_compressed,
    {
        let mut source = Vec::<u8, 96>::new();
        for i in 0..96 {
            source.push(i & 0x3F).unwrap();
        }
        source
    },
    is_not_compressed
);

test_compress!(
    compress_ordered_double_not_compressed,
    {
        let mut source = Vec::<u8, 96>::new();
        for i in 0..96 {
            source.push(i >> 1).unwrap();
        }
        source
    },
    is_not_compressed
);

test_compress!(
    compress_ordered_triple_not_compressed,
    {
        let mut source = Vec::<u8, 96>::new();
        for i in 0..96 {
            source.push(i / 3).unwrap();
        }
        source
    },
    is_not_compressed
);

test_compress!(
    compress_ordered_quadruple_not_compressed,
    {
        let mut source = Vec::<u8, 96>::new();
        for i in 0..96 {
            source.push(i / 4).unwrap();
        }
        source
    },
    is_not_compressed
);

test_compress!(compress_ordered_quintuple_compressed, {
    let mut source = Vec::<u8, 96>::new();
    for i in 0..96 {
        source.push(i / 5).unwrap();
    }
    source
});

test_compress!(compress_repeat_2, {
    let mut source = Vec::<u8, 96>::new();
    for _ in 0..48 {
        source.push(0x01).unwrap();
        source.push(0x34).unwrap();
    }
    source
});

test_compress!(compress_repeat_4, {
    let mut source = Vec::<u8, 96>::new();
    for _ in 0..24 {
        source.push(0x15).unwrap();
        source.push(0x2A).unwrap();
        source.push(0x0C).unwrap();
        source.push(0x33).unwrap();
    }
    source
});

test_compress!(compress_repeat_checked, {
    let mut source = Vec::<u8, 96>::new();
    for _ in 0..4 {
        for _ in 0..4 {
            source.push(0x15).unwrap();
            source.push(0x2A).unwrap();
        }
        for _ in 0..4 {
            source.push(0x2A).unwrap();
            source.push(0x15).unwrap();
        }
    }
    for _ in 0..16 {
        source.push(0x15).unwrap();
    }
    for _ in 0..16 {
        source.push(0x2A).unwrap();
    }
    source
});
