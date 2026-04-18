use crate::lz::CompressionLevel;
use crate::{chunk, demosaic_uv, mosaic_uv};
use alloc::vec::Vec as VecA;
use core::mem::swap;
use heapless::Vec as VecH;

#[test]
fn mosaic() {
    let original_left: [u8; 64] = [
        0x89, 0x89, 0x1b, 0x1b, 0x78, 0x78, 0xbb, 0xbb, //
        0x89, 0x89, 0x1b, 0x1b, 0x78, 0x78, 0xbb, 0xbb, //
        0x80, 0x80, 0xc8, 0xc8, 0x67, 0x67, 0xcd, 0xcd, //
        0x80, 0x80, 0xc8, 0xc8, 0x67, 0x67, 0xcd, 0xcd, //
        0x68, 0x68, 0xd8, 0xd8, 0xe5, 0xe5, 0x4e, 0x4e, //
        0x68, 0x68, 0xd8, 0xd8, 0xe5, 0xe5, 0x4e, 0x4e, //
        0x21, 0x21, 0x6b, 0x6b, 0x1a, 0x1a, 0x97, 0x97, //
        0x21, 0x21, 0x6b, 0x6b, 0x1a, 0x1a, 0x97, 0x97, //
    ];
    let original_right: [u8; 64] = [
        0xf3, 0xf3, 0x75, 0x75, 0xd8, 0xd8, 0x96, 0x96, //
        0xf3, 0xf3, 0x75, 0x75, 0xd8, 0xd8, 0x96, 0x96, //
        0xb1, 0xb1, 0x97, 0x97, 0x30, 0x30, 0xed, 0xed, //
        0xb1, 0xb1, 0x97, 0x97, 0x30, 0x30, 0xed, 0xed, //
        0x7f, 0x7f, 0x26, 0x26, 0xbd, 0xbd, 0x56, 0x56, //
        0x7f, 0x7f, 0x26, 0x26, 0xbd, 0xbd, 0x56, 0x56, //
        0xdb, 0xdb, 0xc8, 0xc8, 0x5a, 0x5a, 0x95, 0x95, //
        0xdb, 0xdb, 0xc8, 0xc8, 0x5a, 0x5a, 0x95, 0x95, //
    ];
    let expected_left: [u8; 16] = [
        0x89, 0x1b, 0x78, 0xbb, //
        0x80, 0xc8, 0x67, 0xcd, //
        0x68, 0xd8, 0xe5, 0x4e, //
        0x21, 0x6b, 0x1a, 0x97, //
    ];
    let expected_right: [u8; 16] = [
        0xf3, 0x75, 0xd8, 0x96, //
        0xb1, 0x97, 0x30, 0xed, //
        0x7f, 0x26, 0xbd, 0x56, //
        0xdb, 0xc8, 0x5a, 0x95, //
    ];

    let (left, right) = mosaic_uv(&original_left, &original_right);
    assert_eq!(left, expected_left);
    assert_eq!(right, expected_right);

    let left2 = demosaic_uv(&left);
    let right2 = demosaic_uv(&right);
    assert_eq!(left2, original_left);
    assert_eq!(right2, original_right);
}

macro_rules! test_compress {
    ($ident:ident, $source:expr, $size_is_compressed:ident) => {
        #[test]
        fn $ident() {
            let source = $source;
            assert_eq!(source.len(), 96);
            let source = source.into_array().unwrap();

            let mut vec1 = VecH::new();
            chunk::compress_fast(&source, &mut vec1);

            $size_is_compressed(vec1.len());

            let mut vec2 = VecH::new();
            chunk::decompress(&vec1, &mut vec2).unwrap();
            assert_eq!(&source, vec2.as_slice());

            for level in [
                CompressionLevel::Fast,
                CompressionLevel::Default,
                CompressionLevel::Best,
            ] {
                let mut vec3 = VecH::new();
                chunk::compress_lazy(&source, &mut vec3, level);

                $size_is_compressed(vec3.len());

                let mut vec4 = VecH::new();
                chunk::decompress(&vec3, &mut vec4).unwrap();
                assert_eq!(&source, vec4.as_slice());
            }
        }
    };
    ($ident:ident, $source:expr) => {
        test_compress!($ident, $source, is_compressed);
    };
}

#[allow(unused)]
fn dump(data: &[u8]) {
    for (index, line) in data.chunks(16).enumerate() {
        print!("{:04x}: ", index * 16);
        for byte in line {
            print!("{:02x} ", byte);
        }
        println!();
    }
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
    let mut source = VecH::<u8, 96>::new();
    for _ in 0..96 {
        source.push(0).unwrap();
    }
    source
});

test_compress!(compress_all_max, {
    let mut source = VecH::<u8, 96>::new();
    for _ in 0..96 {
        source.push(0x3F).unwrap();
    }
    source
});

test_compress!(compress_all_test_2a, {
    let mut source = VecH::<u8, 96>::new();
    for _ in 0..96 {
        source.push(0x2A).unwrap();
    }
    source
});

test_compress!(compress_all_test_15, {
    let mut source = VecH::<u8, 96>::new();
    for _ in 0..96 {
        source.push(0x15).unwrap();
    }
    source
});

test_compress!(compress_ordered_compressed, {
    let mut source = VecH::<u8, 96>::new();
    for i in 0..96 {
        source.push(i & 0x3F).unwrap();
    }
    source
});

test_compress!(
    compress_ordered_double_not_compressed,
    {
        let mut source = VecH::<u8, 96>::new();
        for i in 0..96 {
            source.push(i >> 1).unwrap();
        }
        source
    },
    is_not_compressed
);

test_compress!(compress_ordered_3_compressed, {
    let mut source = VecH::<u8, 96>::new();
    for i in 0..96 {
        source.push(i / 3).unwrap();
    }
    source
});

test_compress!(compress_ordered_4_compressed, {
    let mut source = VecH::<u8, 96>::new();
    for i in 0..96 {
        source.push(i / 4).unwrap();
    }
    source
});

test_compress!(compress_ordered_5_compressed, {
    let mut source = VecH::<u8, 96>::new();
    for i in 0..96 {
        source.push(i / 5).unwrap();
    }
    source
});

test_compress!(compress_ordered_6_compressed, {
    let mut source = VecH::<u8, 96>::new();
    for i in 0..96 {
        source.push(i / 6).unwrap();
    }
    source
});

test_compress!(compress_ordered_7_compressed, {
    let mut source = VecH::<u8, 96>::new();
    for i in 0..96 {
        source.push(i / 7).unwrap();
    }
    source
});

test_compress!(compress_ordered_8_compressed, {
    let mut source = VecH::<u8, 96>::new();
    for i in 0..96 {
        source.push(i / 8).unwrap();
    }
    source
});

test_compress!(compress_ordered_16_compressed, {
    let mut source = VecH::<u8, 96>::new();
    for i in 0..96 {
        source.push(i / 16).unwrap();
    }
    source
});

test_compress!(compress_ordered_24_compressed, {
    let mut source = VecH::<u8, 96>::new();
    for i in 0..96 {
        source.push(i / 24).unwrap();
    }
    source
});

test_compress!(compress_ordered_32_compressed, {
    let mut source = VecH::<u8, 96>::new();
    for i in 0..96 {
        source.push(i / 32).unwrap();
    }
    source
});

test_compress!(compress_ordered_48_compressed, {
    let mut source = VecH::<u8, 96>::new();
    for i in 0..96 {
        source.push(i / 48).unwrap();
    }
    source
});

test_compress!(compress_repeat_2, {
    let mut source = VecH::<u8, 96>::new();
    for _ in 0..48 {
        source.push(0x01).unwrap();
        source.push(0x34).unwrap();
    }
    source
});

test_compress!(compress_repeat_4, {
    let mut source = VecH::<u8, 96>::new();
    for _ in 0..24 {
        source.push(0x15).unwrap();
        source.push(0x2A).unwrap();
        source.push(0x0C).unwrap();
        source.push(0x33).unwrap();
    }
    source
});

test_compress!(compress_repeat_8, {
    let mut source = VecH::<u8, 96>::new();
    for _ in 0..12 {
        source.push(0x15).unwrap();
        source.push(0x2A).unwrap();
        source.push(0x0C).unwrap();
        source.push(0x33).unwrap();
        source.push(0x0F).unwrap();
        source.push(0x30).unwrap();
        source.push(0x14).unwrap();
        source.push(0x28).unwrap();
    }
    source
});

test_compress!(compress_repeat_checked, {
    let mut source = VecH::<u8, 96>::new();
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

test_compress!(compress_fib, {
    let mut source = VecH::<u8, 96>::new();
    let fib = fib_str(1, 2, 96);
    source.extend_from_slice(&fib).unwrap();
    source
});

/// A Fibonacci word generator for testing purposes.
#[cfg(test)]
pub(crate) fn fib_str(a: u8, b: u8, limit: usize) -> VecA<u8> {
    let mut n = 1;
    let mut x = VecA::new();
    let mut y = VecA::new();
    let mut c = VecA::new();
    while x.len() < limit {
        match n {
            0 => {}
            1 => x.push(a),
            2 => y.push(b),
            _ => {
                c.clear();
                c.extend_from_slice(&x);
                c.extend_from_slice(&y);
                swap(&mut x, &mut y);
                swap(&mut x, &mut c);
            }
        }
        n += 1;
    }
    x.truncate(limit);
    x
}
