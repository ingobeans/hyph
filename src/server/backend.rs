use std::{
    collections::{HashMap, hash_map::Entry},
    io::{self},
    net::SocketAddr,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
};

use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::utils::*;

#[path = "../utils.rs"]
mod utils;

#[derive(Debug)]
pub struct ServersideClient {
    #[expect(dead_code)]
    name: String,
}

pub enum ServerBackendMessage {}

pub struct Server {
    addr: String,
    registered_clients: Arc<Mutex<HashMap<u64, ServersideClient>>>,
    exam_path: PathBuf,
    listener: TcpListener,
    logging_tx: Sender<ServerBackendMessage>,
    terminate_rx: Receiver<()>,
}
impl Server {
    pub async fn start(
        addr: String,
        registered_clients: Arc<Mutex<HashMap<u64, ServersideClient>>>,
        exam_path: PathBuf,
    ) -> std::io::Result<(Self, Receiver<ServerBackendMessage>, Sender<()>)> {
        let (terminate_tx, terminate_rx) = mpsc::channel();
        let (logging_tx, logging_rx) = mpsc::channel();
        let listener = TcpListener::bind(&addr).await?;
        println!("connected to socket");

        let new = Self {
            addr,
            registered_clients,
            exam_path,
            listener,
            logging_tx,
            terminate_rx,
        };
        Ok((new, logging_rx, terminate_tx))
    }
    pub async fn listen(&mut self) {
        let received_path = Arc::new(self.exam_path.join("received"));
        let submission_path = Arc::new(self.exam_path.join("submissions"));
        loop {
            let received_path = received_path.clone();
            let submission_path = submission_path.clone();
            match self.listener.accept().await {
                Ok((socket, addr)) => {
                    let a = self.registered_clients.clone();
                    tokio::spawn(async move {
                        if let Err(e) =
                            Self::handle_connection(socket, addr, a, received_path, submission_path)
                                .await
                        {
                            eprintln!("[Connection Crashed] ({addr}): {e}");
                        }
                    });
                }
                Err(e) => println!("client failed connecting: {e:?}"),
            }
        }
    }

    async fn handle_connection(
        mut socket: TcpStream,
        addr: SocketAddr,
        registered_clients: Arc<Mutex<HashMap<u64, ServersideClient>>>,
        received_path: Arc<PathBuf>,
        submissions_path: Arc<PathBuf>,
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
                let mut name_buf = [0u8; MAX_NAME_LENGTH];

                socket.read_exact(&mut name_buf).await?;
                let len: usize = name_buf.iter().position(|f| *f == 0).unwrap_or(0);

                if len == 0 {
                    println!("{message} failed to register user, bad name!");
                    socket
                        .write_all(&[ServerMessageType::RegisterStatus as u8, 0])
                        .await?;
                    return Ok(());
                }
                let name_buf = String::from_utf8_lossy(&name_buf[..len]).to_string();

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
                socket
                    .write_all(&[ServerMessageType::RegisterStatus as u8, result])
                    .await?;
            }
            ClientMessageType::Transmit => {
                let mut data = [0u8; 8192];
                let path = received_path.join(format!("{id}.bin"));
                let mut file = fs::File::create(&path).await?;
                let mut total = 0;
                loop {
                    let len = socket.read(&mut data).await?;
                    if len == 0 {
                        println!(
                            "{message} received {total} bytes ({}MB). Unzipping...",
                            total / 1000000
                        );
                        break;
                    }
                    total += len;
                    file.write_all(&data[..len]).await?;
                }
                drop(file);

                // todo: handle result
                let _result = tokio::task::spawn_blocking(move || {
                    let unzipped_path = submissions_path.join(id.to_string());
                    ripzip::extract_to_directory(&path, &unzipped_path, &ripzip::NoProgress)
                })
                .await;
            }
            _ => {}
        }

        Ok(())
    }
}
