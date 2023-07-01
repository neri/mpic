//! mpic cli sample app

use std::{
    env,
    path::{Path, PathBuf},
    process::exit,
};

enum Format {
    MPic,
    Raw,
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
        _ if ext == "raw" => Format::Raw,
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
            let raw_image = decoder.decode().expect("cannot decode input file");
            image::DynamicImage::ImageRgb8(
                image::RgbImage::from_raw(info.width(), info.height(), raw_image).unwrap(),
            )
        }
        Format::Raw => unreachable!(),
    };

    match output_format {
        Format::Image(format) => dynamic_image
            .save_with_format(&output, format)
            .expect("cannot write output"),
        Format::MPic => {
            let rgb = dynamic_image.to_rgb8();
            let raw_image = rgb.as_raw();

            let output_buf = mpic::Encoder::encode(&raw_image, rgb.width(), rgb.height())
                .expect("cannot write output");
            std::fs::write(&output, output_buf).expect("cannot write output");
        }
        Format::Raw => {
            let rgb = dynamic_image.to_rgb8();
            let raw_image = rgb.as_raw();
            std::fs::write(&output, raw_image).expect("cannot write output");
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
