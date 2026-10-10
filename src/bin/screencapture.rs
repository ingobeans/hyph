use std::{io::BufRead, path::PathBuf, sync::mpsc::{self, Sender, TryRecvError}, thread::{self, JoinHandle}, time::Duration};

use ab_glyph::FontRef;
use chrono::Local;
use image::{DynamicImage, Rgba, RgbaImage};
use win_screenshot::prelude::*;

fn take_screenshot(save_path:&str) {
    let buf = capture_display().unwrap();
    
    let mut img = DynamicImage::ImageRgba8(
        RgbaImage::from_raw(buf.width, buf.height, buf.pixels).unwrap());


    let date = Local::now();
    let unix_time = date.timestamp();
    let time_text = format!("{}", date.format("%H:%M:%S"));

    let font = FontRef::try_from_slice(include_bytes!("../../assets/DejaVuSans.ttf")).unwrap();

    let x = img.width() - 200;
    imageproc::drawing::draw_text_mut(&mut img, Rgba ([255,255,255,255]), x as i32, 5, 51.0, &font , &time_text);
    imageproc::drawing::draw_text_mut(&mut img, Rgba ([0,0,0,255]), x as i32, 50, 24.0, &font , &time_text);

    img.to_rgb8().save(PathBuf::from(save_path).join(format!("{unix_time}.jpg"))).unwrap();
}

fn start_screencapture_thread(save_path:String) ->( JoinHandle<()>,Sender<()>){
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        loop {
            match rx.try_recv() {
                Ok(_) | Err(TryRecvError::Disconnected) => {
                    println!("stopping screen capture.");
                    break;
                }
                Err(TryRecvError::Empty) => {}
            }
            take_screenshot(&save_path);
            thread::sleep(Duration::from_secs(2));
        }
    });
    (handle,tx)
}

fn main() {
    let (handle,tx) = start_screencapture_thread("capture/".to_string());

    println!("press enter to stop thread: ");
    let mut line = String::new();
    let stdin = std::io::stdin();
    let _ = stdin.lock().read_line(&mut line);

    let _ = tx.send(());

    handle.join().unwrap();
}