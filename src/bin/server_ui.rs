use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use slint::{ToSharedString, VecModel};

fn load_previous_exams(base: &Path) -> Vec<Exam> {
    let mut previous_exams = Vec::new();
    for item in std::fs::read_dir(base).unwrap().flatten() {
        if item.file_type().unwrap().is_file() {
            continue;
        }

        let Ok(id) = item.file_name().to_string_lossy().parse() else {
            continue;
        };

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
        };
        previous_exams.push(exam);
    }
    previous_exams
}

fn load_exam_submissions(exam: &Exam, base: &Path) -> Vec<RegisteredUser> {
    let mut users = Vec::new();

    let submissions_path = base.join(exam.id.to_string()).join("submissions");

    for submission in std::fs::read_dir(submissions_path).unwrap().flatten() {
        if submission.file_type().unwrap().is_file() {
            continue;
        }

        let Ok(id) = submission.file_name().to_string_lossy().parse() else {
            continue;
        };

        let path = submission.path();
        let info_path = path.join("info.txt");
        let Ok(info) = std::fs::read_to_string(info_path) else {
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
    app.on_show_previous_exam(move |id| {
        let Some(exam) = previous_exams.iter().find(|a| a.id == id) else {
            return;
        };
        if let Some(handle) = handle.upgrade() {
            handle.set_registered_users(VecModel::from_slice(&load_exam_submissions(
                exam,
                &exams_base,
            )));
            handle.set_active_exam(exam.clone());
        }
    });

    app.run().unwrap();
}
