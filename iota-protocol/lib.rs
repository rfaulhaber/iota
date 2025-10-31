use bincode::{Decode, Encode};
use iota_input::EditorKey;
use std::path::PathBuf;

/// Position in the buffer (line and column coordinates)
#[derive(Debug, Clone, Encode, Decode)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

/// Editor events sent from server to client
#[derive(Debug, Clone, Encode, Decode)]
pub enum EditorEvent {
    /// Request the frontend to shutdown
    Shutdown,
    /// Request the frontend to redraw
    Redraw,
    /// Display an error message to the user
    Error(String),
    /// Display an info message to the user
    Info(String),
}

/// Rendering data for the frontend - a snapshot of buffer content prepared for display
#[derive(Debug, Clone, Encode, Decode)]
pub struct RenderData {
    pub lines: Vec<String>,
    pub cursor: Position,
    pub viewport_start: usize,
    pub viewport_height: usize,
}

/// Editor metadata - cursor position, file info, buffer statistics
#[derive(Debug, Clone, Encode, Decode)]
pub struct EditorInfo {
    pub cursor: Position,
    pub filepath: Option<String>,
    pub name: Option<String>,
    pub modified: bool,
    pub line_count: usize,
    pub char_count: usize,
}

/// Messages sent between client and server
#[derive(Debug, Encode, Decode)]
pub enum Message {
    /// Client sends a key press to the server
    KeyPress {
        key: EditorKey,
    },
    /// Server responds with editor events and updated state
    StateUpdate {
        events: Vec<EditorEvent>,
        render_data: RenderData,
        info: EditorInfo,
    },
}

impl Message {
    /// Encode a message with a length prefix for socket transmission
    pub fn encode(&self) -> Result<Vec<u8>, bincode::error::EncodeError> {
        let config = bincode::config::standard();
        let encoded = bincode::encode_to_vec(self, config)?;
        let len = encoded.len() as u32;
        let mut result = Vec::with_capacity(4 + encoded.len());
        result.extend_from_slice(&len.to_be_bytes());
        result.extend_from_slice(&encoded);
        Ok(result)
    }

    /// Decode a message from bytes (without length prefix)
    pub fn decode(bytes: &[u8]) -> Result<Self, bincode::error::DecodeError> {
        let config = bincode::config::standard();
        let (message, _) = bincode::decode_from_slice(bytes, config)?;
        Ok(message)
    }
}

/// Get the socket path for the iota server.
///
/// Resolution order:
/// 1. `$IOTA_SERVER_SOCKET` - explicit override
/// 2. `$XDG_RUNTIME_DIR/iota-server.sock` - XDG standard location
/// 3. `/tmp/iota-server.sock` - fallback for all platforms
pub fn get_socket_path() -> PathBuf {
    // Check for explicit override
    if let Ok(socket_path) = std::env::var("IOTA_SERVER_SOCKET") {
        return PathBuf::from(socket_path);
    }

    // Check for XDG_RUNTIME_DIR (common on Linux)
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        let mut path = PathBuf::from(runtime_dir);
        path.push("iota-server.sock");
        return path;
    }

    // Cross-platform fallback to /tmp
    PathBuf::from("/tmp/iota-server.sock")
}
