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
    let addr = format!("127.0.0.1:{}", PORT);
    let client = Client::new(addr);
    let c = Rc::new(Mutex::new(client));

    let app = App::new().unwrap();
    
    let app_handle = Rc::new(app.as_weak());
    app.on_pingClick(move || {
        let c = c.clone();
        let handle = app_handle.clone();
        if let Some(handle) = handle.upgrade() {
            let mut c = c.lock().unwrap();
            c.addr = format!("{}:{}",handle.get_ip_address(),PORT);
            println!("{}",c.addr);
        }
        
        let future = async move {
            let c = c.lock().unwrap();
            let state = c.ping_server().await.is_ok();
            println!("{state}");
            if let Some(handle) = handle.upgrade() {
                handle.set_ping_state(state.to_shared_string());
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
