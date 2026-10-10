use std::{rc::Rc, sync::Mutex};

use slint::ToSharedString;

use crate::{backend::*,ui::*, utils::*};

#[path = "../utils.rs"]
mod utils;

#[path = "../client/ui.rs"]
mod ui;
#[path = "../client/backend.rs"]
mod backend;

fn main() -> std::io::Result<()> {
    let name = whoami::realname()?;
    let client = Client::new(String::new());
    let client_rc = Rc::new(Mutex::new(client));

    let app = App::new().unwrap();
    
    app.set_username(format!("Logged in as: {name}").to_shared_string());
    
    let app_handle = Rc::new(app.as_weak());

    let c = client_rc.clone();
    let handle = app_handle.clone();
    app.on_pingClick(move || {
        let c = c.clone();
        let handle = handle.clone();
        if let Some(handle) = handle.upgrade() {
            handle.set_ping_enabled(false);
            let mut c = c.lock().unwrap();
            c.addr = format!("{}:{}",handle.get_ip_address(),PORT);
            println!("{}",c.addr);
        }
        
        let future = async move {
            let state =  {
                let c = c.lock().unwrap();
                c.ping_server().await.is_ok()
            };
            println!("{state}");
            if let Some(handle) = handle.upgrade() {
                handle.set_ping_enabled(true);
                handle.set_ping_success(state);
                handle.set_ping_fail(!state);
            }
        };
        slint::spawn_local(async_compat::Compat::new(future)).unwrap();
    });
    
    let c = client_rc.clone();
    let handle = app_handle.clone();
    app.on_startExamClick(move || {
        let c = c.clone();
        let handle = handle.clone();
        let name = name.clone();
        if let Some(handle) = handle.upgrade() {
            handle.set_ping_enabled(false);
            handle.set_start_exam_enabled(false);
        }
        
        let future = async move {
            let state =  {
                let c = c.lock().unwrap();
                c.register(&name).await.is_ok_and(|f| f)
            };
            println!("{state}");
            if let Some(handle) = handle.upgrade() {
                handle.set_ping_enabled(true);
                handle.set_start_exam_success(state);
                handle.set_start_exam_fail(!state);
            }
        };
        slint::spawn_local(async_compat::Compat::new(future)).unwrap();
    });


    app.run().unwrap();

    // let server_accessible = client.ping_server().is_ok();
    // if !server_accessible {
    //     println!("server is inaccessible");
    //     return Ok(());
    // }
    // let success = client.register("my cool name")?;
    // println!("{success}");
    // client.send_file_to_server("dummy.bin")?;
    // println!("succesfully transmitted dummy.bin to server");
    Ok(())
}
