use bincode::{Decode, Encode};
use iota_input::EditorKey;

#[derive(Debug, Encode, Decode)]
pub enum Message {
    Request { id: u64, key: EditorKey },
    Response { id: u64 },
    Notification { id: u64 },
}
