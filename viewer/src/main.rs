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
    let image = Image::new(&decoder, Point::new(0, 0));
    let mut display = SimulatorDisplay::<Rgb565>::new(image.bounding_box().size);
    decoder.draw(&mut display).unwrap();

    let output_settings = OutputSettingsBuilder::new().build();
    Window::new("Image Viewer", &output_settings).show_static(&display);
}
