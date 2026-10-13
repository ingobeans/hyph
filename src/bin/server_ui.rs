use slint::{ToSharedString, VecModel};

slint::include_modules!();
fn main() {
    let app = ServerApp::new().unwrap();
    app.set_registered_users(VecModel::from_slice(&[RegisteredUser {
        name: "hi".to_shared_string(),
        state: 0,
    }]));
    app.run().unwrap();
}
