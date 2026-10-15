use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, Mutex},
};

use chrono::Local;
use slint::{SharedString, ToSharedString, VecModel};

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

        let Ok(id) = submission.file_name().to_string_lossy().parse() else {
            continue;
        };

        let path = submission.path();
        let info_path = path.join("info.txt");
        let Ok(info) = tokio::fs::read_to_string(info_path).await else {
            continue;
        };
        let mut info_rows = info.split("\n");
        let Some(name) = info_rows.next() else {
            continue;
        };

        let user = RegisteredUser {
            id,
            name: name.to_shared_string(),
            state: UserState::FinishedExam,
        };
        users.push(user);
    }
    users
}

slint::include_modules!();
fn main() {
    let exams_base = PathBuf::from("exams/");
    let previous_exams = load_previous_exams(&exams_base);

    let app = ServerApp::new().unwrap();

    app.set_previous_exams(VecModel::from_slice(&previous_exams));
    let previous_exams_rc = Rc::new(previous_exams);

    let app_handle = Rc::new(app.as_weak());

    let handle = app_handle.clone();
    app.on_close_program(move || {
        if let Some(handle) = handle.upgrade() {
            handle.window().hide().unwrap();
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

    let handle = app_handle.clone();
    let exams_base_clone = exams_base.clone();
    app.on_start_exam(move |mut exam| {
        let Some(handle) = handle.clone().upgrade() else {
            return;
        };
        let exams_base = exams_base_clone.clone();
        let exam_path = exams_base.join(exam.id.cast_unsigned().to_string());

        let future = async move {
            let registered_clients = Arc::new(Mutex::new(HashMap::new()));

            let Ok((mut server, logging_rx, terminate_tx)) =
                Server::start(format!("0.0.0.0:{}", PORT), registered_clients, exam_path).await
            else {
                println!("todo: server log fail");
                return;
            };

            exam.ended = false;
            handle.set_active_exam(exam);

            server.listen().await;
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
