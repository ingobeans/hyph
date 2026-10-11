#![allow(clippy::await_holding_lock)]

use std::{
    path::PathBuf,
    rc::Rc,
    sync::{Mutex, mpsc::TryRecvError},
    time::Duration,
};

use chrono::Local;
use rfd::FileDialog;
use slint::ToSharedString;
use tokio::time::sleep;

use crate::{backend::*, monitoring::*, utils::*};

#[path = "../utils.rs"]
mod utils;

#[path = "../client/backend.rs"]
mod backend;
#[path = "../client/monitoring.rs"]
mod monitoring;

slint::include_modules!();
fn main() -> std::io::Result<()> {
    let name = whoami::realname()?;
    let client = Client::new(String::new());
    let client_rc = Rc::new(Mutex::new(client));

    let app = App::new().unwrap();

    app.set_username(format!("Logged in as: {name}").to_shared_string());

    let app_handle = Rc::new(app.as_weak());

    let handle = app_handle.clone();
    app.on_close_program(move || {
        if let Some(handle) = handle.upgrade() {
            handle.window().hide().unwrap();
        }
    });

    let handle = app_handle.clone();
    app.window().on_close_requested(move || {
        let exam_active = if let Some(handle) = handle.upgrade() {
            !handle.get_start_exam_enabled()
        } else {
            false
        };
        if exam_active {
            slint::CloseRequestResponse::KeepWindowShown
        } else {
            slint::CloseRequestResponse::HideWindow
        }
    });

    let handle = app_handle.clone();
    app.on_mouse_move(move |delta_x, delta_y| {
        let Some(pin_win_clone) = handle.upgrade() else {
            return;
        };
        let logical_pos = pin_win_clone
            .window()
            .position()
            .to_logical(pin_win_clone.window().scale_factor());
        pin_win_clone
            .window()
            .set_position(slint::LogicalPosition::new(
                logical_pos.x + delta_x,
                logical_pos.y + delta_y,
            ));
    });

    let c = client_rc.clone();
    let handle = app_handle.clone();
    app.on_ping_click(move || {
        let c = c.clone();
        let handle = handle.clone();
        let mut addr = String::new();
        if let Some(handle) = handle.upgrade() {
            handle.set_ping_enabled(false);
            handle.set_ping_success(false);
            handle.set_ping_fail(false);
            handle.set_start_exam_success(false);
            handle.set_start_exam_fail(false);

            let mut c = c.lock().unwrap();
            addr = format!("{}:{}", handle.get_ip_address(), PORT);
            c.addr = addr.clone();
        }

        let future = async move {
            let state = {
                let c = c.lock().unwrap();
                c.ping_server().await.is_ok()
            };

            if let Some(handle) = handle.upgrade() {
                handle.set_ping_enabled(true);
                handle.set_ping_success(state);
                handle.set_ping_fail(!state);

                if state {
                    println!("Connected to: {}", addr)
                };
            }
        };
        slint::spawn_local(async_compat::Compat::new(future)).unwrap();
    });

    let c = client_rc.clone();
    let handle = app_handle.clone();
    app.on_start_exam_click(move || {
        let c = c.clone();
        let handle = handle.clone();
        let name = name.clone();
        if let Some(handle) = handle.upgrade() {
            handle.set_ping_enabled(false);
            handle.set_start_exam_enabled(false);
            handle.set_start_exam_success(false);
            handle.set_start_exam_fail(false);
        }

        let future = async move {
            let state = {
                let mut c = c.lock().unwrap();
                let state = c.register(&name).await.is_ok_and(|f| f);
                if state {
                    let (thread, logging_rx) =
                        start_monitoring_thread("exam_data/".into(), "output.bin".into());
                    c.monitoring_thread = MonitoringState::Active(thread);
                    Some(logging_rx)
                } else {
                    None
                }
            };

            if let Some(handle) = handle.upgrade() {
                handle.set_start_exam_success(state.is_some());
                handle.set_start_exam_fail(state.is_none());
                if state.is_some() {
                    handle.set_exam_active(true);
                    println!("Started exam.");
                } else {
                    handle.set_ping_enabled(true);
                    handle.set_start_exam_enabled(true);
                }
            }

            // listen for (error) messages over logging_rx.

            if let Some(logging_rx) = state {
                loop {
                    match logging_rx.try_recv() {
                        Ok(message) => {
                            let message = match message {
                                MonitoringLoggingMessage::ScreenshotError(err) => {
                                    let date = Local::now();
                                    let text: String = format!("{}", date.format("%H:%M:%S"));
                                    format!(
                                        "Error encountered while monitoring exam: {err:?} ({text})"
                                    )
                                    .to_shared_string()
                                }
                                MonitoringLoggingMessage::ListProgramsError => {
                                    "Error encountered while monitoring exam".to_shared_string()
                                }
                            };

                            if let Some(handle) = handle.upgrade() {
                                handle.set_monitoring_logging(message);
                            }
                        }
                        Err(TryRecvError::Disconnected) => {
                            break;
                        }
                        Err(TryRecvError::Empty) => {}
                    }
                    sleep(Duration::from_millis(100)).await;
                }
            }
        };
        slint::spawn_local(async_compat::Compat::new(future)).unwrap();
    });

    let c = client_rc.clone();
    let handle = app_handle.clone();
    app.on_submit_exam_click(move || {
        let c = c.clone();

        // if this is first submission, open file picker dialog.
        // else, if user is retrying after a failed submission, get None.
        let folder = {
            let previous_submission_failed =
                { matches!(c.lock().unwrap().monitoring_thread, MonitoringState::Ended) };
            if previous_submission_failed {
                None
            } else {
                let Some(folder) = FileDialog::new().pick_folder() else {
                    // if user cancels, just return.
                    return;
                };
                Some(folder)
            }
        };

        let handle = handle.clone();
        if let Some(handle) = handle.upgrade() {
            handle.set_ping_enabled(false);
            handle.set_exam_stopped(true);
            handle.set_submit_exam_success(false);
            handle.set_submit_exam_fail(false);
            handle.set_submit_exam_status("Stopping exam...".to_shared_string());
        }

        let future = submit_exam(c, folder, handle);
        slint::spawn_local(async_compat::Compat::new(future)).unwrap();
    });

    app.run().unwrap();
    Ok(())
}

async fn submit_exam(c: Rc<Mutex<Client>>, folder: Option<PathBuf>, handle: Rc<slint::Weak<App>>) {
    let state = {
        let mut c = c.lock().unwrap();
        match c.monitoring_thread.take() {
            MonitoringState::ExamNotStarted => {
                // todo: report error
                false
            }
            MonitoringState::Ended => {
                let join_handle = retry_failed_submission("exam_data/".into(), "output.bin".into());
                if let Some(handle) = handle.upgrade() {
                    handle.set_submit_exam_status("Retrying exam submission...".to_shared_string());
                }
                loop {
                    sleep(Duration::from_millis(500)).await;
                    if join_handle.is_finished() {
                        break;
                    }
                }
                match join_handle.join().unwrap() {
                    Ok(_) => c.send_file_to_server("output.bin").await.is_ok(),
                    Err(err) => {
                        if let Some(handle) = handle.upgrade() {
                            handle.set_submit_exam_status(
                                format!("Failed resubmitting exam: {err:?}").to_shared_string(),
                            );
                        }
                        false
                    }
                }
            }
            MonitoringState::Active(monitoring_thread) => {
                'handle_thread_stop: {
                    // check if thread has already died from an error
                    if monitoring_thread.join_handle.is_finished() {
                        break 'handle_thread_stop;
                    }

                    // unwrap is safe because folder can only be none if monitoring state is Ended.
                    let folder = folder.clone().unwrap();

                    // send over tx to tell thread to terminate,
                    // and check for error
                    if let Err(e) = monitoring_thread.terminate_tx.send(folder) {
                        println!("error sending termination to monitoring thread: {e}");
                        if let Some(handle) = handle.upgrade() {
                            handle.set_submit_exam_status(
                                format!("Error ending exam: {e}").to_shared_string(),
                            );
                        }
                        break 'handle_thread_stop;
                    }

                    // wait for thread to completely finish
                    loop {
                        print!(".");
                        sleep(Duration::from_millis(500)).await;

                        // meanwhile, also check for status update messages
                        if let Ok(message) = monitoring_thread.status_rx.try_recv() {
                            let message = match message {
                                MonitoringStateMessage::Stopped => Some("Saving submission..."),
                                MonitoringStateMessage::FinishedSavingSubmission => {
                                    Some("Compressing exam data...")
                                }
                            };
                            if let Some(message) = message
                                && let Some(handle) = handle.upgrade()
                            {
                                handle.set_submit_exam_status(message.to_shared_string())
                            }
                        }

                        if monitoring_thread.join_handle.is_finished() {
                            break 'handle_thread_stop;
                        }
                    }
                }

                // check that the thread exited sucessfully

                // unwrap is safe since we only break if join handle is finished
                let result = monitoring_thread.join_handle.join().unwrap();

                if let Some(handle) = handle.upgrade() {
                    if let Err(err) = result {
                        handle.set_submit_exam_success(false);
                        handle.set_submit_exam_fail(true);
                        handle.set_exam_stopped(false);
                        handle.set_submit_exam_status(
                            format!("Error saving exam data: {err:?}").to_shared_string(),
                        );
                        return;
                    }

                    handle.set_submit_exam_status(
                        "Transmitting exam data to server...".to_shared_string(),
                    );
                }
                c.send_file_to_server("output.bin").await.is_ok()
            }
        }
    };

    if let Some(handle) = handle.upgrade() {
        handle.set_submit_exam_success(state);
        handle.set_submit_exam_fail(!state);
        if state {
            handle.set_submit_exam_status(
                "Exam submitted sucessfully! You can now close this window.".to_shared_string(),
            );
            handle.set_exam_active(false);
            println!("Sucessfully submitted exam.");
        } else {
            handle.set_submit_exam_status(
                "Failed to upload exam. Backing up exam submission...".to_shared_string(),
            );

            // immediately back up submission to prevent tampering
            if let Some(folder) = folder
                && let Err(err) = backup_failed_submission("exam_data/".into(), folder).await
            {
                handle.set_submit_exam_status(
                    format!("Failed to upload exam. Failed to back up exam submission. {err:?}")
                        .to_shared_string(),
                );
                // if backing up exam submission fails, return so that set_exam_stopped is never called,
                // so that the user can't press the submit button again
                return;
            }
            handle.set_submit_exam_status("Failed to upload exam. Retry?".to_shared_string());
            handle.set_exam_stopped(false);
        }
    }
}
