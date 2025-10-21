use crate::{buffer::Buffer, location::Position};

#[derive(Debug)]
pub enum EditorInput {
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
    OpenFile(String),

    MoveUp(usize),
    MoveDown(usize),
    MoveLeft(usize),
    MoveRight(usize),

    NewBuffer,
    DeleteBuffer,
    NextBuffer,
    PreviousBuffer,
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

    /// Create an editor with a file opened
    pub fn with_file(path: &str) -> Result<Self, crate::buffer::BufferError> {
        let buffer = Buffer::from_file(path)?;
        Ok(Self {
            buffers: vec![buffer],
            current_buffer: 0,
        })
    }

    pub fn execute_command(&mut self, command: EditorInput) {
        match command {
            // Buffer management commands
            EditorInput::NewBuffer => {
                self.buffers.push(Buffer::new());
                self.current_buffer = self.buffers.len() - 1;
            }
            EditorInput::DeleteBuffer => {
                // Only delete if there's more than one buffer
                if self.buffers.len() > 1 {
                    self.buffers.remove(self.current_buffer);
                    // Adjust current_buffer index if needed
                    if self.current_buffer >= self.buffers.len() {
                        self.current_buffer = self.buffers.len() - 1;
                    }
                }
            }
            EditorInput::NextBuffer => {
                if !self.buffers.is_empty() {
                    self.current_buffer = (self.current_buffer + 1) % self.buffers.len();
                }
            }
            EditorInput::PreviousBuffer => {
                if !self.buffers.is_empty() {
                    self.current_buffer = if self.current_buffer == 0 {
                        self.buffers.len() - 1
                    } else {
                        self.current_buffer - 1
                    };
                }
            }

            // File I/O commands
            EditorInput::OpenFile(path) => {
                match Buffer::from_file(&path) {
                    Ok(buffer) => {
                        self.buffers.push(buffer);
                        self.current_buffer = self.buffers.len() - 1;
                    }
                    Err(_) => {
                        // TODO: Handle error - for now just ignore
                    }
                }
            }
            EditorInput::Save => {
                let _ = self.buffers[self.current_buffer].save();
                // TODO: Handle errors
            }
            EditorInput::SaveAs(path) => {
                let _ = self.buffers[self.current_buffer].save_as(&path);
                // TODO: Handle errors
            }

            // All other commands operate on the current buffer
            _ => {
                let current_buffer = &mut self.buffers[self.current_buffer];
                match command {
                    EditorInput::InsertChar(c) => current_buffer.insert_char(c),
                    EditorInput::InsertString(s) => current_buffer.insert_string(&s),
                    EditorInput::DeleteChar => {
                        let _ = current_buffer.delete_char();
                    }
                    EditorInput::DeleteRange(_) => todo!(),
                    EditorInput::Undo => todo!(),
                    EditorInput::Redo => todo!(),
                    EditorInput::MoveUp(_) => {
                        let _ = current_buffer.move_up();
                    }
                    EditorInput::MoveDown(_) => {
                        let _ = current_buffer.move_down();
                    }
                    EditorInput::MoveLeft(_) => {
                        let _ = current_buffer.move_left();
                    }
                    EditorInput::MoveRight(_) => {
                        let _ = current_buffer.move_right();
                    }
                    EditorInput::Backspace => {
                        let _ = current_buffer.move_left();
                        current_buffer.delete_char();
                    }
                    EditorInput::InsertNewLine => {
                        let _ = current_buffer.insert_char('\n');
                    }

                    _ => {}
                }
            }
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
            buffer_text: buffer.to_string(),
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
    pub buffer_text: String,
}
