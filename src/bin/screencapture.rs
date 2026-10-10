use std::path::PathBuf;

use ab_glyph::FontRef;
use image::{DynamicImage, Rgba, RgbaImage};
use win_screenshot::prelude::*;

fn take_screenshot(save_path:&str) {
    let buf = capture_display().unwrap();
    
    let mut img = DynamicImage::ImageRgba8(
        RgbaImage::from_raw(buf.width, buf.height, buf.pixels).unwrap());


    let font = FontRef::try_from_slice(include_bytes!("../../assets/DejaVuSans.ttf")).unwrap();

    let x = img.width() - 200;
    imageproc::drawing::draw_text_mut(&mut img, Rgba ([255,255,255,255]), x as i32, 5, 51.0, &font , "17:49:32");
    imageproc::drawing::draw_text_mut(&mut img, Rgba ([0,0,0,255]), x as i32, 50, 24.0, &font , "17:49:32");

    img.to_rgb8().save(PathBuf::from(save_path).join(format!("123.jpg"))).unwrap();
}

fn main() {
    take_screenshot("received/");
}