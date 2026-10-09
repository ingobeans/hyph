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
        let id = 0;
        Self { addr, id }
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
        stream.write(&[ClientMessageType::Ping as u8])?;
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
            stream.write(&buf[0..len])?;
            if len == 0 {
                return Ok(());
            }
        }
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
    client.send_file_to_server("dummy.bin")?;
    println!("succesfully transmitted dummy.bin to server");
    Ok(())
}
