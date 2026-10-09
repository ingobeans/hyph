use std::io::prelude::*;
use std::net::TcpStream;
// use std::thread::sleep;
// use std::time::Duration;

use crate::utils::*;

#[path = "../utils.rs"]
mod utils;

fn main() -> std::io::Result<()> {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", PORT))?;
    println!("connected to socket");

    stream.write(&[1])?;
    println!("sent to socket");
    // stream.read(&mut [0; 128])?;
    // sleep(Duration::from_secs(4));
    Ok(())
}
