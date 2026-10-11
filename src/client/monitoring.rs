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

fn take_screenshot(save_path: &PathBuf) -> Result<(), std::io::Error> {
    let buf = capture_display().or(Err(std::io::Error::other("Couldn't take screen capture")))?;

    let mut img = DynamicImage::ImageRgba8(
        RgbaImage::from_raw(buf.width, buf.height, buf.pixels).ok_or(std::io::Error::other(
            "Couldn't convert screen capture to img",
        ))?,
    );

    let date = Local::now();
    let unix_time = date.timestamp();
    let time_text = format!("{}", date.format("%H:%M:%S"));

    let font = FontRef::try_from_slice(include_bytes!("../../ui/assets/DejaVuSans.ttf"))
        .map_err(|e| std::io::Error::other(e))?;

    let x = img.width().saturating_sub(200);
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
        .map_err(std::io::Error::other)?;
    Ok(())
}

pub enum MonitoringStateMessage {
    // FrameTaken,
    Stopped,
    FinishedSavingSubmission,
    // FinishedCompilingVideoData,
}

fn run_monitoring(
    exam_data_dir: &PathBuf,
    status_tx: Sender<MonitoringStateMessage>,
    terminate_rx: Receiver<PathBuf>,
    output_path: &PathBuf,
) -> std::io::Result<()> {
    let captures_save_dir = exam_data_dir.join("captures");
    std::fs::create_dir_all(&captures_save_dir)?;

    let submission_dir = loop {
        // stop monitoring when message is received over rx.
        // the message also contains a user submitted pathbuf to their
        // exam submission.
        match terminate_rx.try_recv() {
            Ok(data) => {
                break data;
            }
            Err(TryRecvError::Disconnected) => {
                return Err(std::io::Error::other("Termination channel crashed"));
            }
            Err(TryRecvError::Empty) => {}
        }
        take_screenshot(&captures_save_dir)?;

        // arbitrary delay
        thread::sleep(Duration::from_secs(2));
    };

    println!("stopped monitoring. attempting to zip exam data (screen capture + submission).");
    status_tx
        .send(MonitoringStateMessage::Stopped)
        .map_err(std::io::Error::other)?;

    // copy exam submission directory
    let new_submission_dir = exam_data_dir.join("submission");
    if new_submission_dir.exists() {
        std::fs::remove_dir_all(&new_submission_dir)?;
    }
    clone_tree(
        submission_dir,
        new_submission_dir,
        &clonetree::Options::default(),
    )
    .map_err(std::io::Error::other)?;

    status_tx
        .send(MonitoringStateMessage::FinishedSavingSubmission)
        .map_err(std::io::Error::other)?;

    zip_dir(
        &exam_data_dir,
        &output_path,
        zip::CompressionMethod::default(),
    )
    .or(Err(std::io::Error::other("Couldn't zip dir.")))?;

    println!("finished creating exam data ZIP.");
    Ok(())
}

pub fn start_monitoring_thread(
    exam_data_dir: PathBuf,
    output_path: PathBuf,
) -> (
    JoinHandle<std::io::Result<()>>,
    Sender<PathBuf>,
    Receiver<MonitoringStateMessage>,
) {
    let (terminate_tx, terminate_rx) = mpsc::channel();
    let (status_tx, status_rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        run_monitoring(&exam_data_dir, status_tx, terminate_rx, &output_path)
    });
    (handle, terminate_tx, status_rx)
}
