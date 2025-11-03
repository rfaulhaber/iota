use iota_core::location::Position;
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
}

impl Buffer {
    /// Create a new empty buffer
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            filepath: None,
            name: None,
            modified: false,
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

    /// Insert a character at the given cursor position, returns new cursor position
    pub fn insert_char(&mut self, cursor: usize, ch: char) -> usize {
        self.rope.insert_char(cursor, ch);
        self.modified = true;
        cursor + 1 // Characters are always 1 char, not bytes
    }

    /// Insert a string at the given cursor position, returns new cursor position
    pub fn insert_string(&mut self, cursor: usize, s: &str) -> usize {
        self.rope.insert(cursor, s);
        self.modified = true;
        cursor + s.chars().count() // Count characters, not bytes
    }

    /// Delete the character at the cursor position, returns (success, new_cursor)
    pub fn delete_char(&mut self, cursor: usize) -> (bool, usize) {
        if cursor < self.rope.len_chars() {
            let char_end = self.rope.char_to_byte(cursor + 1);
            let char_start = self.rope.char_to_byte(cursor);
            self.rope.remove(char_start..char_end);
            self.modified = true;
            (true, cursor) // Cursor stays in same position after delete
        } else {
            (false, cursor)
        }
    }

    /// Delete the character before the cursor (backspace), returns (success, new_cursor)
    pub fn backspace(&mut self, cursor: usize) -> (bool, usize) {
        if cursor > 0 {
            let prev_pos = cursor - 1;
            let char_start = self.rope.char_to_byte(prev_pos);
            let char_end = self.rope.char_to_byte(cursor);
            self.rope.remove(char_start..char_end);
            self.modified = true;
            (true, prev_pos)
        } else {
            (false, cursor)
        }
    }

    /// Move cursor left, returns Some(new_cursor) if successful, None otherwise
    pub fn move_left(&self, cursor: usize) -> Option<usize> {
        if cursor > 0 { Some(cursor - 1) } else { None }
    }

    /// Move cursor right, returns Some(new_cursor) if successful, None otherwise
    pub fn move_right(&self, cursor: usize) -> Option<usize> {
        if cursor < self.rope.len_chars() {
            Some(cursor + 1)
        } else {
            None
        }
    }

    /// Move cursor up, returns Some(new_cursor) if successful, None otherwise
    pub fn move_up(&self, cursor: usize) -> Option<usize> {
        let pos = self.cursor_to_position(cursor);
        if pos.line > 0 {
            let target_line = pos.line - 1;
            let line_len = self.rope.line(target_line).len_chars();
            let target_col = pos.column.min(line_len.saturating_sub(1));
            Some(self.position_to_cursor(Position::new(target_line, target_col)))
        } else {
            None
        }
    }

    /// Move cursor down, returns Some(new_cursor) if successful, None otherwise
    pub fn move_down(&self, cursor: usize) -> Option<usize> {
        let pos = self.cursor_to_position(cursor);
        let last_line = self.rope.len_lines().saturating_sub(1);

        if pos.line < last_line {
            let target_line = pos.line + 1;
            let line_len = self.rope.line(target_line).len_chars();
            let target_col = pos.column.min(line_len.saturating_sub(1));
            Some(self.position_to_cursor(Position::new(target_line, target_col)))
        } else {
            None
        }
    }

    /// Move cursor to start of line, returns new cursor position
    pub fn move_to_line_start(&self, cursor: usize) -> usize {
        let line_idx = self.rope.char_to_line(cursor);
        self.rope.line_to_char(line_idx)
    }

    /// Move cursor to end of line, returns new cursor position
    pub fn move_to_line_end(&self, cursor: usize) -> usize {
        let line_idx = self.rope.char_to_line(cursor);
        let line = self.rope.line(line_idx);
        let line_start = self.rope.line_to_char(line_idx);
        let line_len = line.len_chars();

        // Don't include the newline character
        line_start
            + line_len.saturating_sub(if line_len > 0 && line.char(line_len - 1) == '\n' {
                1
            } else {
                0
            })
    }

    /// Move cursor to start of document, returns new cursor position
    pub fn move_to_start(&self) -> usize {
        0
    }

    /// Move cursor to end of document, returns new cursor position
    pub fn move_to_end(&self) -> usize {
        self.rope.len_chars()
    }

    /// Convert cursor position to line/column
    pub fn cursor_to_position(&self, cursor: usize) -> Position {
        let line = self.rope.char_to_line(cursor);
        let line_start = self.rope.line_to_char(line);
        let column = cursor - line_start;
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

    /// Mark buffer as clean (not modified)
    /// Useful for discarding changes or after external save operations
    pub fn mark_clean(&mut self) {
        self.modified = false;
    }

    /// Get filepath
    pub fn filepath(&self) -> Option<&PathBuf> {
        self.filepath.as_ref()
    }

    /// Get the entire content as a string
    pub fn to_string(&self) -> String {
        self.rope.to_string()
    }

    /// Get buffer name
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Set buffer name
    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn test_new_buffer() {
        let buffer = Buffer::new();
        assert!(!buffer.is_modified());
        assert_eq!(buffer.to_string(), "");
        let (lines, chars) = buffer.stats();
        assert_eq!(lines, 1); // Empty rope has 1 line
        assert_eq!(chars, 0);
    }

    #[test]
    fn test_insert_char() {
        let mut buffer = Buffer::new();
        let new_pos = buffer.insert_char(0, 'H');
        assert_eq!(buffer.to_string(), "H");
        assert_eq!(buffer.cursor_to_position(new_pos).line, 0);
        assert_eq!(buffer.cursor_to_position(new_pos).column, 1);
        assert!(buffer.is_modified());

        let new_pos_i = buffer.insert_char(new_pos, 'i');
        assert_eq!(buffer.to_string(), "Hi");
        assert_eq!(new_pos_i, 2);
    }

    #[test]
    fn test_insert_string() {
        let mut buffer = Buffer::new();
        let pos = buffer.insert_string(0, "Hello");
        assert_eq!(buffer.to_string(), "Hello");
        assert_eq!(pos, 5);
        assert!(buffer.is_modified());

        let new_pos = buffer.insert_string(pos, " World");
        assert_eq!(buffer.to_string(), "Hello World");
        assert_eq!(new_pos, 11);
    }

    #[test]
    fn test_insert_multiline() {
        let mut buffer = Buffer::new();
        let pos = buffer.insert_string(0, "Line 1\nLine 2\nLine 3");
        assert_eq!(buffer.to_string(), "Line 1\nLine 2\nLine 3");
        let (lines, _) = buffer.stats();
        assert_eq!(lines, 3);
    }

    #[test]
    fn test_backspace() {
        let mut buffer = Buffer::new();
        let mut cursor = buffer.insert_string(0, "Hello");

        let (success, new_cursor) = buffer.backspace(cursor);

        // Backspace at end
        assert!(success);
        assert_eq!(buffer.to_string(), "Hell");
        assert_eq!(new_cursor, 4);
        cursor = new_cursor;

        // Multiple backspaces
        let (success2, cursor2) = buffer.backspace(cursor);
        assert!(success2);
        cursor = cursor2;

        let (success3, cursor3) = buffer.backspace(cursor);
        assert!(success3);
        assert_eq!(buffer.to_string(), "He");
        assert_eq!(cursor3, 2);
        cursor = cursor3;

        // Backspace at beginning
        cursor = buffer.move_to_start();
        let (success_start, cursor_start) = buffer.backspace(cursor);
        assert!(!success_start); // Can't backspace at start
        assert_eq!(buffer.to_string(), "He");
        assert_eq!(cursor_start, 0);
    }

    #[test]
    fn test_delete_char() {
        let mut buffer = Buffer::new();
        buffer.insert_string(0, "Hello");
        let mut cursor = buffer.move_to_start();

        // Delete at start
        let (success, new_cursor) = buffer.delete_char(cursor);
        assert!(success);
        assert_eq!(buffer.to_string(), "ello");
        assert_eq!(new_cursor, 0);
        cursor = new_cursor;

        // Delete in middle
        cursor = buffer.move_right(cursor).unwrap();
        let (success2, new_cursor2) = buffer.delete_char(cursor);
        assert!(success2);
        assert_eq!(buffer.to_string(), "elo");
        assert_eq!(new_cursor2, 1);
        cursor = new_cursor2;

        // Move to end and try to delete (should fail)
        cursor = buffer.move_to_end();
        let (success3, new_cursor3) = buffer.delete_char(cursor);
        assert!(!success3);
        assert_eq!(buffer.to_string(), "elo");
    }

    #[test]
    fn test_move_left_right() {
        let mut buffer = Buffer::new();
        let mut cursor = buffer.insert_string(0, "Hello");

        assert_eq!(cursor, 5);
        cursor = buffer.move_left(cursor).unwrap();
        assert_eq!(cursor, 4);
        cursor = buffer.move_left(cursor).unwrap();
        assert_eq!(cursor, 3);

        cursor = buffer.move_right(cursor).unwrap();
        assert_eq!(cursor, 4);
        cursor = buffer.move_right(cursor).unwrap();
        assert_eq!(cursor, 5);

        // Can't move right beyond end
        assert!(buffer.move_right(cursor).is_none());
        assert_eq!(cursor, 5);

        // Move to start
        cursor = buffer.move_to_start();
        assert_eq!(cursor, 0);

        // Can't move left beyond start
        assert!(buffer.move_left(cursor).is_none());
        assert_eq!(cursor, 0);
    }

    #[test]
    fn test_move_up_down() {
        let mut buffer = Buffer::new();
        let mut cursor = buffer.insert_string(0, "Line 1\nLine 2\nLine 3");

        // Cursor is at end of "Line 3"
        let pos = buffer.cursor_to_position(cursor);
        assert_eq!(pos.line, 2);
        assert_eq!(pos.column, 6);

        // Move up to line 2
        cursor = buffer.move_up(cursor).unwrap();
        let pos = buffer.cursor_to_position(cursor);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.column, 6);

        // Move up to line 1
        cursor = buffer.move_up(cursor).unwrap();
        let pos = buffer.cursor_to_position(cursor);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.column, 6);

        // Can't move up beyond first line
        assert!(buffer.move_up(cursor).is_none());

        // Move down to line 2
        cursor = buffer.move_down(cursor).unwrap();
        let pos = buffer.cursor_to_position(cursor);
        assert_eq!(pos.line, 1);

        // Move down to line 3
        cursor = buffer.move_down(cursor).unwrap();
        let pos = buffer.cursor_to_position(cursor);
        assert_eq!(pos.line, 2);

        // Can't move down beyond last line
        assert!(buffer.move_down(cursor).is_none());
    }

    #[test]
    fn test_move_up_down_with_varying_line_lengths() {
        let mut buffer = Buffer::new();
        buffer.insert_string(0, "Short\nThis is a longer line\nMed");
        let mut cursor = buffer.move_to_start();

        // Start at beginning of first line (column 0)
        // Move to end of first line
        cursor = buffer.move_to_line_end(cursor);
        let pos = buffer.cursor_to_position(cursor);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.column, 5); // "Short" has 5 chars

        // Move down - column should be capped to line length
        cursor = buffer.move_down(cursor).unwrap();
        let pos = buffer.cursor_to_position(cursor);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.column, 5); // Column preserved

        // Move to end of long line
        cursor = buffer.move_to_line_end(cursor);
        let pos = buffer.cursor_to_position(cursor);
        assert_eq!(pos.column, 21); // "This is a longer line" has 21 chars

        // Move down to shorter line - column should be capped
        cursor = buffer.move_down(cursor).unwrap();
        let pos = buffer.cursor_to_position(cursor);
        assert_eq!(pos.line, 2);
        assert_eq!(pos.column, 2); // "Med" has 3 chars, so max column is 2 (0-indexed)
    }

    #[test]
    fn test_move_to_line_start_end() {
        let mut buffer = Buffer::new();
        buffer.insert_string(0, "Hello World");

        // Move to middle
        let mut cursor = buffer.move_to_start();
        cursor = buffer.move_right(cursor).unwrap();
        cursor = buffer.move_right(cursor).unwrap();
        assert_eq!(cursor, 2);

        // Move to line end
        cursor = buffer.move_to_line_end(cursor);
        assert_eq!(cursor, 11);

        // Move to line start
        cursor = buffer.move_to_line_start(cursor);
        assert_eq!(cursor, 0);
    }

    #[test]
    fn test_move_to_line_end_with_newline() {
        let mut buffer = Buffer::new();
        buffer.insert_string(0, "Line 1\nLine 2");
        let mut cursor = buffer.move_to_start();

        // Move to end of first line (should not include newline)
        cursor = buffer.move_to_line_end(cursor);
        let pos = buffer.cursor_to_position(cursor);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.column, 6); // After "Line 1", before newline
    }

    #[test]
    fn test_move_to_start_end() {
        let mut buffer = Buffer::new();
        let cursor = buffer.insert_string(0, "Line 1\nLine 2\nLine 3");

        // Cursor starts at end
        let end_cursor = cursor;

        let cursor_at_start = buffer.move_to_start();
        assert_eq!(cursor_at_start, 0);

        let cursor_at_end = buffer.move_to_end();
        assert_eq!(cursor_at_end, end_cursor);
    }

    #[test]
    fn test_cursor_to_position() {
        let mut buffer = Buffer::new();
        buffer.insert_string(0, "Line 1\nLine 2\nLine 3");

        let mut cursor = buffer.move_to_start();
        let pos = buffer.cursor_to_position(cursor);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.column, 0);

        // Move to middle of first line
        cursor = buffer.move_right(cursor).unwrap();
        cursor = buffer.move_right(cursor).unwrap();
        cursor = buffer.move_right(cursor).unwrap();
        let pos = buffer.cursor_to_position(cursor);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.column, 3);

        // Move to start of second line
        cursor = buffer.move_to_start();
        cursor = buffer.move_down(cursor).unwrap();
        cursor = buffer.move_to_line_start(cursor);
        let pos = buffer.cursor_to_position(cursor);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.column, 0);
    }

    #[test]
    fn test_position_to_cursor() {
        let mut buffer = Buffer::new();
        buffer.insert_string(0, "Line 1\nLine 2\nLine 3");

        let cursor = buffer.position_to_cursor(Position::new(0, 0));
        assert_eq!(cursor, 0);

        let cursor = buffer.position_to_cursor(Position::new(0, 3));
        assert_eq!(cursor, 3);

        let cursor = buffer.position_to_cursor(Position::new(1, 0));
        assert_eq!(cursor, 7); // After "Line 1\n"

        let cursor = buffer.position_to_cursor(Position::new(2, 2));
        assert_eq!(cursor, 16); // "Line 1\n" (7) + "Line 2\n" (7) + "Li" (2)
    }

    #[test]
    fn test_position_to_cursor_beyond_line_length() {
        let mut buffer = Buffer::new();
        buffer.insert_string(0, "Hi\nLonger line");

        // Try to position beyond line length - should clamp
        let cursor = buffer.position_to_cursor(Position::new(0, 100));
        let pos = buffer.cursor_to_position(cursor);
        assert_eq!(pos.line, 0);
        assert!(pos.column <= 2); // "Hi" is only 2 chars
    }

    #[test]
    fn test_get_lines() {
        let mut buffer = Buffer::new();
        buffer.insert_string(0, "Line 1\nLine 2\nLine 3\nLine 4");

        let lines = buffer.get_lines(0, 2);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "Line 1");
        assert_eq!(lines[1], "Line 2");

        let lines = buffer.get_lines(1, 2);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "Line 2");
        assert_eq!(lines[1], "Line 3");

        // Request beyond available lines
        let lines = buffer.get_lines(2, 10);
        assert_eq!(lines.len(), 2); // Only lines 3 and 4 exist
        assert_eq!(lines[0], "Line 3");
        assert_eq!(lines[1], "Line 4");
    }

    #[test]
    fn test_stats() {
        let buffer = Buffer::new();
        let (lines, chars) = buffer.stats();
        assert_eq!(lines, 1);
        assert_eq!(chars, 0);

        let mut buffer = Buffer::new();
        buffer.insert_string(0, "Hello");
        let (lines, chars) = buffer.stats();
        assert_eq!(lines, 1);
        assert_eq!(chars, 5);

        let mut buffer = Buffer::new();
        buffer.insert_string(0, "Line 1\nLine 2\nLine 3");
        let (lines, chars) = buffer.stats();
        assert_eq!(lines, 3);
        assert_eq!(chars, 20); // Including newlines
    }

    #[test]
    fn test_modified_flag() {
        let mut buffer = Buffer::new();
        assert!(!buffer.is_modified());

        buffer.insert_char(0, 'a');
        assert!(buffer.is_modified());

        // Note: The modified flag is not cleared by operations,
        // only by save/save_as operations
    }

    #[test]
    fn test_unicode_characters() {
        let mut buffer = Buffer::new();

        // Test inserting unicode characters
        let mut cursor = buffer.insert_string(0, "Hello 世界");
        assert_eq!(buffer.to_string(), "Hello 世界");
        assert_eq!(cursor, 8); // 6 ASCII + 2 CJK = 8 chars

        // Test moving through unicode text
        cursor = buffer.move_left(cursor).unwrap();
        assert_eq!(cursor, 7);
        cursor = buffer.move_left(cursor).unwrap();
        assert_eq!(cursor, 6);

        // Test backspace with unicode
        let (success, new_cursor) = buffer.backspace(cursor);
        assert!(success);
        assert_eq!(buffer.to_string(), "Hello世界");
        assert_eq!(new_cursor, 5);
        cursor = new_cursor;

        // Test inserting emoji at start of buffer
        cursor = buffer.move_to_start();
        cursor = buffer.insert_char(cursor, '🦀');
        assert_eq!(buffer.to_string(), "🦀Hello世界");

        // Test character count is correct
        let (_, chars) = buffer.stats();
        assert_eq!(chars, 8); // emoji + "Hello" (5 chars) + 2 CJK
    }

    #[test]
    fn test_empty_buffer_operations() {
        let mut buffer = Buffer::new();
        let cursor = 0;

        // Movement on empty buffer
        assert!(buffer.move_left(cursor).is_none());
        assert!(buffer.move_right(cursor).is_none());
        assert!(buffer.move_up(cursor).is_none());
        assert!(buffer.move_down(cursor).is_none());

        // Deletion on empty buffer
        let (backspace_success, _) = buffer.backspace(cursor);
        assert!(!backspace_success);
        let (delete_success, _) = buffer.delete_char(cursor);
        assert!(!delete_success);

        // Position at start
        let pos = buffer.cursor_to_position(cursor);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.column, 0);
    }

    #[tokio::test]
    async fn test_save_without_path() {
        let mut buffer = Buffer::new();
        buffer.insert_string(0, "Test content");

        let result = buffer.save().await;
        assert!(result.is_err());
        match result {
            Err(BufferError::NoPathSet) => {}
            _ => panic!("Expected NoPathSet error"),
        }
    }

    #[tokio::test]
    async fn test_save_as_and_save() {
        let mut buffer = Buffer::new();
        let cursor = buffer.insert_string(0, "Test content");

        // Save to a temp file
        let temp_path = make_tmp_file_path("/tmp/iota_test_from_file.txt");
        buffer
            .save_as(temp_path.to_str().unwrap())
            .await
            .expect("save_as failed");

        assert!(!buffer.is_modified());
        assert_eq!(buffer.filepath(), Some(&PathBuf::from(temp_path.clone())));

        // Verify file was written
        let content = tokio::fs::read_to_string(temp_path.clone())
            .await
            .expect("read failed");
        assert_eq!(content, "Test content");

        // Modify and save again
        buffer.insert_string(cursor, "\nMore content");
        assert!(buffer.is_modified());
        buffer.save().await.expect("save failed");
        assert!(!buffer.is_modified());

        let content = tokio::fs::read_to_string(temp_path.clone())
            .await
            .expect("read failed");
        assert_eq!(content, "Test content\nMore content");

        // Cleanup
        let _ = tokio::fs::remove_file(temp_path.clone()).await;
    }

    #[tokio::test]
    async fn test_from_file() {
        // Create a temp file
        let temp_path = make_tmp_file_path("/tmp/iota_test_from_file.txt");
        let test_content = "Line 1\nLine 2\nLine 3";
        tokio::fs::write(temp_path.clone(), test_content)
            .await
            .expect("write failed");

        // Load buffer from file
        let buffer = Buffer::from_file(temp_path.to_str().unwrap())
            .await
            .expect("from_file failed");

        assert_eq!(buffer.to_string(), test_content);
        assert!(!buffer.is_modified());
        assert_eq!(buffer.filepath(), Some(&PathBuf::from(temp_path.clone())));

        let (lines, _) = buffer.stats();
        assert_eq!(lines, 3);

        // Cleanup
        let _ = tokio::fs::remove_file(temp_path.clone()).await;
    }

    fn make_tmp_file_path(file_name: &str) -> PathBuf {
        let mut tmp_dir = std::env::temp_dir();
        tmp_dir.push(file_name);

        return tmp_dir;
    }
}
