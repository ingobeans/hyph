use std::{
    collections::{HashMap, hash_map::Entry},
    io::{self, Write},
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

use crate::utils::*;

#[path = "../utils.rs"]
mod utils;

#[derive(Debug)]
struct ServersideClient {
    name: String,
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
        socket.read_exact(&mut message_buf).await?;

        let message_type = parse_message_type(message_buf[0])?;

        // handle ping separately, as it doesn't need a client ID
        if let ClientMessageType::Ping = message_type {
            socket.write_all(&[ServerMessageType::Pong as u8]).await?;
            println!("[{addr}]: {message_type:?} - pinged back!");
            return Ok(());
        }

        let mut id_buf = [0u8; 8];
        socket.read_exact(&mut id_buf).await?;
        let id = u64::from_le_bytes(id_buf);
        let message = format!("[{addr}] ({id:0>20}): {message_type:?} -");

        match message_type {
            ClientMessageType::Register => {
                // read name data
                let mut name_buf = Vec::new();

                // use buf reader so we can use .read_until to find 0x00
                // which marks end of name
                let mut reader = BufReader::new(socket);
                reader.read_until(0, &mut name_buf).await?;

                if name_buf.last() == Some(&0) {
                    name_buf.pop();
                }

                let name_buf = String::from_utf8_lossy(&name_buf).to_string();

                let result = {
                    let mut lock = registered_clients.lock().unwrap();
                    if let Entry::Vacant(e) = lock.entry(id) {
                        println!("{message} registered user '{name_buf}'!");
                        e.insert(ServersideClient { name: name_buf });
                        1
                    } else {
                        println!("{message} failed to register user, ID already used!");
                        0
                    }
                };
                reader
                    .write_all(&[ServerMessageType::RegisterStatus as u8, result])
                    .await?;
            }
            ClientMessageType::Transmit => {
                let mut data = [0u8; 8192];
                let mut file = fs::File::create(format!("received/{id}.bin")).await?;
                let mut total = 0;
                loop {
                    let len = socket.read(&mut data).await?;
                    if len == 0 {
                        println!("{message} received {total} bytes");
                        break;
                    }
                    total += len;
                    file.write_all(&data[..len]).await?;
                }
            }
            _ => {}
        }

        Ok(())
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
