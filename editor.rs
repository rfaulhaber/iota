use crate::{buffer::Buffer, location::Position};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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

#[derive(Debug, Clone)]
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
    pub async fn with_file(path: &str) -> Result<Self, crate::buffer::BufferError> {
        let buffer = Buffer::from_file(path).await?;
        Ok(Self {
            buffers: vec![buffer],
            current_buffer: 0,
        })
    }

    /// Process a key event and return events for the frontend to handle
    pub async fn process_key_event(&mut self, key: KeyEvent) -> Vec<EditorEvent> {
        let mut events = Vec::new();

        // Convert key event to editor command
        let command = match (key.code, key.modifiers) {
            // System commands that generate events
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                events.push(EditorEvent::Shutdown);
                return events;
            }

            // File operations
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => Some(EditorInput::Save),
            (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
                Some(EditorInput::SaveAs("untitled.txt".to_string()))
            }

            // Buffer management
            (KeyCode::Char('n'), KeyModifiers::CONTROL) => Some(EditorInput::NewBuffer),
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => Some(EditorInput::DeleteBuffer),
            (KeyCode::Char('h'), KeyModifiers::CONTROL) => Some(EditorInput::PreviousBuffer),
            (KeyCode::Char('l'), KeyModifiers::CONTROL) => Some(EditorInput::NextBuffer),

            // Navigation
            (KeyCode::Left, _) => Some(EditorInput::MoveLeft(1)),
            (KeyCode::Right, _) => Some(EditorInput::MoveRight(1)),
            (KeyCode::Up, _) => Some(EditorInput::MoveUp(1)),
            (KeyCode::Down, _) => Some(EditorInput::MoveDown(1)),

            // Editing
            (KeyCode::Backspace, _) => Some(EditorInput::Backspace),
            (KeyCode::Delete, _) => Some(EditorInput::DeleteChar),
            (KeyCode::Enter, _) => Some(EditorInput::InsertNewLine),
            (KeyCode::Tab, _) => Some(EditorInput::InsertString("    ".to_string())),

            // Character input
            (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                Some(EditorInput::InsertChar(c))
            }

            _ => None,
        };

        // Execute the command if one was generated
        if let Some(cmd) = command {
            let cmd_events = self.execute_command(cmd).await;
            events.extend(cmd_events);
        }

        events
    }

    async fn execute_command(&mut self, command: EditorInput) -> Vec<EditorEvent> {
        let mut events = Vec::new();
        match command {
            // Buffer management commands
            EditorInput::NewBuffer => {
                self.buffers.push(Buffer::new());
                self.current_buffer = self.buffers.len() - 1;
                events.push(EditorEvent::Redraw);
            }
            EditorInput::DeleteBuffer => {
                // Only delete if there's more than one buffer
                if self.buffers.len() > 1 {
                    self.buffers.remove(self.current_buffer);
                    // Adjust current_buffer index if needed
                    if self.current_buffer >= self.buffers.len() {
                        self.current_buffer = self.buffers.len() - 1;
                    }
                    events.push(EditorEvent::Redraw);
                }
            }
            EditorInput::NextBuffer => {
                if !self.buffers.is_empty() {
                    self.current_buffer = (self.current_buffer + 1) % self.buffers.len();
                    events.push(EditorEvent::Redraw);
                }
            }
            EditorInput::PreviousBuffer => {
                if !self.buffers.is_empty() {
                    self.current_buffer = if self.current_buffer == 0 {
                        self.buffers.len() - 1
                    } else {
                        self.current_buffer - 1
                    };
                    events.push(EditorEvent::Redraw);
                }
            }

            // File I/O commands
            EditorInput::OpenFile(path) => {
                match Buffer::from_file(&path).await {
                    Ok(buffer) => {
                        self.buffers.push(buffer);
                        self.current_buffer = self.buffers.len() - 1;
                        events.push(EditorEvent::Info(format!("Opened {}", path)));
                        events.push(EditorEvent::Redraw);
                    }
                    Err(e) => {
                        log::error!("Failed to open file {}: {:?}", path, e);
                        events.push(EditorEvent::Error(format!("Failed to open {}: {}", path, e)));
                    }
                }
            }
            EditorInput::Save => {
                match self.buffers[self.current_buffer].save().await {
                    Ok(_) => {
                        events.push(EditorEvent::Info("Saved".to_string()));
                    }
                    Err(e) => {
                        log::error!("Failed to save buffer: {:?}", e);
                        events.push(EditorEvent::Error(format!("Save failed: {}", e)));
                    }
                }
            }
            EditorInput::SaveAs(path) => {
                match self.buffers[self.current_buffer].save_as(&path).await {
                    Ok(_) => {
                        events.push(EditorEvent::Info(format!("Saved as {}", path)));
                    }
                    Err(e) => {
                        log::error!("Failed to save buffer as {}: {:?}", path, e);
                        events.push(EditorEvent::Error(format!("Save as {} failed: {}", path, e)));
                    }
                }
            }

            // All other commands operate on the current buffer
            _ => {
                let current_buffer = &mut self.buffers[self.current_buffer];
                match command {
                    EditorInput::InsertChar(c) => {
                        current_buffer.insert_char(c);
                        events.push(EditorEvent::Redraw);
                    }
                    EditorInput::InsertString(s) => {
                        current_buffer.insert_string(&s);
                        events.push(EditorEvent::Redraw);
                    }
                    EditorInput::DeleteChar => {
                        let _ = current_buffer.delete_char();
                        events.push(EditorEvent::Redraw);
                    }
                    EditorInput::DeleteRange(_) => todo!(),
                    EditorInput::Undo => todo!(),
                    EditorInput::Redo => todo!(),
                    EditorInput::MoveUp(_) => {
                        let _ = current_buffer.move_up();
                        events.push(EditorEvent::Redraw);
                    }
                    EditorInput::MoveDown(_) => {
                        let _ = current_buffer.move_down();
                        events.push(EditorEvent::Redraw);
                    }
                    EditorInput::MoveLeft(_) => {
                        let _ = current_buffer.move_left();
                        events.push(EditorEvent::Redraw);
                    }
                    EditorInput::MoveRight(_) => {
                        let _ = current_buffer.move_right();
                        events.push(EditorEvent::Redraw);
                    }
                    EditorInput::Backspace => {
                        let _ = current_buffer.move_left();
                        current_buffer.delete_char();
                        events.push(EditorEvent::Redraw);
                    }
                    EditorInput::InsertNewLine => {
                        let _ = current_buffer.insert_char('\n');
                        events.push(EditorEvent::Redraw);
                    }

                    _ => {}
                }
            }
        }

        events
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
