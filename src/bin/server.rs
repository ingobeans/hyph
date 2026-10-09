use std::{
    fs,
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
    print!("[{addr}]: {message_type:?} - ");

    match message_type {
        ClientMessageType::Ping => {
            socket.write_all(&[ClientMessageType::Ping as u8])?;
            print!("pinged back!");
        }
        ClientMessageType::Register => {}
        ClientMessageType::Transmit => {
            let mut data = [0u8; 8192];
            let mut file = fs::File::create("received.bin")?;
            let mut total = 0;
            loop {
                let len = socket.read(&mut data)?;
                if len == 0 {
                    println!("received {total} bytes");
                    return Ok(());
                }
                total += len;
                file.write_all(&data[..len])?;
            }
        }
    }
    println!();
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
