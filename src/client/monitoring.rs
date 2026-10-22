use std::{
    collections::{HashMap, hash_map::Entry},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use ab_glyph::FontRef;
use chrono::Local;
use clonetree::clone_tree;
use image::{DynamicImage, Rgba, RgbaImage};
use list_windows::*;
use ripzip::{NoProgress, compress_directory};
use win_screenshot::prelude::*;

use crate::utils::*;

mod list_windows;

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
    let mut opened_last: Vec<String> = Vec::new();

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
                let date = Local::now();
                let unix_time = date.timestamp();
                for window in windows.iter() {
                    if !opened_last.contains(&window) {
                        // window was just opened, add an entry for it
                        match opened_windows.entry(window.clone()) {
                            Entry::Occupied(mut entry) => {
                                entry.get_mut().push(WindowOpenTime {
                                    opened: unix_time,
                                    closed: 0,
                                });
                            }
                            Entry::Vacant(entry) => {
                                entry.insert(vec![WindowOpenTime {
                                    opened: unix_time,
                                    closed: 0,
                                }]);
                            }
                        }
                    }
                }
                for window in opened_last.into_iter() {
                    if !windows.contains(&window) {
                        // window was closed. update last entry of the window with the close time

                        if let Some(entry) = opened_windows.get_mut(&window)
                            && let Some(entry) = entry.last_mut()
                        {
                            entry.closed = unix_time;
                        }
                    }
                }
                opened_last = windows;
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

    println!("stopped monitoring. writing exam user data.");

    let name =
        whoami::realname().unwrap_or(whoami::username().unwrap_or("Unknown Name".to_string()));
    std::fs::write(exam_data_dir.join("info.txt"), format!("{name}\n"))?;

    println!("attempting to zip exam data (screen capture + submission).");

    status_tx
        .send(MonitoringStateMessage::Stopped)
        .map_err(std::io::Error::other)?;

    let date = Local::now();
    let unix_time = date.timestamp();

    // replace all windows that are unclosed's close time as current time
    for (_k, v) in opened_windows.iter_mut() {
        for t in v {
            if t.closed == 0 {
                t.closed = unix_time;
            }
        }
    }

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

    let now = Instant::now();
    compress_directory(
        exam_data_dir,
        output_path,
        1,
        ripzip::CompressionMethod::Deflate,
        &NoProgress,
    )
    .or(Err(std::io::Error::other("Couldn't zip dir.")))?;
    let after = Instant::now();
    println!(
        "zipping took: {}",
        after.duration_since(now).as_millis() as f32 / 1000.0
    );

    println!("Finished creating exam data ZIP.");
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

#[derive(Debug)]
pub enum RetrySubmissionError {
    /// There's nothing that can be done. Exam is fully corrupted
    CriticalFail,
    #[allow(dead_code)]
    IoError(std::io::Error),
    ZipFail,
}

pub fn retry_failed_submission(
    exam_data_dir: PathBuf,
    output_path: PathBuf,
) -> JoinHandle<Result<(), RetrySubmissionError>> {
    thread::spawn(move || {
        if !exam_data_dir.exists() {
            Err(RetrySubmissionError::CriticalFail)?;
        }

        let new_submission_dir = exam_data_dir.join("submission");
        if !new_submission_dir.exists() {
            Err(RetrySubmissionError::CriticalFail)?;
        }

        compress_directory(
            &exam_data_dir,
            &output_path,
            1,
            ripzip::CompressionMethod::Deflate,
            &NoProgress,
        )
        .or(Err(RetrySubmissionError::ZipFail))?;

        Ok(())
    })
}

pub async fn backup_failed_submission(
    exam_data_dir: PathBuf,
    submission_dir: PathBuf,
) -> Result<(), RetrySubmissionError> {
    if !exam_data_dir.exists() {
        Err(RetrySubmissionError::CriticalFail)?;
    }

    let new_submission_dir = exam_data_dir.join("submission");
    if new_submission_dir.exists() {
        tokio::fs::remove_dir_all(&new_submission_dir)
            .await
            .map_err(RetrySubmissionError::IoError)?;
    }

    // don't use clonetree for safety, just in case it was the issue,
    // instead retry with a more simple/robust method of cloning a tree (which also happens to be async)
    async_copy_dir_slow(submission_dir, new_submission_dir)
        .await
        .map_err(RetrySubmissionError::IoError)?;
    Ok(())
}

/// Copies a directory recursively.
/// Only used if clonetree failed
///
/// (clonetree is much faster since it does filesystem tomfoolery to copy the directory, while this actually copies every file making it more robust)
async fn async_copy_dir_slow(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    tokio::fs::create_dir_all(&dst).await?;
    let mut iterator = tokio::fs::read_dir(src).await?;
    while let Some(entry) = iterator.next_entry().await? {
        let ty = entry.file_type().await?;
        if ty.is_dir() {
            Box::pin(async_copy_dir_slow(
                entry.path(),
                dst.as_ref().join(entry.file_name()),
            ))
            .await?;
        } else {
            tokio::fs::copy(entry.path(), dst.as_ref().join(entry.file_name())).await?;
        }
    }
    Ok(())
}
