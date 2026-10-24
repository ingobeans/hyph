use std::{
    path::{Path, PathBuf},
    rc::Rc,
    sync::Mutex,
};

use chrono::Local;
use image::load_from_memory_with_format;
use slint::{Model, SharedString, ToSharedString, VecModel, Weak};

use crate::{backend::*, utils::*};

#[path = "../utils.rs"]
mod utils;

#[path = "../server/backend.rs"]
mod backend;

impl Exam {
    fn null() -> Self {
        Exam {
            date: SharedString::new(),
            id: 0, // id 0 denotes null
            name: SharedString::new(),
            submissions: 0,
            ended: false,
        }
    }
}

fn load_previous_exams(base: &Path) -> Vec<Exam> {
    let mut previous_exams = Vec::new();
    for item in std::fs::read_dir(base).unwrap().flatten() {
        if item.file_type().unwrap().is_file() {
            continue;
        }

        let Ok(id): Result<u64, _> = item.file_name().to_string_lossy().parse() else {
            continue;
        };
        let id = id.cast_signed();

        let path = item.path();
        let info_path = path.join("info.txt");
        let Ok(info) = std::fs::read_to_string(info_path) else {
            continue;
        };

        let submissions_path = path.join("submissions");
        let Ok(count) = std::fs::read_dir(submissions_path).map(|f| f.count()) else {
            continue;
        };

        let mut info_rows = info.split("\n");
        let Some(name) = info_rows.next() else {
            continue;
        };
        let Some(date) = info_rows.next() else {
            continue;
        };

        let exam = Exam {
            id,
            date: date.to_shared_string(),
            name: name.to_shared_string(),
            submissions: count as i32,
            ended: true,
        };
        previous_exams.push(exam);
    }
    previous_exams
}

async fn load_exam_submissions(exam: &Exam, base: &Path) -> Vec<RegisteredUser> {
    let mut users = Vec::new();

    let submissions_path = base
        .join(exam.id.cast_unsigned().to_string())
        .join("submissions");

    let mut iterator = tokio::fs::read_dir(submissions_path).await.unwrap();

    while let Some(submission) = iterator.next_entry().await.unwrap() {
        if submission.file_type().await.unwrap().is_file() {
            continue;
        }

        let Ok(id) = submission.file_name().to_string_lossy().parse::<u64>() else {
            continue;
        };

        let path = submission.path();

        let mut name = "Unknown".to_string();
        'get_info: {
            let info_path = path.join("info.txt");
            let Ok(info) = tokio::fs::read_to_string(info_path).await else {
                break 'get_info;
            };
            let mut info_rows = info.split("\n");

            if let Some(n) = info_rows.next() {
                name = n.to_string();
            };
        }

        let user = RegisteredUser {
            id: id.cast_signed(),
            name: name.to_shared_string(),
            state: UserState::FinishedExam,
            cheating_detected: false,
        };
        users.push(user);
    }
    users
}

slint::include_modules!();
fn main() {
    println!("Hyph Server v{}\n", env!("CARGO_PKG_VERSION"));
    let exams_base = PathBuf::from("exams/");
    let previous_exams = load_previous_exams(&exams_base);

    let app = ServerApp::new().unwrap();

    app.set_previous_exams(VecModel::from_slice(&previous_exams));

    let previous_exams_rc = Rc::new(previous_exams);
    let app_handle = Rc::new(app.as_weak());

    let server_backend = Rc::new(Mutex::new(None));
    let listener_terminate = Rc::new(Mutex::new(None));
    let listener_logger = Rc::new(Mutex::new(None));

    let server_backend_rc = server_backend.clone();
    let listener_terminate_rc = listener_terminate.clone();
    let listener_logger_rc = listener_logger.clone();

    let future = async move {
        let Ok((server, logging_rx, terminate_tx)) =
            Server::start(format!("0.0.0.0:{}", PORT)).await
        else {
            panic!("Another server instance is already running.");
        };

        *server_backend_rc.lock().unwrap() = Some(server);
        *listener_terminate_rc.lock().unwrap() = Some(terminate_tx);
        *listener_logger_rc.lock().unwrap() = Some(logging_rx);
    };

    slint::spawn_local(async_compat::Compat::new(future)).unwrap();

    let handle = app_handle.clone();
    app.on_close_program(move || {
        if let Some(handle) = handle.upgrade() {
            handle.window().hide().unwrap();
        }
    });
    let handle = app_handle.clone();
    app.on_minimize_program(move || {
        if let Some(handle) = handle.upgrade() {
            handle.window().set_minimized(true);
        }
    });
    let handle = app_handle.clone();
    app.on_maximize_program(move || {
        if let Some(handle) = handle.upgrade() {
            handle
                .window()
                .set_maximized(!handle.window().is_maximized());
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

    let handle = app_handle.clone();
    let exams_base_clone = exams_base.clone();
    app.on_inspect_submission(move |user, exam| {
        let exams_base = exams_base_clone.clone();
        async fn inspect_exam(
            user: RegisteredUser,
            exam: Exam,
            handle: Rc<Weak<ServerApp>>,
            base: PathBuf,
        ) {
            let Some(handle) = handle.clone().upgrade() else {
                return;
            };
            let user_id = user.id.cast_unsigned();

            let exam_dir = base.join(exam.id.cast_unsigned().to_string());
            let submissions_dir = exam_dir.join("submissions");
            let submission_dir = submissions_dir.join(user_id.to_string());

            if !submission_dir.exists() {
                eprintln!("Exam submission directory '{submission_dir:?}' doesn't exist");
                return;
            }

            let opened_windows_text =
                tokio::fs::read_to_string(submission_dir.join("opened_windows.txt"))
                    .await
                    .unwrap_or_else(|_| {
                        eprintln!("opened_windows.txt doesn't exist for user {user_id}");
                        String::new()
                    });
            let lines = opened_windows_text.lines();
            let mut entries = Vec::new();
            for line in lines {
                let Some((name, times)) = line.rsplit_once(":") else {
                    continue;
                };
                let mut bars: Vec<TimelineBar> = Vec::new();
                let mut bars_time: Vec<(u64, u64)> = Vec::new();

                let mut lowest = u64::MAX;
                let mut highest = 0;
                for bar in times.split(",") {
                    let Some((start, end)) = bar.split_once("-") else {
                        continue;
                    };
                    let Ok(start) = start.parse() else {
                        continue;
                    };
                    let Ok(end) = end.parse() else {
                        continue;
                    };
                    if end > highest {
                        highest = end;
                    }
                    if start < lowest {
                        lowest = start;
                    }
                    bars_time.push((start, end));
                }
                for bar in &mut bars_time {
                    let start = (bar.0 - lowest) as f32 / (highest - lowest) as f32;
                    let end = (bar.1 - lowest) as f32 / (highest - lowest) as f32;
                    bars.push(TimelineBar { start, end });
                }

                entries.push(TimelineEntry {
                    bars: VecModel::from_slice(&bars),
                    name: name.to_shared_string(),
                });
            }

            handle.set_inspected_submission(InspectedSubmission {
                exam_length: 0.0,
                user,
                timeline_entries: VecModel::from_slice(&entries),
            });

            let captures_path = submission_dir.join("captures");
            let read_dir = tokio::fs::read_dir(&captures_path).await;
            if let Ok(mut read_dir) = read_dir {
                let mut frame = Default::default();
                for _ in 0..handle.get_exam_screenshot_frame() + 1 {
                    let Ok(Some(r)) = read_dir.next_entry().await else {
                        break;
                    };
                    frame = r.file_name().to_string_lossy().to_shared_string();
                }
                println!("{frame}");
                if let Ok(data) = tokio::fs::read(&captures_path.join(frame)).await {
                    let result = tokio::task::spawn_blocking(move || {
                        if let Ok(dynamic_image) =
                            load_from_memory_with_format(&data, image::ImageFormat::Jpeg)
                        {
                            let rgb = dynamic_image.to_rgb8();
                            let buffer = slint::SharedPixelBuffer::clone_from_slice(
                                rgb.as_raw(),
                                rgb.width(),
                                rgb.height(),
                            );
                            Some(buffer)
                        } else {
                            None
                        }
                    })
                    .await;

                    if let Ok(Some(buffer)) = result {
                        let image = slint::Image::from_rgb8(buffer);
                        handle.set_exam_screenshot_image(image);
                    }
                }
            }
        }

        slint::spawn_local(async_compat::Compat::new(inspect_exam(
            user,
            exam,
            handle.clone(),
            exams_base,
        )))
        .unwrap();
    });

    let handle = app_handle.clone();
    let previous_exams = previous_exams_rc.clone();
    let exams_base_clone = exams_base.clone();
    app.on_show_previous_exam(move |id| {
        let previous_exams = previous_exams.clone();
        let handle = handle.clone();
        let exams_base = exams_base_clone.clone();
        let future = async move {
            if let Some(handle) = handle.upgrade() {
                let Some(exam) = previous_exams.iter().find(|a| a.id == id) else {
                    return;
                };
                let submissions = load_exam_submissions(exam, &exams_base).await;
                handle.set_registered_users(VecModel::from_slice(&submissions));
                handle.set_active_exam(exam.clone());
            }
        };
        slint::spawn_local(async_compat::Compat::new(future)).unwrap();
    });

    let handle = app_handle.clone();
    let exams_base_clone = exams_base.clone();
    app.on_new_exam(move |name| {
        let handle = handle.clone();
        let exams_base = exams_base_clone.clone();
        let future = async move {
            if let Some(handle) = handle.upgrade() {
                let date = Local::now();
                let date_text: String = format!("{}", date.format("%d/%m %Y"));
                let id: i64 = rand::random();

                let exam_path = exams_base.join(id.cast_unsigned().to_string());

                tokio::fs::create_dir_all(exam_path.join("submissions"))
                    .await
                    .unwrap();
                tokio::fs::create_dir_all(exam_path.join("received"))
                    .await
                    .unwrap();

                let info = format!("{name}\n{date_text}");
                tokio::fs::write(exam_path.join("info.txt"), info)
                    .await
                    .unwrap();

                let exam = Exam {
                    date: date_text.to_shared_string(),
                    ended: true,
                    id,
                    name,
                    submissions: 0,
                };

                handle.set_registered_users(VecModel::from_slice(&[]));
                handle.set_active_exam(exam);
            }
        };
        slint::spawn_local(async_compat::Compat::new(future)).unwrap();
    });

    let listener_terminate_rc = listener_terminate.clone();
    app.on_stop_exam(move || {
        let listener_terminate = listener_terminate_rc.clone();

        let future = async move {
            let result = listener_terminate
                .lock()
                .unwrap()
                .as_mut()
                .unwrap()
                .send(())
                .await;
            if let Err(e) = result {
                eprintln!("error stopping exam: {e:?}");
            }
        };

        slint::spawn_local(async_compat::Compat::new(future)).unwrap();
    });

    let handle = app_handle.clone();
    let exams_base_clone = exams_base.clone();
    let server_backend_rc = server_backend.clone();
    let listener_logger_rc = listener_logger.clone();
    app.on_start_exam(move |mut exam| {
        let Some(handle) = handle.clone().upgrade() else {
            return;
        };
        let server_backend_mutex = server_backend_rc.clone();
        let listener_logger = listener_logger_rc.clone();

        let exams_base = exams_base_clone.clone();
        let exam_path = exams_base.join(exam.id.cast_unsigned().to_string());

        let future = async move {
            let mut server_backend = {
                let mut server_backend = server_backend_mutex.lock().unwrap();
                server_backend.take().unwrap()
            };

            let mut listener_logger = listener_logger.lock().unwrap();
            let listener_logger = listener_logger.as_mut().unwrap();
            let registered_clients = Vec::new();

            exam.ended = false;
            handle.set_active_exam(exam.clone());

            let listen_future = tokio::spawn(async move {
                server_backend
                    .listen(registered_clients, exam_path)
                    .await
                    .map(|_| server_backend)
            });

            while let Some(message) = listener_logger.recv().await {
                match message {
                    ServerBackendMessage::Stopped => {
                        break;
                    }
                    ServerBackendMessage::RegisterUser(user) => {
                        let users = handle.get_registered_users();
                        users.push_row(user).unwrap();
                        handle.set_registered_users(users);
                    }
                    ServerBackendMessage::UserSubmit(user_id) => {
                        let user_id = user_id.cast_signed();
                        let users = handle.get_registered_users();
                        let mut new = Vec::new();
                        for mut user in users.iter() {
                            if user.id == user_id {
                                user.state = UserState::FinishedExam;
                                new.push(user);
                                break;
                            }
                            new.push(user);
                        }
                        handle.set_registered_users(VecModel::from_slice(&new));
                    }
                    _ => {}
                }
            }

            let result = listen_future.await;

            match result {
                Ok(Ok(server_backend)) => {
                    *server_backend_mutex.lock().unwrap() = Some(server_backend);
                    exam.ended = true;
                    handle.set_active_exam(exam);
                }
                Ok(Err(e)) => eprintln!("Error closing exam: {e:?}"),
                Err(e) => eprintln!("Error closing exam: {e:?}"),
            }
        };
        slint::spawn_local(async_compat::Compat::new(future)).unwrap();
    });

    let handle = app_handle.clone();
    app.on_back_exam(move || {
        if let Some(handle) = handle.upgrade() {
            handle.set_active_exam(Exam::null());
        }
    });

    app.run().unwrap();
}
