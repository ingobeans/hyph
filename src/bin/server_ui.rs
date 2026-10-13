use std::rc::Rc;

use slint::{ToSharedString, VecModel};

slint::include_modules!();
fn main() {
    let app = ServerApp::new().unwrap();

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
