use std::fs;
use std::io::{self, prelude::*};
use std::net::TcpStream;

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

fn send_file_to_server(filepath: &str, addr: &str) -> io::Result<()> {
    let mut stream = TcpStream::connect(addr)?;
    stream.write(&[MessageType::Transmit as u8])?;

    let mut file = fs::File::open(filepath)?;
    let mut buf = [0u8; 8192];
    loop {
        let len = file.read(&mut buf)?;
        stream.write(&buf[0..len])?;
        if len == 0 {
            return Ok(());
        }
    }
}

fn main() -> std::io::Result<()> {
    let addr = format!("127.0.0.1:{}", PORT);
    let server_accessible = ping_server(&addr).is_ok();
    if !server_accessible {
        println!("server is inaccessible");
        return Ok(());
    }
    send_file_to_server("dummy.bin", &addr)?;
    println!("succesfully transmitted dummy.bin to server");
    Ok(())
}
