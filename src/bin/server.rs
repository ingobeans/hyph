use std::{io::Read, net::TcpListener};

use crate::utils::*;

#[path = "../utils.rs"]
mod utils;

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", PORT))?;
    println!("connected to socket");

    loop {
        match listener.accept() {
            Ok((mut socket, addr)) => {
                println!("{addr} just connected!");
                let mut data = [0u8; 1024];
                loop {
                    let len = socket.read(&mut data)?;
                    if len == 0 {
                        break;
                    }
                    println!("received data: {:?}", &data[..len]);
                }
            }
            Err(e) => println!("client failed connecting: {e:?}"),
        }
    }
}
