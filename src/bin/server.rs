use std::{
    io::Read,
    net::{SocketAddr, TcpListener, TcpStream},
};

use crate::utils::*;

#[path = "../utils.rs"]
mod utils;

async fn handle_connection(mut socket: TcpStream, _addr: SocketAddr) -> std::io::Result<()> {
    let mut data = [0u8; 8192];
    loop {
        let len = socket.read(&mut data)?;
        if len == 0 {
            return Ok(());
        }
        println!("received data: {:?}", &data[..len]);
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
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
