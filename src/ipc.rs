use crate::{
    AppData,
    wm::{
        task::Task,
        utils::{Position, Rect},
    },
};
use nix::sys::eventfd::EventFd;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::{
    collections::{HashMap, HashSet},
    io::{self, Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::Duration,
};
use wayland_backend::client::ObjectId;

pub type ClientId = u64;

#[derive(Debug)]
pub struct IpcState {
    pub watchers: HashMap<ObjectId, HashSet<ClientId>>,
}

impl IpcState {
    pub fn new() -> Self {
        Self {
            watchers: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum MainRequest {
    Watch {
        client_id: ClientId,
        request_id: u64,
        app_id: String,
    },
    Unwatch {
        client_id: ClientId,
        request_id: u64,
        app_id: String,
    },
    Focus {
        client_id: ClientId,
        request_id: u64,
        app_id: String,
    },
    Disconnected {
        client_id: ClientId,
    },
}

#[derive(Debug, Clone)]
// TODO: maybe rename this
pub enum MainResponse {
    Geometry {
        client_id: ClientId,
        window_id: String,
        center: Position,
    },
    Ok {
        client_id: ClientId,
        request_id: u64,
    },
    OkWatch {
        client_id: ClientId,
        window_id: String,
        request_id: u64,
    },
    Error {
        client_id: ClientId,
        request_id: u64,
        message: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
enum SocketRequest {
    Watch { request_id: u64, app_id: String },
    Unwatch { request_id: u64, app_id: String },
    Focus { request_id: u64, app_id: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum SocketResponse {
    Geometry { window_id: String, center: Position },
    Ok { request_id: u64 },
    OkWatch { request_id: u64, window_id: String },
    Error { request_id: u64, message: String },
}

struct ClientState {
    stream: UnixStream,
    read_buf: Vec<u8>,
}

pub fn spawn_ipc_thread(
    socket_path: PathBuf,
    to_main: Sender<MainRequest>,
    from_main: Receiver<MainResponse>,
    waker: Arc<EventFd>,
) -> io::Result<thread::JoinHandle<()>> {
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)?;
    }

    let listener = UnixListener::bind(&socket_path)?;
    listener.set_nonblocking(true)?;

    let handle = thread::spawn(move || {
        if let Err(err) = run_ipc_thread(listener, to_main, from_main, waker) {
            eprintln!("ipc thread exited: {err}");
        }
    });

    Ok(handle)
}

fn run_ipc_thread(
    listener: UnixListener,
    to_main: Sender<MainRequest>,
    from_main: Receiver<MainResponse>,
    waker: Arc<EventFd>,
) -> io::Result<()> {
    let mut next_client_id: ClientId = 0;
    let mut clients: HashMap<ClientId, ClientState> = HashMap::new();

    loop {
        accept_clients(&listener, &mut clients, &mut next_client_id)?;
        read_clients(&mut clients, &to_main, &waker)?;
        drain_main_events(&mut clients, &from_main)?;
        thread::sleep(Duration::from_millis(5));
    }
}

fn accept_clients(
    listener: &UnixListener,
    clients: &mut HashMap<ClientId, ClientState>,
    next_client_id: &mut ClientId,
) -> io::Result<()> {
    loop {
        match listener.accept() {
            Ok((stream, _addr)) => {
                stream.set_nonblocking(true)?;
                let client_id = *next_client_id;
                *next_client_id += 1;

                clients.insert(
                    client_id,
                    ClientState {
                        stream,
                        read_buf: Vec::new(),
                    },
                );
            }
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
            Err(err) => return Err(err),
        }
    }

    Ok(())
}

fn read_clients(
    clients: &mut HashMap<ClientId, ClientState>,
    to_main: &Sender<MainRequest>,
    waker: &EventFd,
) -> io::Result<()> {
    let mut disconnected = Vec::new();

    for (&client_id, client) in clients.iter_mut() {
        let mut buf = [0u8; 4096];

        loop {
            match client.stream.read(&mut buf) {
                Ok(0) => {
                    disconnected.push(client_id);
                    break;
                }
                Ok(n) => {
                    client.read_buf.extend_from_slice(&buf[..n]);

                    while let Some(pos) = client.read_buf.iter().position(|b| *b == b'\n') {
                        let mut line = client.read_buf.drain(..=pos).collect::<Vec<u8>>();
                        if line.last() == Some(&b'\n') {
                            line.pop();
                        }
                        if line.is_empty() {
                            continue;
                        }

                        match serde_json::from_slice::<SocketRequest>(&line) {
                            Ok(req) => {
                                let msg = match req {
                                    SocketRequest::Watch { request_id, app_id } => {
                                        MainRequest::Watch {
                                            client_id,
                                            request_id,
                                            app_id,
                                        }
                                    }
                                    SocketRequest::Unwatch { request_id, app_id } => {
                                        MainRequest::Unwatch {
                                            client_id,
                                            request_id,
                                            app_id,
                                        }
                                    }
                                    SocketRequest::Focus { request_id, app_id } => {
                                        MainRequest::Focus {
                                            client_id,
                                            request_id,
                                            app_id,
                                        }
                                    }
                                };

                                let _ = to_main.send(msg);
                                let _ = waker.write(1);
                            }
                            Err(err) => {
                                eprintln!("invalid ipc request from client {client_id}: {err}");
                            }
                        }
                    }
                }
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(_) => {
                    disconnected.push(client_id);
                    break;
                }
            }
        }
    }

    for client_id in disconnected {
        clients.remove(&client_id);
        let _ = to_main.send(MainRequest::Disconnected { client_id });
        let _ = waker.write(1);
    }

    Ok(())
}

pub fn drain_main_events(
    clients: &mut HashMap<ClientId, ClientState>,
    from_main: &Receiver<MainResponse>,
) -> io::Result<()> {
    loop {
        match from_main.try_recv() {
            Ok(event) => {
                let (client_id, response) = match event {
                    MainResponse::Geometry {
                        client_id,
                        window_id,
                        center,
                    } => (client_id, SocketResponse::Geometry { window_id, center }),
                    MainResponse::OkWatch {
                        client_id: _client_id,
                        window_id,
                        request_id,
                    } => (
                        _client_id,
                        SocketResponse::OkWatch {
                            request_id,
                            window_id,
                        },
                    ),
                    MainResponse::Ok {
                        client_id: _client_id,
                        request_id,
                    } => (_client_id, SocketResponse::Ok { request_id }),
                    MainResponse::Error {
                        client_id: _client_id,
                        request_id,
                        message,
                    } => (
                        _client_id,
                        SocketResponse::Error {
                            request_id,
                            message,
                        },
                    ),
                };

                if let Some(client) = clients.get_mut(&client_id) {
                    write_json_line(&mut client.stream, &response)?;
                }
            }
            Err(mpsc::TryRecvError::Empty) => break,
            Err(mpsc::TryRecvError::Disconnected) => break,
        }
    }

    Ok(())
}

fn write_json_line(stream: &mut UnixStream, response: &SocketResponse) -> io::Result<()> {
    let mut bytes = serde_json::to_vec(response)?;
    bytes.push(b'\n');
    stream.write_all(&bytes)?;
    stream.flush()?;
    Ok(())
}
