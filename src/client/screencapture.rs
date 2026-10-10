use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread::{self, JoinHandle},
    time::Duration,
};

use ab_glyph::FontRef;
use chrono::Local;
use clonetree::clone_tree;
use image::{DynamicImage, Rgba, RgbaImage};
use win_screenshot::prelude::*;

use crate::utils::zip_dir;

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

pub enum ScreenCaptureStateMessage {
    FrameTaken,
    Stopped,
    FinishedSavingSubmission,
    // FinishedCompilingVideoData,
}

pub fn start_screencapture_thread(
    exam_data_dir: PathBuf,
    output_path: PathBuf,
) -> (
    JoinHandle<()>,
    Sender<PathBuf>,
    Receiver<ScreenCaptureStateMessage>,
) {
    let captures_save_dir = exam_data_dir.join("captures");
    std::fs::create_dir_all(&captures_save_dir).unwrap();
    let (terminate_tx, terminate_rx) = mpsc::channel();
    let (status_tx, status_rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let submission_dir = loop {
            // stop screen capture when message is received over rx.
            // the message also contains a user submitted pathbuf to their
            // exam submission.
            match terminate_rx.try_recv() {
                Ok(data) => {
                    break data;
                }
                Err(TryRecvError::Disconnected) => {
                    panic!()
                }
                Err(TryRecvError::Empty) => {}
            }
            take_screenshot(&captures_save_dir);
            let _ = status_tx.send(ScreenCaptureStateMessage::FrameTaken);

            // arbitrary delay
            thread::sleep(Duration::from_secs(2));
        };
        println!(
            "stopped screen capture. attempting to zip exam data (screen capture + submission)."
        );
        let _ = status_tx.send(ScreenCaptureStateMessage::Stopped);

        // copy exam submission directory
        let new_submission_dir = exam_data_dir.join("submission");
        if new_submission_dir.exists() {
            std::fs::remove_dir_all(&new_submission_dir).unwrap();
        }
        clone_tree(
            submission_dir,
            new_submission_dir,
            &clonetree::Options::default(),
        )
        .unwrap();
        let _ = status_tx.send(ScreenCaptureStateMessage::FinishedSavingSubmission);

        zip_dir(
            &exam_data_dir,
            &output_path,
            zip::CompressionMethod::default(),
        )
        .expect("Couldn't save exam data ZIP");

        println!("finished creating exam data ZIP.");
    });
    (handle, terminate_tx, status_rx)
}
