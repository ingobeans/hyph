use std::{rc::Rc, sync::Mutex, time::Duration};

use rfd::FileDialog;
use slint::ToSharedString;
use tokio::time::sleep;

use crate::{backend::*, screencapture::*, utils::*};

#[path = "../utils.rs"]
mod utils;

#[path = "../client/backend.rs"]
mod backend;
#[path = "../client/screencapture.rs"]
mod screencapture;

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
                    let thread =
                        start_screencapture_thread("exam_data/".into(), "output.bin".into());
                    c.screen_capture_thread = Some(thread);
                }
                state
            };

            if let Some(handle) = handle.upgrade() {
                handle.set_start_exam_success(state);
                handle.set_start_exam_fail(!state);
                if state {
                    handle.set_exam_active(true);
                    println!("Started exam.");
                } else {
                    handle.set_ping_enabled(true);
                    handle.set_start_exam_enabled(true);
                }
            }
        };
        slint::spawn_local(async_compat::Compat::new(future)).unwrap();
    });

    let c = client_rc.clone();
    let handle = app_handle.clone();
    app.on_submit_exam_click(move || {
        let Some(folder) = FileDialog::new().pick_folder() else {
            return;
        };

        let c = c.clone();
        let handle = handle.clone();
        if let Some(handle) = handle.upgrade() {
            handle.set_ping_enabled(false);
            handle.set_exam_stopped(true);
            handle.set_submit_exam_success(false);
            handle.set_submit_exam_fail(false);
            handle.set_submit_exam_status("Stopping exam...".to_shared_string());
        }

        let future = async move {
            let state = {
                let c = c.lock().unwrap();
                if let Some((join_handle, tx, rx)) = &c.screen_capture_thread {
                    // send over tx to tell thread to terminate
                    let _ = tx.send(folder.clone());

                    // wait for thread to completely finish
                    loop {
                        print!(".");
                        sleep(Duration::from_millis(500)).await;

                        // meanwhile, also check for status update messages
                        if let Ok(message) = rx.try_recv() {
                            let message = match message {
                                ScreenCaptureStateMessage::Stopped => Some("Saving submission..."),
                                ScreenCaptureStateMessage::FinishedSavingSubmission => {
                                    Some("Compressing exam data...")
                                }
                                _ => None,
                            };
                            if let Some(message) = message
                                && let Some(handle) = handle.upgrade()
                            {
                                println!("{message}");
                                handle.set_submit_exam_status(message.to_shared_string())
                            }
                        }

                        if join_handle.is_finished() {
                            break;
                        }
                    }
                    if let Some(handle) = handle.upgrade() {
                        handle.set_submit_exam_status(
                            "Transmitting exam data to server...".to_shared_string(),
                        );
                    }
                    c.send_file_to_server("output.bin").await.is_ok()
                } else {
                    false
                }
            };

            if let Some(handle) = handle.upgrade() {
                handle.set_submit_exam_success(state);
                handle.set_submit_exam_fail(!state);
                if state {
                    handle.set_submit_exam_status(
                        "Exam submitted sucessfully! You can now close this window."
                            .to_shared_string(),
                    );
                    handle.set_exam_active(false);
                    println!("Sucessfully submitted exam.");
                } else {
                    handle.set_submit_exam_status("Failed to upload exam.".to_shared_string());
                    handle.set_exam_stopped(false);
                }
            }
        };
        slint::spawn_local(async_compat::Compat::new(future)).unwrap();
    });

    app.run().unwrap();
    Ok(())
}
