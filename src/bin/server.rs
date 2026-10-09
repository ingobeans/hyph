use std::{
    collections::HashMap,
    fs,
    io::{self, BufRead, BufReader, Bytes, Read, Write},
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use tokio::{
    io::Interest,
    net::{TcpListener, TcpStream},
};

use crate::utils::*;

#[path = "../utils.rs"]
mod utils;

#[derive(Debug)]
struct ServersideClient {
    name: String,
}

async fn read_exact(stream: &mut TcpStream, mut buf: &mut [u8]) -> io::Result<()> {
    stream.ready(Interest::READABLE).await;
    while !buf.is_empty() {
        match stream.try_read(buf) {
            Ok(0) => break,
            Ok(n) => {
                buf = &mut buf[n..];
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

async fn write_all(stream: &mut TcpStream, mut buf: &[u8]) -> io::Result<()> {
    stream.ready(Interest::WRITABLE).await;
    while !buf.is_empty() {
        match stream.try_write(buf) {
            Ok(0) => {
                return Err(io::Error::other(""));
            }
            Ok(n) => buf = &buf[n..],
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

struct Server {
    addr: String,
    registered_clients: Arc<Mutex<HashMap<u64, ServersideClient>>>,
}
impl Server {
    async fn handle_connection(
        mut socket: TcpStream,
        addr: SocketAddr,
        registered_clients: Arc<Mutex<HashMap<u64, ServersideClient>>>,
    ) -> io::Result<()> {
        let mut message_buf = [0u8];
        read_exact(&mut socket, &mut message_buf).await?;

        let message_type = parse_message_type(message_buf[0])?;

        // handle ping separately, as it doesn't need a client ID
        if let ClientMessageType::Ping = message_type {
            write_all(&mut socket, &[ServerMessageType::Pong as u8]).await?;
            println!("[{addr}]: {message_type:?} - pinged back!");
            return Ok(());
        }

        let mut id_buf = [0u8; 8];
        read_exact(&mut socket, &mut id_buf).await?;
        let id = u64::from_le_bytes(id_buf);
        let message = format!("[{addr}] ({id:0>20}): {message_type:?} -");

        match message_type {
            ClientMessageType::Register => {
                // read name data
                let mut name_buf = Vec::new();

                read_exact(&mut socket, &mut name_buf).await?;
                let name_buf = String::from_utf8_lossy(&name_buf).to_string();

                let result = {
                    let mut lock = registered_clients.lock().unwrap();
                    if lock.contains_key(&id) {
                        println!("{message} failed to register user, ID already used!");
                        0
                    } else {
                        println!("{message} registered user!");
                        lock.insert(id, ServersideClient { name: name_buf });
                        1
                    }
                };
                write_all(
                    &mut socket,
                    &[ServerMessageType::RegisterStatus as u8, result],
                )
                .await?;
            }
            ClientMessageType::Transmit => {
                let mut data = [0u8; 8192];
                let mut file = fs::File::create(format!("received/{id}.bin"))?;
                let mut total = 0;
                loop {
                    socket.ready(Interest::READABLE).await?;
                    let len = socket.try_read(&mut data)?;
                    if len == 0 {
                        println!("{message} received {total} bytes");
                        break;
                    }
                    total += len;
                    file.write_all(&data[..len])?;
                }
            }
            _ => {}
        }

        return Ok(());
    }
    fn new(addr: String, registered_clients: Arc<Mutex<HashMap<u64, ServersideClient>>>) -> Self {
        Self {
            addr,
            registered_clients,
        }
    }
    async fn listen(&mut self) -> io::Result<()> {
        let listener = TcpListener::bind(&self.addr).await?;
        println!("connected to socket");

        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    let a = self.registered_clients.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(socket, addr, a).await {
                            eprintln!("{e}");
                        }
                    });
                }
                Err(e) => println!("client failed connecting: {e:?}"),
            }
        }
    }
}
#[tokio::main]
async fn main() -> io::Result<()> {
    let addr = format!("127.0.0.1:{}", PORT);
    let registered_clients = Arc::new(Mutex::new(HashMap::new()));
    let mut server = Server::new(addr, registered_clients);
    server.listen().await
}
