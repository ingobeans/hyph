use std::{
    collections::{HashMap, hash_map::Entry},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread::{self, JoinHandle},
    time::Duration,
};

use ab_glyph::FontRef;
use chrono::Local;
use clonetree::clone_tree;
use image::{DynamicImage, Rgba, RgbaImage};
use list_windows::*;
use walkdir::WalkDir;
use win_screenshot::prelude::*;
use zip::{result::ZipError, write::SimpleFileOptions};

use crate::utils::{OpenedWindows, serialize_opened_windows};

mod list_windows;

/// Take a screenshot every n seconds
pub const SCREENSHOT_INTERVAL: u8 = 2;

pub struct MonitoringThread {
    pub join_handle: JoinHandle<std::io::Result<()>>,
    pub terminate_tx: Sender<PathBuf>,
    pub status_rx: Receiver<MonitoringStateMessage>,
}
impl MonitoringThread {
    pub fn new(
        join_handle: JoinHandle<std::io::Result<()>>,
        terminate_tx: Sender<PathBuf>,
        status_rx: Receiver<MonitoringStateMessage>,
    ) -> Self {
        Self {
            join_handle,
            terminate_tx,
            status_rx,
        }
    }
}

#[derive(Debug)]
pub enum TakeScreenshotError {
    CaptureDisplayFail,
    ImageParsingFail,
    FontLoadingFail,
    #[expect(dead_code)]
    ImageError(image::ImageError),
}

fn take_screenshot(save_path: &Path, font: &FontRef) -> Result<(), TakeScreenshotError> {
    let buf = capture_display().or(Err(TakeScreenshotError::CaptureDisplayFail))?;

    let mut img = DynamicImage::ImageRgba8(
        RgbaImage::from_raw(buf.width, buf.height, buf.pixels)
            .ok_or(TakeScreenshotError::ImageParsingFail)?,
    );

    let date = Local::now();
    let unix_time = date.timestamp();
    let time_text = format!("{}", date.format("%H:%M:%S"));

    let x = img.width().saturating_sub(200);
    imageproc::drawing::draw_text_mut(
        &mut img,
        Rgba([255, 255, 255, 255]),
        x as i32,
        5,
        51.0,
        font,
        &time_text,
    );
    imageproc::drawing::draw_text_mut(
        &mut img,
        Rgba([0, 0, 0, 255]),
        x as i32,
        50,
        24.0,
        font,
        &time_text,
    );

    img.to_rgb8()
        .save(save_path.join(format!("{unix_time}.jpg")))
        .map_err(TakeScreenshotError::ImageError)?;
    Ok(())
}

#[derive(Debug)]
pub enum MonitoringLoggingMessage {
    ScreenshotError(TakeScreenshotError),
    ListProgramsError,
}
pub enum MonitoringStateMessage {
    Stopped,
    FinishedSavingSubmission,
    // FinishedCompilingVideoData,
}

fn run_monitoring(
    exam_data_dir: &Path,
    status_tx: Sender<MonitoringStateMessage>,
    logging_tx: Sender<MonitoringLoggingMessage>,
    terminate_rx: Receiver<PathBuf>,
    output_path: &Path,
) -> std::io::Result<()> {
    let captures_save_dir = exam_data_dir.join("captures");
    std::fs::create_dir_all(&captures_save_dir)?;

    let Ok(font) = FontRef::try_from_slice(include_bytes!("../../ui/assets/DejaVuSans.ttf")) else {
        let _ = logging_tx.send(MonitoringLoggingMessage::ScreenshotError(
            TakeScreenshotError::FontLoadingFail,
        ));
        return Err(std::io::Error::other("Font loading fail"));
    };

    let mut opened_windows: OpenedWindows = HashMap::new();
    let mut screenshot_counter: u8 = 0;

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

        // track open programs
        match get_open_windows() {
            Some(windows) => {
                for window in windows.into_iter() {
                    let date = Local::now();
                    let unix_time = date.timestamp();
                    match opened_windows.entry(window) {
                        Entry::Occupied(mut entry) => {
                            entry.get_mut().push(unix_time);
                        }
                        Entry::Vacant(entry) => {
                            entry.insert(vec![unix_time]);
                        }
                    }
                }
            }
            None => {
                logging_tx
                    .send(MonitoringLoggingMessage::ListProgramsError)
                    .map_err(std::io::Error::other)?;
            }
        }

        // take screenshot if screenshot_counter reaches 0
        screenshot_counter = screenshot_counter.saturating_sub(1);
        if screenshot_counter == 0 {
            screenshot_counter = SCREENSHOT_INTERVAL;
            let result = take_screenshot(&captures_save_dir, &font);

            // if error sending screenshot, dont crash thread,
            // rather send it over logging channel
            if let Err(err) = result {
                logging_tx
                    .send(MonitoringLoggingMessage::ScreenshotError(err))
                    .map_err(std::io::Error::other)?;
            }
        }

        // arbitrary delay
        thread::sleep(Duration::from_secs(1));
    };

    println!("stopped monitoring. attempting to zip exam data (screen capture + submission).");

    status_tx
        .send(MonitoringStateMessage::Stopped)
        .map_err(std::io::Error::other)?;

    // write opened windows data to file
    let serialized_window_data = serialize_opened_windows(opened_windows);
    std::fs::write(
        exam_data_dir.join("opened_windows.txt"),
        serialized_window_data,
    )?;

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
        exam_data_dir,
        output_path,
        zip::CompressionMethod::default(),
    )
    .or(Err(std::io::Error::other("Couldn't zip dir.")))?;

    println!("finished creating exam data ZIP.");
    Ok(())
}

pub fn start_monitoring_thread(
    exam_data_dir: PathBuf,
    output_path: PathBuf,
) -> (MonitoringThread, Receiver<MonitoringLoggingMessage>) {
    let (terminate_tx, terminate_rx) = mpsc::channel();
    let (status_tx, status_rx) = mpsc::channel();
    let (logging_tx, logging_rx) = mpsc::channel();

    let handle = thread::spawn(move || {
        run_monitoring(
            &exam_data_dir,
            status_tx,
            logging_tx,
            terminate_rx,
            &output_path,
        )
    });
    (
        MonitoringThread::new(handle, terminate_tx, status_rx),
        logging_rx,
    )
}

// joinked directly from the zip crate examples: https://github.com/zip-rs/zip2/blob/master/examples/write_dir.rs#L78
pub fn zip_dir(
    src_dir: &Path,
    dest_file: &Path,
    method: zip::CompressionMethod,
) -> Result<(), Box<dyn std::error::Error>> {
    if !Path::new(src_dir).is_dir() {
        return Err(ZipError::FileNotFound.into());
    }

    let file = std::fs::File::create(dest_file)?;

    let walkdir = WalkDir::new(src_dir);

    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(method)
        .unix_permissions(0o755);

    for entry_result in walkdir.into_iter() {
        let entry = match entry_result {
            Ok(entry) => entry,
            Err(e) => {
                return Err(format!("Error while traversing directory {src_dir:?}: {e}").into());
            }
        };
        let path = entry.path();
        let path_stripped = path.strip_prefix(src_dir)?;
        let path_as_string = path_stripped
            .to_str()
            .map(str::to_owned)
            .ok_or_else(|| format!("{:?} is a Non UTF-8 Path", path_stripped.display()))?;

        if path.is_file() {
            zip.start_file(path_as_string, options)?;
            let mut f = std::fs::File::open(path)?;

            std::io::copy(&mut f, &mut zip)?;
        } else if !path_stripped.as_os_str().is_empty() {
            zip.add_directory(path_as_string, options)?;
        }
    }
    zip.finish()?;
    Ok(())
}
