use std::fs;
use std::io::{self, prelude::*};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::utils::*;

pub struct Client {
    pub addr: String,
    pub id: u64,
}
impl Client {
    pub fn new(addr: String) -> Self {
        Self {
            addr,
            id: rand::random(),
        }
    }

    async fn send_packet_header(
        &self,
        stream: &mut TcpStream,
        message_type: ClientMessageType,
    ) -> io::Result<()> {
        stream.write_all(&[message_type as u8]).await?;
        stream.write_all(&self.id.to_le_bytes()).await
    }

    pub async fn ping_server(&self) -> io::Result<()> {
        let mut stream = TcpStream::connect(&self.addr).await?;
        stream.write_all(&[ClientMessageType::Ping as u8]).await?;
        let mut resp_buf = [0u8];
        stream.read_exact(&mut resp_buf).await?;
        let message_type: ServerMessageType = parse_message_type(resp_buf[0])?;
        if message_type != ServerMessageType::Pong {
            return std::io::Result::Err(std::io::Error::other("Incorrect ping response"));
        }
        Ok(())
    }

    pub async fn send_file_to_server(&self, filepath: &str) -> io::Result<()> {
        let mut stream = TcpStream::connect(&self.addr).await?;
        self.send_packet_header(&mut stream, ClientMessageType::Transmit).await?;

        let mut file = fs::File::open(filepath)?;
        let mut buf = [0u8; 8192];
        loop {
            let len = file.read(&mut buf)?;
            if len == 0 {
                return Ok(());
            }
            stream.write_all(&buf[0..len]).await?;
        }
    }

    pub async fn register(&self, name: &str) -> io::Result<bool> {
        let bytes = name.as_bytes();
        if bytes.len() >= MAX_NAME_LENGTH {
            return Ok(false);
        }

        let mut stream = TcpStream::connect(&self.addr).await?;
        self.send_packet_header(&mut stream, ClientMessageType::Register).await?;

        let mut bytes = bytes.to_vec();
        bytes.resize(MAX_NAME_LENGTH, 0);
        stream.write_all(&bytes).await?;

        let mut resp_buf = [0u8; 2];
        stream.read_exact(&mut resp_buf).await?;
        let message_type: ServerMessageType = parse_message_type(resp_buf[0])?;
        if message_type != ServerMessageType::RegisterStatus {
            return std::io::Result::Err(std::io::Error::other("Incorrect register status"));
        }

        Ok(resp_buf[1] == 1)
    }
}