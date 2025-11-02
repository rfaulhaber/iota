use interprocess::local_socket::{
    GenericNamespaced, ListenerOptions,
    tokio::{Listener, Stream, prelude::*},
};
use iota_editor::{Editor, EditorEvent as IotaEditorEvent};
use iota_protocol::{EditorEvent, EditorInfo, Message, Position, RenderData};
use log::{error, info};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
    sync::RwLock,
};

#[derive(Debug)]
pub struct Server {
    listener: Listener,
    editor: Arc<RwLock<Editor>>,
}

impl Server {
    pub async fn local(socket_path: PathBuf) -> anyhow::Result<Self> {
        if fs::try_exists(&socket_path).await? {
            fs::remove_file(&socket_path).await?;
        }

        let socket_name = socket_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid socket path"))?
            .to_ns_name::<GenericNamespaced>()?;

        let opts = ListenerOptions::new().name(socket_name);
        let listener = opts.create_tokio()?;

        info!("Server listening on socket: {:?}", socket_path);

        Ok(Self {
            listener,
            editor: Arc::new(RwLock::new(Editor::new())),
        })
    }

    pub async fn run(self) -> anyhow::Result<()> {
        loop {
            match self.listener.accept().await {
                Ok(conn) => {
                    info!("New client connected");
                    let editor = Arc::clone(&self.editor);
                    tokio::spawn(async move {
                        if let Err(e) = handle_client(conn, editor).await {
                            error!("Client handler error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }
}

/// Handle a single client connection
async fn handle_client(mut conn: Stream, editor: Arc<RwLock<Editor>>) -> anyhow::Result<()> {
    loop {
        // Read message length (4 bytes, big-endian u32)
        let mut len_buf = [0u8; 4];
        match conn.read_exact(&mut len_buf).await {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                info!("Client disconnected: {:?}", e);
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        }

        let msg_len = u32::from_be_bytes(len_buf) as usize;

        // Read message data
        let mut msg_buf = vec![0u8; msg_len];
        conn.read_exact(&mut msg_buf).await?;

        // Decode message
        let message = Message::decode(&msg_buf)?;

        // Process message and get response
        let response = match message {
            Message::KeyPress { key } => {
                let mut editor = editor.write().await;
                let events = editor.process_key(key).await;

                // Convert iota_editor events to protocol events
                let protocol_events: Vec<EditorEvent> = events
                    .into_iter()
                    .map(|e| match e {
                        IotaEditorEvent::Shutdown => EditorEvent::Shutdown,
                        IotaEditorEvent::Redraw => EditorEvent::Redraw,
                        IotaEditorEvent::Error(msg) => EditorEvent::Error(msg),
                        IotaEditorEvent::Info(msg) => EditorEvent::Info(msg),
                    })
                    .collect();

                // Get current view for viewport calculations
                let view = editor
                    .get_current_view()
                    .ok_or_else(|| anyhow::anyhow!("No current view"))?;

                // For now, use a fixed viewport size - client will send actual size later
                let viewport_height = 24;
                let viewport_start = view.scroll_line();

                // Get render data and info
                let render_data_internal = editor.get_render_data(viewport_start, viewport_height);
                let info_internal = editor.get_info();

                // Convert to protocol types
                let render_data = RenderData {
                    lines: render_data_internal.lines,
                    cursor: Position {
                        line: render_data_internal.cursor.line,
                        column: render_data_internal.cursor.column,
                    },
                    viewport_start: render_data_internal.viewport_start,
                    viewport_height: render_data_internal.viewport_height,
                };

                let info = EditorInfo {
                    cursor: Position {
                        line: info_internal.cursor.line,
                        column: info_internal.cursor.column,
                    },
                    filepath: info_internal.filepath,
                    name: info_internal.name,
                    modified: info_internal.modified,
                    line_count: info_internal.line_count,
                    char_count: info_internal.char_count,
                };

                Message::StateUpdate {
                    events: protocol_events,
                    render_data,
                    info,
                }
            }
            Message::ClientStart => {
                // Get current view for viewport calculations
                let editor = editor.read().await;
                let view = editor
                    .get_current_view()
                    .ok_or_else(|| anyhow::anyhow!("No current view"))?;

                // For now, use a fixed viewport size - client will send actual size later
                let viewport_height = 24;
                let viewport_start = view.scroll_line();

                // Get render data and info
                let render_data_internal = editor.get_render_data(viewport_start, viewport_height);
                let info_internal = editor.get_info();

                // Convert to protocol types
                let render_data = RenderData {
                    lines: render_data_internal.lines,
                    cursor: Position {
                        line: render_data_internal.cursor.line,
                        column: render_data_internal.cursor.column,
                    },
                    viewport_start: render_data_internal.viewport_start,
                    viewport_height: render_data_internal.viewport_height,
                };

                let info = EditorInfo {
                    cursor: Position {
                        line: info_internal.cursor.line,
                        column: info_internal.cursor.column,
                    },
                    filepath: info_internal.filepath,
                    name: info_internal.name,
                    modified: info_internal.modified,
                    line_count: info_internal.line_count,
                    char_count: info_internal.char_count,
                };

                Message::StateUpdate {
                    events: vec![],
                    render_data,
                    info,
                }
            }
            Message::ServerStatusCheck => Message::ServerStatusOk,
            _ => continue,
        };

        // Send response
        let response_bytes = response.encode()?;
        conn.write_all(&response_bytes).await?;
    }
}
