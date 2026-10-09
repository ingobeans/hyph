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

    stream.write(&[MessageType::Ping as u8])?;
    // sleep(Duration::from_secs(1));
    let mut resp_buf = [0u8];
    stream.read_exact(&mut resp_buf)?;
    let message_type = parse_message_type(resp_buf[0])?;
    println!("pinged with response type: {message_type:?}");
    Ok(())
}
