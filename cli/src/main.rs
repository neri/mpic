//! mpic cli sample app

use std::{
    env,
    path::{Path, PathBuf},
    process::exit,
};

enum Format {
    MPic,
    Image(image::ImageFormat),
}

fn main() {
    let mut args = env::args();
    let _ = args.next().unwrap();

    let input = match args.next() {
        Some(v) => PathBuf::from(v),
        None => usage(),
    };

    let ext = input.extension().expect("unknown file extention");
    let input_format = match ext {
        _ if ext == mpic::PREFERRED_FILE_EXT => Format::MPic,
        _ => {
            Format::Image(image::ImageFormat::from_extension(ext).expect("unknown file extention"))
        }
    };

    let output = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| match input_format {
            Format::MPic => input.with_extension("png"),
            _ => input.with_extension(mpic::PREFERRED_FILE_EXT),
        });

    let ext = output.extension().expect("unknown file extention");
    let output_format = match ext {
        _ if ext == mpic::PREFERRED_FILE_EXT => Format::MPic,
        _ => {
            Format::Image(image::ImageFormat::from_extension(ext).expect("unknown file extention"))
        }
    };

    let read_data = std::fs::read(&input).expect("cannot read input");

    let dynamic_image = match input_format {
        Format::Image(format) => image::load_from_memory_with_format(&read_data, format)
            .expect("cannot decode input file"),
        Format::MPic => {
            let decoder = mpic::Decoder::<()>::new(read_data.as_slice()).unwrap();
            let info = decoder.info();
            let mut raw_image = vec![0xCCu8; info.width() as usize * info.height() as usize * 3];
            decoder
                .decode(|x, y, rgb| {
                    let base = (x as usize + y as usize * info.width() as usize) * 3;
                    raw_image[base] = rgb.r;
                    raw_image[base + 1] = rgb.g;
                    raw_image[base + 2] = rgb.b;
                })
                .expect("cannot decode input file");
            image::DynamicImage::ImageRgb8(
                image::RgbImage::from_raw(info.width(), info.height(), raw_image).unwrap(),
            )
        }
    };

    match output_format {
        Format::Image(format) => dynamic_image
            .save_with_format(&output, format)
            .expect("cannot write output"),
        Format::MPic => {
            let rgb = dynamic_image.to_rgb8();
            let raw_image = rgb.as_raw();

            let mut output_buf = Vec::new();
            mpic::Encoder::encode(&raw_image, rgb.width(), rgb.height(), |v| {
                output_buf.extend_from_slice(v)
            })
            .expect("cannot write output");
            std::fs::write(&output, output_buf).expect("cannot write output");
        }
    }
}

fn usage() -> ! {
    let mut args = env::args_os();
    let arg = args.next().unwrap();
    let path = Path::new(&arg);
    let lpc = path.file_name().unwrap();
    eprintln!("usage: {} INFILE [OUTFILE]", lpc.to_str().unwrap());
    exit(1);
}
