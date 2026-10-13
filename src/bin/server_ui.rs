use std::rc::Rc;

use slint::{ToSharedString, VecModel};

fn load_previous_exams() -> Vec<PreviousExam> {
    let mut previous_exams = Vec::new();
    for item in std::fs::read_dir("exams/").unwrap().flatten() {
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

        let exam = PreviousExam {
            id: id,
            date: date.to_shared_string(),
            name: name.to_shared_string(),
            submissions: count as i32,
        };
        previous_exams.push(exam);
    }
    previous_exams
}

slint::include_modules!();
fn main() {
    let previous_exams = load_previous_exams();

    let app = ServerApp::new().unwrap();

    app.set_previous_exams(VecModel::from_slice(&previous_exams));

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

    app.set_registered_users(VecModel::from_slice(&[RegisteredUser {
        name: "hi".to_shared_string(),
        state: 0,
    }]));
    app.run().unwrap();
}
