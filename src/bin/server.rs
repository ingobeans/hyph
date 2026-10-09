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
    let count = *guard;
    drop(guard);

    // handle ping separately, as it doesn't need a client ID
    if let ClientMessageType::Ping = message_type {
        socket.write_all(&[ClientMessageType::Ping as u8])?;
        println!("{count:0>3}. [{addr}]: {message_type:?} - pinged back!");
        return Ok(());
    }

    let mut id_buf = [0u8; 8];
    socket.read_exact(&mut id_buf)?;
    let id = u64::from_le_bytes(id_buf);
    let message = format!("{count:0>3}. [{addr}] ({id:0>20}): {message_type:?} -");

    match message_type {
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
        _ => {}
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
