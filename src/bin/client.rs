use std::fs;
use std::io::{self, prelude::*};
use std::net::TcpStream;

use crate::utils::*;

#[path = "../utils.rs"]
mod utils;

struct Client {
    addr: String,
    id: u64,
}
impl Client {
    fn new(addr: String) -> Self {
        Self {
            addr,
            id: rand::random(),
        }
    }
    fn send_packet_header(
        &self,
        stream: &mut TcpStream,
        message_type: ClientMessageType,
    ) -> io::Result<()> {
        stream.write_all(&[message_type as u8])?;
        stream.write_all(&self.id.to_le_bytes())
    }

    fn ping_server(&self) -> io::Result<()> {
        let mut stream = TcpStream::connect(&self.addr)?;
        stream.write_all(&[ClientMessageType::Ping as u8])?;
        let mut resp_buf = [0u8];
        stream.read_exact(&mut resp_buf)?;
        let message_type: ServerMessageType = parse_message_type(resp_buf[0])?;
        if message_type != ServerMessageType::Pong {
            return std::io::Result::Err(std::io::Error::other("Incorrect ping response"));
        }
        Ok(())
    }

    fn send_file_to_server(&self, filepath: &str) -> io::Result<()> {
        let mut stream = TcpStream::connect(&self.addr)?;
        self.send_packet_header(&mut stream, ClientMessageType::Transmit)?;

        let mut file = fs::File::open(filepath)?;
        let mut buf = [0u8; 8192];
        loop {
            let len = file.read(&mut buf)?;
            if len == 0 {
                return Ok(());
            }
            stream.write_all(&buf[0..len])?;
        }
    }

    fn register(&self, name: &str) -> io::Result<bool> {
        let mut stream = TcpStream::connect(&self.addr)?;
        self.send_packet_header(&mut stream, ClientMessageType::Register)?;

        let bytes = name.as_bytes();
        if bytes.len() > MAX_NAME_LENGTH {
            return Ok(false);
        }
        let mut bytes = bytes.to_vec();
        bytes.resize(MAX_NAME_LENGTH, 0);
        stream.write_all(&bytes)?;

        let mut resp_buf = [0u8; 2];
        stream.read_exact(&mut resp_buf)?;
        let message_type: ServerMessageType = parse_message_type(resp_buf[0])?;
        if message_type != ServerMessageType::RegisterStatus {
            return std::io::Result::Err(std::io::Error::other("Incorrect register status"));
        }

        Ok(resp_buf[1] == 1)
    }
}

fn main() -> std::io::Result<()> {
    let addr = format!("127.0.0.1:{}", PORT);
    let client = Client::new(addr);
    let server_accessible = client.ping_server().is_ok();
    if !server_accessible {
        println!("server is inaccessible");
        return Ok(());
    }
    // client.send_file_to_server("dummy.bin")?;
    // println!("succesfully transmitted dummy.bin to server");
    let success = client.register("name person")?;
    println!("{success}");
    Ok(())
}
