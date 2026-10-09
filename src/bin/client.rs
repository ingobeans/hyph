use crate::{backend::*,ui::*, utils::*};

#[path = "../utils.rs"]
mod utils;

#[path = "../client/ui.rs"]
mod ui;
#[path = "../client/backend.rs"]
mod backend;

fn main() -> std::io::Result<()> {
    let app = App::new().unwrap();
    app.run().unwrap();

    return Ok(());
    let addr = format!("127.0.0.1:{}", PORT);
    let client = Client::new(addr);
    let server_accessible = client.ping_server().is_ok();
    if !server_accessible {
        println!("server is inaccessible");
        return Ok(());
    }
    let success = client.register("my cool name")?;
    println!("{success}");
    client.send_file_to_server("dummy.bin")?;
    println!("succesfully transmitted dummy.bin to server");
    Ok(())
}
