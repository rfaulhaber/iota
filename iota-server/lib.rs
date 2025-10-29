use interprocess::local_socket::{
    GenericNamespaced, ListenerOptions,
    tokio::{Listener, Stream, prelude::*},
};
use iota_editor::Editor;
use iota_protocol::Message;
use std::path::PathBuf;
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
};

#[derive(Debug)]
pub enum ServerStatus {
    Connected,
    Running,
    Disconnected,
}

#[derive(Debug)]
pub enum ConnectionType {
    // TODO embedded?
    Local(Listener),
    // Remote(SocketAddr),
}

#[derive(Debug)]
pub struct Server {
    connection: ConnectionType,
    editor: Editor,
}

impl Server {
    pub async fn local(socket_path: PathBuf) -> anyhow::Result<Self> {
        if fs::try_exists(&socket_path).await? {
            fs::remove_file(&socket_path).await?;
        }

        let socket_name = socket_path
            .to_str()
            .unwrap()
            .to_ns_name::<GenericNamespaced>()?;

        let opts = ListenerOptions::new().name(socket_name);

        let listener = opts.create_tokio()?;

        Ok(Self {
            connection: ConnectionType::Local(listener),
            editor: Editor::new(),
        })
    }

    pub async fn run(self) -> anyhow::Result<()> {
        match self.connection {
            ConnectionType::Local(listener) => local_listener_handler(listener).await,
        }
    }
}

async fn local_listener_handler(listener: Listener) -> anyhow::Result<()> {
    loop {
        let conn = listener.accept().await?;

        tokio::spawn(async move {
            match handle_conn(conn).await {
                Ok(c) => c,
                Err(_) => todo!(),
            }
        });
    }
}

async fn handle_conn(conn: Stream) -> anyhow::Result<()> {
    let mut recver = BufReader::new(&conn);
    let mut sender = &conn;

    let mut buf = Vec::new();
    let recv = recver.read(&mut buf).await?;

    let message: Message = bincode::decode_from_slice(&buf, bincode::config::standard())?.0;

    let response = match message {
        Message::Request { id, key } => todo!(),
        Message::Response { id } => todo!(),
        Message::Notification { id } => todo!(),
    };

    let send = sender.write(b"hello world").await?;

    Ok(())
}
