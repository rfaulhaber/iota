use crate::{buffer::Buffer, location::Position};

#[derive(Debug)]
pub enum Command {
    InsertChar(char),
    InsertString(String),
    InsertNewLine,

    DeleteChar,
    DeleteRange(crate::location::Range),

    Backspace,

    Undo,
    Redo,

    Save,
    SaveAs(String),

    MoveUp(usize),
    MoveDown(usize),
    MoveLeft(usize),
    MoveRight(usize),
}

#[derive(Debug)]
pub struct Editor {
    buffers: Vec<Buffer>,
    current_buffer: usize,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            buffers: vec![Buffer::new()],
            current_buffer: 0,
        }
    }

    pub fn execute_command(&mut self, command: Command) {
        let current_buffer = &mut self.buffers[self.current_buffer];

        match command {
            Command::InsertChar(c) => current_buffer.insert_char(c),
            Command::InsertString(s) => current_buffer.insert_string(&s),
            Command::DeleteChar => {
                let _ = current_buffer.delete_char();
            }
            Command::DeleteRange(_) => todo!(),
            Command::Undo => todo!(),
            Command::Redo => todo!(),
            Command::MoveUp(_) => {
                let _ = current_buffer.move_up();
            }
            Command::MoveDown(_) => {
                let _ = current_buffer.move_down();
            }
            Command::MoveLeft(_) => {
                let _ = current_buffer.move_left();
            }
            Command::MoveRight(_) => {
                let _ = current_buffer.move_right();
            }
            Command::Backspace => {
                let _ = current_buffer.move_left();
                current_buffer.delete_char();
            }
            Command::InsertNewLine => {
                let _ = current_buffer.insert_char('\n');
            }

            _ => {}
        }
    }

    /// Get a view of the buffer for rendering
    pub fn get_buffer_view(&self, viewport_start: usize, viewport_height: usize) -> BufferView {
        let buffer = &self.buffers[self.current_buffer];
        let lines = buffer.get_lines(viewport_start, viewport_height);

        BufferView {
            lines,
            cursor: buffer.cursor_to_position(),
            viewport_start,
            viewport_height,
        }
    }

    pub fn get_info(&self) -> EditorInfo {
        let buffer = &self.buffers[self.current_buffer];
        let (line_count, char_count) = buffer.stats();

        EditorInfo {
            cursor: buffer.cursor_to_position(),
            filepath: buffer
                .filepath()
                .and_then(|p| p.to_str())
                .map(|s| s.to_string()),
            modified: buffer.is_modified(),
            line_count,
            char_count,
        }
    }
}

/// Information about the current editor state
#[derive(Debug)]
pub struct EditorInfo {
    pub cursor: Position,
    pub filepath: Option<String>,
    pub modified: bool,
    pub line_count: usize,
    pub char_count: usize,
}

/// View of the buffer for rendering
#[derive(Debug)]
pub struct BufferView {
    pub lines: Vec<String>,
    pub cursor: Position,
    pub viewport_start: usize,
    pub viewport_height: usize,
}
