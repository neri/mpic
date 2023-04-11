# MPIC

Simple Lossy Compression Image Format for Embedded Platforms

![](images/img_3246.jpg)

## Features

- Simple.
- Lossy compression.
- Small memory footprint, only a few hundred bytes of stack memory required for decoding.
- Designed for 16bpp color images and supports `embedded-graphics`; add `features = ["embedded"]` to Cargo.toml.
- Support for `no_std`, No `alloc` is needed for decoding.

## How it works

- Divide the image into blocks of 8 x 8 pixels.
- Convert RGB with 8 bits per channel to YUV with 6 bits per channel.
- Thin out the U and V channels to 1/4.
- Because the color difference information is thinned out, even in the worst case, the compression is guaranteed to be more than half of the raw bitmap.
- Finally, lossless compression is performed using the sliding dictionary method.
- When decoding, these processes are performed in reverse order.

## Comparison with sample images

| Mandrill              | Original Size | Converted  PNG Size |                              |
| --------------------- | ------------- | ------------------- | ---------------------------- |
| Original 24bit Bitmap | 197KB         | 155KB               | ![](images/Mandrill-org.png) |
| MPIC                  | 75KB          | 135KB               | ![](images/Mandrill.png)     |
| JPEG                  | 40KB          | -                   | ![](images/Mandrill.jpeg)    |

| Parrots               | Original Size | Converted  PNG Size |                             |
| --------------------- | ------------- | ------------------- | --------------------------- |
| Original 24bit Bitmap | 197KB         | 105KB               | ![](images/Parrots-org.png) |
| MPIC                  | 59KB          | 87KB                | ![](images/Parrots.png)     |
| JPEG                  | 21KB          | -                   | ![](images/Parrots.jpeg)    |

| Pepper                | Original Size | Converted  PNG Size |                            |
| --------------------- | ------------- | ------------------- | -------------------------- |
| Original 24bit Bitmap | 197KB         | 117KB               | ![](images/Pepper-org.png) |
| MPIC                  | 69KB          | 105KB               | ![](images/Pepper.png)     |
| JPEG                  | 28KB          | -                   | ![](images/Pepper.jpeg)    |

## File Format

### File Header

- All multi-byte data is encoded in little-endian.

```
#[repr(C, packed)]
pub struct FileHeader {
    magic: [u8; 4], // b"\x00mpi"
    width: u16,
    height: u16,
    version: u8,    // must be zero
}
```

- At this time, only multiples of 8 are allowed for `width` and `height`.

### Chunk Data

- Image data is divided into 8 x 8 blocks and stored in chunks.
- Number of Chunks = ceil(`width` / 8) * ceil(`height` / 8)
- For uncompressed chunks, the data size (96), followed by the 64-byte Y channel, 16-byte U channel, and 16-byte V channel. `96` also serves as an identifier for uncompressed data.
- The Y channel stores all 8x8 data, while the U and V channels store only 4x4 pixels. The method of thinning the U and V channels is left to the encoder. The decoder should use nearest-neighbor interpolation to expand them by a factor of 2 in height and width.
- For a 6-bit compacted chunk, the data size is `72`. The order of the data is the same as for the uncompressed chunk, but the 6 bits of the uncompressed chunk are compacted into 8 bits, so the data size is 3/4 of the uncompressed chunk.
- If the data size after compression exceeds 72 with other compression methods, the 6-bit compaction method shall be selected.

### LZ Compression Data Encoding

| Representation          | Meaning                                                                                                             |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------- |
| `00vv_vvvv`             | Raw Value                                                                                                           |
| `01nn_nnnn` `0mmm_mmmm` | Together with the trailing byte value, it indicates the length `(n+3)` and offset `-(m+1)` of the slide dictionary. |
| `1xxx_xxxx`             | Reserved                                                                                                            |

## License

MIT

(C) 2023 Nerry
