use std::io::{self, prelude::*};
use std::net::TcpStream;
// use std::thread::sleep;
// use std::time::Duration;

use crate::utils::*;

#[path = "../utils.rs"]
mod utils;

fn ping_server(addr: &str) -> io::Result<()> {
    let mut stream = TcpStream::connect(addr)?;
    stream.write(&[MessageType::Ping as u8])?;
    let mut resp_buf = [0u8];
    stream.read_exact(&mut resp_buf)?;
    let message_type = parse_message_type(resp_buf[0])?;
    if message_type != MessageType::Ping {
        return std::io::Result::Err(std::io::Error::other("Incorrect ping response"));
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    let addr = format!("127.0.0.1:{}", PORT);
    let server_accessible = ping_server(&addr).is_ok();
    println!("{server_accessible}");

    Ok(())
}
