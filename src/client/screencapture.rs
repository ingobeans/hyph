use std::{
    io::BufRead,
    path::PathBuf,
    sync::mpsc::{self, Sender, TryRecvError},
    thread::{self, JoinHandle},
    time::Duration,
};

use ab_glyph::FontRef;
use chrono::Local;
use image::{DynamicImage, Rgba, RgbaImage};
use win_screenshot::prelude::*;
use zip::write::SimpleFileOptions;

fn take_screenshot(save_path: &PathBuf) {
    let buf = capture_display().unwrap();

    let mut img =
        DynamicImage::ImageRgba8(RgbaImage::from_raw(buf.width, buf.height, buf.pixels).unwrap());

    let date = Local::now();
    let unix_time = date.timestamp();
    let time_text = format!("{}", date.format("%H:%M:%S"));

    let font = FontRef::try_from_slice(include_bytes!("../../ui/assets/DejaVuSans.ttf")).unwrap();

    let x = img.width() - 200;
    imageproc::drawing::draw_text_mut(
        &mut img,
        Rgba([255, 255, 255, 255]),
        x as i32,
        5,
        51.0,
        &font,
        &time_text,
    );
    imageproc::drawing::draw_text_mut(
        &mut img,
        Rgba([0, 0, 0, 255]),
        x as i32,
        50,
        24.0,
        &font,
        &time_text,
    );

    img.to_rgb8()
        .save(save_path.join(format!("{unix_time}.jpg")))
        .unwrap();
}

pub fn start_screencapture_thread(
    save_dir: PathBuf,
    output_path: PathBuf,
) -> (JoinHandle<()>, Sender<()>) {
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        loop {
            // stop screen capture when message is received over rx.
            match rx.try_recv() {
                Ok(_) | Err(TryRecvError::Disconnected) => {
                    break;
                }
                Err(TryRecvError::Empty) => {}
            }
            take_screenshot(&save_dir);

            // arbitrary delay
            thread::sleep(Duration::from_secs(2));
        }

        println!("stopped screen capture. attempting to compile video data.");

        let file = std::fs::File::create(output_path)
            .expect("Couldn't create screen capture output file.");
        let mut zip = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        for item in std::fs::read_dir(&save_dir).unwrap().flatten() {
            let Ok(filetype) = item.file_type() else {
                continue;
            };
            if filetype.is_dir() {
                continue;
            }
            if !item.file_name().to_string_lossy().ends_with(".jpg") {
                continue;
            }

            let path = item.path();
            let path_stripped = path.strip_prefix(&save_dir).unwrap();
            let Some(path_as_string) = path_stripped.to_str().map(str::to_owned) else {
                continue;
            };

            if zip.start_file(&path_as_string, options).is_err() {
                continue;
            }
            let Ok(mut f) = std::fs::File::open(path) else {
                continue;
            };
            let _ = std::io::copy(&mut f, &mut zip);
        }

        zip.finish().expect("Couldn't save screep capture ZIP");
        println!("finished saving video data ZIP.");
    });
    (handle, tx)
}
