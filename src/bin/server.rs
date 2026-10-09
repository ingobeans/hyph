use std::{
    fs,
    io::{self, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::Mutex,
};

use crate::utils::*;

#[path = "../utils.rs"]
mod utils;

static CONNECTION_COUNT: Mutex<u8> = Mutex::new(0);

async fn handle_connection(mut socket: TcpStream, addr: SocketAddr) -> io::Result<()> {
    let mut message_buf = [0u8];
    socket.read_exact(&mut message_buf)?;

    let message_type = parse_message_type(message_buf[0])?;

    let mut guard = CONNECTION_COUNT.lock().unwrap();
    *guard += 1;
    let message = format!("{guard} - [{addr}]: {message_type:?} -");
    drop(guard);

    match message_type {
        ClientMessageType::Ping => {
            socket.write_all(&[ClientMessageType::Ping as u8])?;
            println!("{message} pinged back!");
        }
        ClientMessageType::Register => {}
        ClientMessageType::Transmit => {
            let mut data = [0u8; 8192];
            let mut file = fs::File::create("received.bin")?;
            let mut total = 0;
            loop {
                let len = socket.read(&mut data)?;
                if len == 0 {
                    println!("{message} received {total} bytes");
                    return Ok(());
                }
                total += len;
                file.write_all(&data[..len])?;
            }
        }
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
