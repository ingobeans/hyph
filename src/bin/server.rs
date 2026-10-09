use std::{
    io::{self, Read},
    net::{SocketAddr, TcpListener, TcpStream},
};

use num_traits::FromPrimitive;

use crate::utils::*;

#[path = "../utils.rs"]
mod utils;

async fn handle_connection(mut socket: TcpStream, addr: SocketAddr) -> io::Result<()> {
    let mut message_buf = [0u8];
    socket.read_exact(&mut message_buf)?;

    println!("[{addr}]: connected with message type: {}", message_buf[0]);
    let Some(message_type) = MessageTypes::from_u8(message_buf[0]) else {
        return io::Result::Err(io::Error::other("Unknown message type"));
    };

    match message_type {
        MessageTypes::Ping => {}
        MessageTypes::Register => {}
    }

    let mut data = [0u8; 256];
    loop {
        let len = socket.read(&mut data)?;
        if len == 0 {
            println!("dc");
            return Ok(());
        }
        println!("received data: {:?}", &data[..len]);
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", PORT))?;
    println!("connected to socket");

    loop {
        match listener.accept() {
            Ok((socket, addr)) => {
                println!("{addr} just connected!");
                tokio::spawn(handle_connection(socket, addr));
            }
            Err(e) => println!("client failed connecting: {e:?}"),
        }
    }
}
