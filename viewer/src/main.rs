//! mpic sample app for embedded-graphics

use embedded_graphics::{image::Image, pixelcolor::Rgb565, prelude::*};
use embedded_graphics_simulator::{OutputSettingsBuilder, SimulatorDisplay, Window};
use std::{env, fs::File, io::Read};

fn main() {
    let mut args = env::args();
    let _ = args.next().unwrap();

    let arg = args.next().expect("file name not given");
    let mut file = File::open(&arg).expect("file cannot open");
    let mut data = Vec::new();
    file.read_to_end(&mut data).expect("file cannot read");

    let decoder = mpic::Decoder::<Rgb565>::new(&data).expect("unexpected file format");
    let image_size = decoder.size();
    let window_size = Size::new(
        64.max(image_size.width + 16),
        64.max(image_size.height + 16),
    );
    let padding = Point::new(
        (window_size.width - image_size.width) as i32 / 2,
        (window_size.height - image_size.height) as i32 / 2,
    );
    let image = Image::new(&decoder, padding);
    let mut display = SimulatorDisplay::<Rgb565>::new(window_size);
    image.draw(&mut display).unwrap();

    let output_settings = OutputSettingsBuilder::new().build();
    Window::new("mPic Image Viewer", &output_settings).show_static(&display);
}
