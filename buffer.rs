use crate::location::Position;
use ropey::Rope;
use std::path::PathBuf;
use thiserror::Error;
use tokio::fs;

type BufferResult<T> = Result<T, BufferError>;

#[derive(Debug, Error)]
pub enum BufferError {
    #[error("Encountered IO error")]
    IoError(#[from] std::io::Error),
    #[error("No buffer path set")]
    NoPathSet,
}

/// A text buffer using the Rope data structure
#[derive(Debug)]
pub struct Buffer {
    rope: Rope,
    filepath: Option<PathBuf>,
    name: Option<String>,
    modified: bool,
    cursor_pos: usize,
}

impl Buffer {
    /// Create a new empty buffer
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            filepath: None,
            name: None,
            modified: false,
            cursor_pos: 0,
        }
    }

    /// Create a buffer from a file
    pub async fn from_file(path: &str) -> BufferResult<Self> {
        let filepath = PathBuf::from(path);
        let file_name = filepath.file_name();
        let rope = Rope::from_str(&fs::read_to_string(&filepath).await?);

        Ok(Self {
            rope,
            filepath: Some(filepath.clone()),
            // TODO make sure we're actually opening a file
            name: match file_name {
                Some(name) => Some(name.to_str().unwrap_or("*new*").to_string()),
                None => Some("*new*".into()),
            },
            modified: false,
            cursor_pos: 0,
        })
    }

    /// Save the buffer to its file
    pub async fn save(&mut self) -> BufferResult<()> {
        if let Some(ref path) = self.filepath {
            // Convert rope to string in memory (fast)
            let content = self.rope.to_string();

            // Async write - doesn't block the runtime
            fs::write(path, content).await?;
            self.modified = false;
            Ok(())
        } else {
            Err(BufferError::NoPathSet)
        }
    }

    /// Save the buffer to a specific file
    pub async fn save_as(&mut self, path: &str) -> BufferResult<()> {
        let filepath = PathBuf::from(path);

        // Convert rope to string in memory (fast)
        let content = self.rope.to_string();

        // Async write - doesn't block the runtime
        fs::write(&filepath, content).await?;
        self.filepath = Some(filepath);
        self.modified = false;
        Ok(())
    }

    /// Insert a character at the cursor position
    pub fn insert_char(&mut self, ch: char) {
        self.rope.insert_char(self.cursor_pos, ch);
        self.cursor_pos += 1; // Characters are always 1 char, not bytes
        self.modified = true;
    }

    /// Insert a string at the cursor position
    pub fn insert_string(&mut self, s: &str) {
        self.rope.insert(self.cursor_pos, s);
        self.cursor_pos += s.chars().count(); // Count characters, not bytes
        self.modified = true;
    }

    /// Delete the character at the cursor position
    pub fn delete_char(&mut self) -> bool {
        if self.cursor_pos < self.rope.len_chars() {
            let char_end = self.rope.char_to_byte(self.cursor_pos + 1);
            let char_start = self.rope.char_to_byte(self.cursor_pos);
            self.rope.remove(char_start..char_end);
            self.modified = true;
            true
        } else {
            false
        }
    }

    /// Delete the character before the cursor (backspace)
    pub fn backspace(&mut self) -> bool {
        if self.cursor_pos > 0 {
            let prev_pos = self.cursor_pos - 1;
            let char_start = self.rope.char_to_byte(prev_pos);
            let char_end = self.rope.char_to_byte(self.cursor_pos);
            self.rope.remove(char_start..char_end);
            self.cursor_pos = prev_pos;
            self.modified = true;
            true
        } else {
            false
        }
    }

    /// Move cursor left
    pub fn move_left(&mut self) -> bool {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            true
        } else {
            false
        }
    }

    /// Move cursor right
    pub fn move_right(&mut self) -> bool {
        if self.cursor_pos < self.rope.len_chars() {
            self.cursor_pos += 1;
            true
        } else {
            false
        }
    }

    /// Move cursor up
    pub fn move_up(&mut self) -> bool {
        let pos = self.cursor_to_position();
        if pos.line > 0 {
            let target_line = pos.line - 1;
            let line_len = self.rope.line(target_line).len_chars();
            let target_col = pos.column.min(line_len.saturating_sub(1));
            self.cursor_pos = self.position_to_cursor(Position::new(target_line, target_col));
            true
        } else {
            false
        }
    }

    /// Move cursor down
    pub fn move_down(&mut self) -> bool {
        let pos = self.cursor_to_position();
        let last_line = self.rope.len_lines().saturating_sub(1);

        if pos.line < last_line {
            let target_line = pos.line + 1;
            let line_len = self.rope.line(target_line).len_chars();
            let target_col = pos.column.min(line_len.saturating_sub(1));
            self.cursor_pos = self.position_to_cursor(Position::new(target_line, target_col));
            true
        } else {
            false
        }
    }

    /// Move cursor to start of line
    pub fn move_to_line_start(&mut self) {
        let line_idx = self.rope.char_to_line(self.cursor_pos);
        self.cursor_pos = self.rope.line_to_char(line_idx);
    }

    /// Move cursor to end of line
    pub fn move_to_line_end(&mut self) {
        let line_idx = self.rope.char_to_line(self.cursor_pos);
        let line = self.rope.line(line_idx);
        let line_start = self.rope.line_to_char(line_idx);
        let line_len = line.len_chars();

        // Don't include the newline character
        self.cursor_pos = line_start
            + line_len.saturating_sub(if line_len > 0 && line.char(line_len - 1) == '\n' {
                1
            } else {
                0
            });
    }

    /// Move cursor to start of document
    pub fn move_to_start(&mut self) {
        self.cursor_pos = 0;
    }

    /// Move cursor to end of document
    pub fn move_to_end(&mut self) {
        self.cursor_pos = self.rope.len_chars();
    }

    /// Convert cursor position to line/column
    pub fn cursor_to_position(&self) -> Position {
        let line = self.rope.char_to_line(self.cursor_pos);
        let line_start = self.rope.line_to_char(line);
        let column = self.cursor_pos - line_start;
        Position::new(line, column)
    }

    /// Convert line/column to cursor position
    pub fn position_to_cursor(&self, pos: Position) -> usize {
        let line_start = self.rope.line_to_char(pos.line);
        let line = self.rope.line(pos.line);
        let max_col = line.len_chars().saturating_sub(1);
        line_start + pos.column.min(max_col)
    }

    /// Get lines for display
    pub fn get_lines(&self, start_line: usize, count: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let end_line = (start_line + count).min(self.rope.len_lines());

        for i in start_line..end_line {
            let line = self.rope.line(i);
            let mut line_string = line.to_string();
            // Remove trailing newline for display
            if line_string.ends_with('\n') {
                line_string.pop();
            }
            lines.push(line_string);
        }

        lines
    }

    /// Get buffer statistics
    pub fn stats(&self) -> (usize, usize) {
        (self.rope.len_lines(), self.rope.len_chars())
    }

    /// Check if buffer is modified
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Get filepath
    pub fn filepath(&self) -> Option<&PathBuf> {
        self.filepath.as_ref()
    }

    /// Get the entire content as a string
    pub fn to_string(&self) -> String {
        self.rope.to_string()
    }

    /// Get cursor position
    pub fn cursor(&self) -> usize {
        self.cursor_pos
    }
}
