use std::{
    io::{self, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
};

use crate::utils::*;

#[path = "../utils.rs"]
mod utils;

async fn handle_connection(mut socket: TcpStream, addr: SocketAddr) -> io::Result<()> {
    let mut message_buf = [0u8];
    socket.read_exact(&mut message_buf)?;

    let message_type = parse_message_type(message_buf[0])?;
    println!("[{addr}]: connected with message type: {message_type:?}");

    match message_type {
        MessageType::Ping => {
            socket.write_all(&[MessageType::Ping as u8])?;
        }
        MessageType::Register => {}
        MessageType::Transmit => {}
    }
    return Ok(());
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", PORT))?;
    println!("connected to socket");

    loop {
        match listener.accept() {
            Ok((socket, addr)) => {
                tokio::spawn(handle_connection(socket, addr));
            }
            Err(e) => println!("client failed connecting: {e:?}"),
        }
    }
}
