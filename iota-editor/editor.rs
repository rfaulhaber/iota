use crate::{
    buffer::Buffer,
    view::{View, ViewId},
};
use iota_core::location::{self, Position};
use iota_input::{EditorKey, KeyCode};
use std::collections::HashMap;

/// Stable identifier for buffers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferId(usize);

impl BufferId {
    pub(crate) fn new(id: usize) -> Self {
        Self(id)
    }
}

#[derive(Debug)]
pub enum EditorInput {
    InsertChar(char),
    InsertString(String),
    InsertNewLine,

    DeleteChar,
    DeleteRange(location::Range),

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
    /// All buffers managed by the editor
    buffers: HashMap<BufferId, Buffer>,
    /// All views into buffers (each view has its own cursor/viewport)
    views: HashMap<ViewId, View>,
    /// Order of views for cycling through them
    view_order: Vec<ViewId>,
    /// The currently active view
    current_view: ViewId,
    /// Next buffer ID to allocate
    next_buffer_id: usize,
    /// Next view ID to allocate
    next_view_id: usize,
}

impl Editor {
    pub fn new() -> Self {
        let buffer_id = BufferId::new(0);
        let view_id = ViewId::new(0);

        let mut buffers = HashMap::new();
        let mut views = HashMap::new();

        buffers.insert(buffer_id, Buffer::new());
        views.insert(view_id, View::new(buffer_id));

        Self {
            buffers,
            views,
            view_order: vec![view_id],
            current_view: view_id,
            next_buffer_id: 1,
            next_view_id: 1,
        }
    }

    /// Create an editor with a file opened
    pub async fn with_file(path: &str) -> Result<Self, crate::buffer::BufferError> {
        let buffer_id = BufferId::new(0);
        let view_id = ViewId::new(0);

        let buffer = Buffer::from_file(path).await?;
        let mut buffers = HashMap::new();
        let mut views = HashMap::new();

        buffers.insert(buffer_id, buffer);
        views.insert(view_id, View::new(buffer_id));

        Ok(Self {
            buffers,
            views,
            view_order: vec![view_id],
            current_view: view_id,
            next_buffer_id: 1,
            next_view_id: 1,
        })
    }

    /// Create a new buffer with a view and switch to it
    /// Returns the ViewId of the newly created view
    fn create_buffer_with_view(&mut self, buffer: Buffer) -> ViewId {
        let buffer_id = BufferId::new(self.next_buffer_id);
        let view_id = ViewId::new(self.next_view_id);
        self.next_buffer_id += 1;
        self.next_view_id += 1;

        self.buffers.insert(buffer_id, buffer);
        self.views.insert(view_id, View::new(buffer_id));
        self.view_order.push(view_id);
        self.current_view = view_id;

        view_id
    }

    /// Switch to the next view in the view order (wraps around)
    fn next_view(&mut self) {
        if let Some(current_idx) = self
            .view_order
            .iter()
            .position(|&id| id == self.current_view)
        {
            let next_idx = (current_idx + 1) % self.view_order.len();
            self.current_view = self.view_order[next_idx];
        }
    }

    /// Switch to the previous view in the view order (wraps around)
    fn prev_view(&mut self) {
        if let Some(current_idx) = self
            .view_order
            .iter()
            .position(|&id| id == self.current_view)
        {
            let prev_idx = if current_idx == 0 {
                self.view_order.len() - 1
            } else {
                current_idx - 1
            };
            self.current_view = self.view_order[prev_idx];
        }
    }

    /// Process a key input and return events for the frontend to handle
    pub async fn process_key(&mut self, key: EditorKey) -> Vec<EditorEvent> {
        let mut events = Vec::new();

        // Convert key input to editor command
        let command = match (key.code, key.modifiers) {
            // System commands
            (KeyCode::Char('c'), m) if m.ctrl && !m.alt && !m.meta => {
                events.push(EditorEvent::Shutdown);
                return events;
            }
            (KeyCode::Char('r'), m) if m.ctrl && !m.alt && !m.meta => {
                events.push(EditorEvent::Redraw);
                return events;
            }

            // File operations
            (KeyCode::Char('s'), m) if m.ctrl && !m.alt && !m.meta => Some(EditorInput::Save),
            (KeyCode::Char('w'), m) if m.ctrl && !m.alt && !m.meta => {
                Some(EditorInput::SaveAs("untitled.txt".to_string()))
            }

            // Buffer management
            (KeyCode::Char('n'), m) if m.ctrl && !m.alt && !m.meta => Some(EditorInput::NewBuffer),
            (KeyCode::Char('d'), m) if m.ctrl && !m.alt && !m.meta => {
                Some(EditorInput::DeleteBuffer)
            }
            (KeyCode::Char('h'), m) if m.ctrl && !m.alt && !m.meta => {
                Some(EditorInput::PreviousBuffer)
            }
            (KeyCode::Char('l'), m) if m.ctrl && !m.alt && !m.meta => Some(EditorInput::NextBuffer),

            // Navigation (arrow keys)
            (KeyCode::Left, _) => Some(EditorInput::MoveLeft(1)),
            (KeyCode::Right, _) => Some(EditorInput::MoveRight(1)),
            (KeyCode::Up, _) => Some(EditorInput::MoveUp(1)),
            (KeyCode::Down, _) => Some(EditorInput::MoveDown(1)),

            // Editing
            (KeyCode::Backspace, _) => Some(EditorInput::Backspace),
            (KeyCode::Delete, _) => Some(EditorInput::DeleteChar),
            (KeyCode::Enter, _) => Some(EditorInput::InsertNewLine),
            (KeyCode::Tab, _) => Some(EditorInput::InsertString("    ".to_string())),

            // Plain character input (no control modifiers)
            (KeyCode::Char(c), m) if !m.ctrl && !m.alt && !m.meta => {
                Some(EditorInput::InsertChar(c))
            }

            // Unhandled keys
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
                self.create_buffer_with_view(Buffer::new());
                events.push(EditorEvent::Redraw);
            }
            EditorInput::DeleteBuffer => {
                // Get the current view to find its buffer
                let current_view = self.get_current_view().unwrap();
                let current_buffer_id = current_view.buffer_id();

                // Only delete if there's more than one view
                if self.views.len() > 1 {
                    // Remove the current view
                    self.views.remove(&self.current_view);
                    self.view_order.retain(|&id| id != self.current_view);

                    // Check if any other views reference this buffer
                    let buffer_in_use = self
                        .views
                        .values()
                        .any(|v| v.buffer_id() == current_buffer_id);

                    // Only remove buffer if no views reference it
                    if !buffer_in_use {
                        self.buffers.remove(&current_buffer_id);
                    }

                    // Switch to another view
                    self.current_view = *self.view_order.last().unwrap();
                    events.push(EditorEvent::Redraw);
                }
            }
            EditorInput::NextBuffer => {
                self.next_view();
                events.push(EditorEvent::Redraw);
            }
            EditorInput::PreviousBuffer => {
                self.prev_view();
                events.push(EditorEvent::Redraw);
            }

            // File I/O commands
            EditorInput::OpenFile(path) => match Buffer::from_file(&path).await {
                Ok(buffer) => {
                    self.create_buffer_with_view(buffer);
                    events.push(EditorEvent::Info(format!("Opened {}", path)));
                    events.push(EditorEvent::Redraw);
                }
                Err(e) => {
                    log::error!("Failed to open file {}: {:?}", path, e);
                    events.push(EditorEvent::Error(format!(
                        "Failed to open {}: {}",
                        path, e
                    )));
                }
            },
            EditorInput::Save => {
                // Get current buffer from current view
                let current_view = self.get_current_view().unwrap();
                let buffer_id = current_view.buffer_id();

                if let Some(buffer) = self.buffers.get_mut(&buffer_id) {
                    match buffer.save().await {
                        Ok(_) => {
                            events.push(EditorEvent::Info("Saved".to_string()));
                        }
                        Err(e) => {
                            log::error!("Failed to save buffer: {:?}", e);
                            events.push(EditorEvent::Error(format!("Save failed: {}", e)));
                        }
                    }
                }
            }
            EditorInput::SaveAs(path) => {
                // Get current buffer from current view
                let current_view = self.get_current_view().unwrap();
                let buffer_id = current_view.buffer_id();

                if let Some(buffer) = self.buffers.get_mut(&buffer_id) {
                    match buffer.save_as(&path).await {
                        Ok(_) => {
                            events.push(EditorEvent::Info(format!("Saved as {}", path)));
                        }
                        Err(e) => {
                            log::error!("Failed to save buffer as {}: {:?}", path, e);
                            events.push(EditorEvent::Error(format!(
                                "Save as {} failed: {}",
                                path, e
                            )));
                        }
                    }
                }
            }

            // All other commands operate on the current view's buffer
            _ => {
                // Get the current view and its buffer
                let current_view = self.get_current_view().unwrap();
                let buffer_id = current_view.buffer_id();
                let current_cursor = current_view.cursor();

                let current_buffer = self.buffers.get_mut(&buffer_id).unwrap();

                match command {
                    EditorInput::InsertChar(c) => {
                        let new_cursor = current_buffer.insert_char(current_cursor, c);
                        self.update_view_cursor(new_cursor);
                        events.push(EditorEvent::Redraw);
                    }
                    EditorInput::InsertString(s) => {
                        let new_cursor = current_buffer.insert_string(current_cursor, &s);
                        self.update_view_cursor(new_cursor);
                        events.push(EditorEvent::Redraw);
                    }
                    EditorInput::DeleteChar => {
                        let (_, new_cursor) = current_buffer.delete_char(current_cursor);
                        self.update_view_cursor(new_cursor);
                        events.push(EditorEvent::Redraw);
                    }
                    EditorInput::DeleteRange(_) => todo!(),
                    EditorInput::Undo => todo!(),
                    EditorInput::Redo => todo!(),
                    EditorInput::MoveUp(count) => {
                        let cursor =
                            execute_movement(current_cursor, count, |c| current_buffer.move_up(c));
                        self.update_view_cursor_sticky(cursor);
                        events.push(EditorEvent::Redraw);
                    }
                    EditorInput::MoveDown(count) => {
                        let cursor = execute_movement(current_cursor, count, |c| {
                            current_buffer.move_down(c)
                        });
                        self.update_view_cursor_sticky(cursor);
                        events.push(EditorEvent::Redraw);
                    }
                    EditorInput::MoveLeft(count) => {
                        let cursor = execute_movement(current_cursor, count, |c| {
                            current_buffer.move_left(c)
                        });
                        self.update_view_cursor(cursor);
                        events.push(EditorEvent::Redraw);
                    }
                    EditorInput::MoveRight(count) => {
                        let cursor = execute_movement(current_cursor, count, |c| {
                            current_buffer.move_right(c)
                        });
                        self.update_view_cursor(cursor);
                        events.push(EditorEvent::Redraw);
                    }
                    EditorInput::Backspace => {
                        let (_, cursor) = current_buffer.backspace(current_cursor);
                        self.update_view_cursor(cursor);
                        events.push(EditorEvent::Redraw);
                    }
                    EditorInput::InsertNewLine => {
                        let new_cursor = current_buffer.insert_char(current_cursor, '\n');
                        self.update_view_cursor(new_cursor);
                        events.push(EditorEvent::Redraw);
                    }

                    _ => {}
                }
            }
        }

        events
    }

    /// Get a view of the buffer for rendering
    pub fn get_render_data(&self, viewport_start: usize, viewport_height: usize) -> RenderData {
        let view = self.get_current_view().unwrap();
        let buffer = self.buffers.get(&view.buffer_id()).unwrap();
        let lines = buffer.get_lines(viewport_start, viewport_height);

        RenderData {
            lines,
            cursor: buffer.cursor_to_position(view.cursor()),
            viewport_start: view.scroll_line(),
            viewport_height,
        }
    }

    pub fn get_info(&self) -> EditorInfo {
        let view = self.get_current_view().unwrap();
        let buffer = self.buffers.get(&view.buffer_id()).unwrap();
        let (line_count, char_count) = buffer.stats();

        EditorInfo {
            cursor: buffer.cursor_to_position(view.cursor()),
            filepath: buffer
                .filepath()
                .and_then(|p| p.to_str())
                .map(|s| s.to_string()),
            name: buffer.name().map(|s| s.to_string()),
            modified: buffer.is_modified(),
            line_count,
            char_count,
        }
    }

    /// Get the current view
    #[inline]
    pub fn get_current_view(&self) -> Option<&View> {
        self.views.get(&self.current_view)
    }

    /// Get the current view mutably
    #[inline]
    pub fn get_current_view_mut(&mut self) -> Option<&mut View> {
        self.views.get_mut(&self.current_view)
    }

    /// Get the current buffer (from the current view)
    #[inline]
    pub fn get_current_buffer(&self) -> Option<&Buffer> {
        let view = self.views.get(&self.current_view)?;
        self.buffers.get(&view.buffer_id())
    }

    /// Get the current buffer mutably (from the current view)
    #[inline]
    pub fn get_current_buffer_mut(&mut self) -> Option<&mut Buffer> {
        let view = self.views.get(&self.current_view)?;
        let buffer_id = view.buffer_id();
        self.buffers.get_mut(&buffer_id)
    }

    /// Update the current view's cursor position
    #[inline]
    fn update_view_cursor(&mut self, cursor: usize) {
        self.get_current_view_mut()
            .expect("current_view must exist")
            .set_cursor(cursor);
    }

    /// Update the current view's cursor position while preserving desired column
    #[inline]
    fn update_view_cursor_sticky(&mut self, cursor: usize) {
        self.get_current_view_mut()
            .expect("current_view must exist")
            .update_cursor(cursor);
    }

    /// Adjust viewport scroll positions to keep cursor visible
    /// Only scrolls when cursor approaches the edge of the viewport
    pub fn adjust_scroll(&mut self, viewport_width: usize, viewport_height: usize) {
        let view = self.get_current_view().unwrap();
        let buffer = self.buffers.get(&view.buffer_id()).unwrap();

        let cursor_pos = buffer.cursor_to_position(view.cursor());
        let mut scroll_line = view.scroll_line();
        let mut scroll_column = view.scroll_column();

        // Vertical scrolling
        // Cursor too close to top
        if cursor_pos.line < scroll_line {
            scroll_line = cursor_pos.line;
        }
        // Cursor too close to bottom
        else if cursor_pos.line >= scroll_line + viewport_height {
            scroll_line = cursor_pos
                .line
                .saturating_sub(viewport_height.saturating_sub(1));
        }

        // Horizontal scrolling
        // Cursor too close to left edge
        if cursor_pos.column < scroll_column {
            scroll_column = cursor_pos.column;
        }
        // Cursor too close to right edge
        else if cursor_pos.column >= scroll_column + viewport_width {
            scroll_column = cursor_pos
                .column
                .saturating_sub(viewport_width.saturating_sub(1));
        }

        // Update the view's scroll positions
        let view = self.get_current_view_mut().unwrap();
        view.set_scroll_line(scroll_line);
        view.set_scroll_column(scroll_column);
    }
}

/// Execute a movement command N times, returning the final cursor position
/// This is a standalone function to avoid borrow checker conflicts when calling
/// buffer methods and editor methods in sequence.
fn execute_movement<F>(start_cursor: usize, count: usize, move_fn: F) -> usize
where
    F: Fn(usize) -> Option<usize>,
{
    let mut cursor = start_cursor;
    for _ in 0..count {
        match move_fn(cursor) {
            Some(new_cursor) => cursor = new_cursor,
            None => break,
        }
    }
    cursor
}

/// Information about the current editor state
#[derive(Debug)]
pub struct EditorInfo {
    pub cursor: Position,
    pub filepath: Option<String>,
    pub name: Option<String>,
    pub modified: bool,
    pub line_count: usize,
    pub char_count: usize,
}

/// Rendering data for the frontend - a snapshot of buffer content prepared for display
#[derive(Debug)]
pub struct RenderData {
    pub lines: Vec<String>,
    pub cursor: Position,
    pub viewport_start: usize,
    pub viewport_height: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper function to create a cross-platform temp file path
    fn temp_path(filename: &str) -> String {
        std::env::temp_dir()
            .join(filename)
            .to_string_lossy()
            .to_string()
    }

    #[tokio::test]
    async fn test_new_editor() {
        let editor = Editor::new();
        let info = editor.get_info();

        assert_eq!(info.line_count, 1);
        assert_eq!(info.char_count, 0);
        assert_eq!(info.cursor.line, 0);
        assert_eq!(info.cursor.column, 0);
        assert!(!info.modified);
        assert_eq!(info.filepath, None);
    }

    #[tokio::test]
    async fn test_new_buffer() {
        let mut editor = Editor::new();

        // Start with one buffer
        assert_eq!(editor.buffers.len(), 1);

        // Create a new buffer
        let events = editor.execute_command(EditorInput::NewBuffer).await;
        assert!(events.iter().any(|e| matches!(e, EditorEvent::Redraw)));
        assert_eq!(editor.buffers.len(), 2);

        // Create another buffer
        editor.execute_command(EditorInput::NewBuffer).await;
        assert_eq!(editor.buffers.len(), 3);
    }

    #[tokio::test]
    async fn test_delete_buffer() {
        let mut editor = Editor::new();

        // Can't delete the last buffer
        editor.execute_command(EditorInput::DeleteBuffer).await;
        assert_eq!(editor.buffers.len(), 1);

        // Create multiple buffers
        editor.execute_command(EditorInput::NewBuffer).await;
        editor.execute_command(EditorInput::NewBuffer).await;
        assert_eq!(editor.buffers.len(), 3);

        // Delete current buffer
        editor.execute_command(EditorInput::DeleteBuffer).await;
        assert_eq!(editor.buffers.len(), 2);

        // Delete another buffer
        editor.execute_command(EditorInput::DeleteBuffer).await;
        assert_eq!(editor.buffers.len(), 1);

        // Can't delete the last buffer
        editor.execute_command(EditorInput::DeleteBuffer).await;
        assert_eq!(editor.buffers.len(), 1);
    }

    #[tokio::test]
    async fn test_next_previous_buffer() {
        let mut editor = Editor::new();

        // Add unique content to first buffer
        editor
            .execute_command(EditorInput::InsertString("Buffer 0".to_string()))
            .await;

        // Create 2 more buffers with unique content
        editor.execute_command(EditorInput::NewBuffer).await;
        editor
            .execute_command(EditorInput::InsertString("Buffer 1".to_string()))
            .await;

        editor.execute_command(EditorInput::NewBuffer).await;
        editor
            .execute_command(EditorInput::InsertString("Buffer 2".to_string()))
            .await;

        assert_eq!(editor.buffers.len(), 3);

        // Go to next buffer (wraps around to first)
        editor.execute_command(EditorInput::NextBuffer).await;
        let view = editor.get_render_data(0, 10);
        assert_eq!(view.lines[0], "Buffer 0");

        // Go to next buffer
        editor.execute_command(EditorInput::NextBuffer).await;
        let view = editor.get_render_data(0, 10);
        assert_eq!(view.lines[0], "Buffer 1");

        // Go to previous buffer
        editor.execute_command(EditorInput::PreviousBuffer).await;
        let view = editor.get_render_data(0, 10);
        assert_eq!(view.lines[0], "Buffer 0");

        // Wrap around to last buffer
        editor.execute_command(EditorInput::PreviousBuffer).await;
        let view = editor.get_render_data(0, 10);
        assert_eq!(view.lines[0], "Buffer 2");
    }

    #[tokio::test]
    async fn test_insert_char_command() {
        let mut editor = Editor::new();

        editor.execute_command(EditorInput::InsertChar('H')).await;
        editor.execute_command(EditorInput::InsertChar('i')).await;

        let info = editor.get_info();
        assert!(info.modified);
        assert_eq!(info.char_count, 2);

        // Verify buffer content
        let view = editor.get_render_data(0, 10);
        assert_eq!(view.lines[0], "Hi");
    }

    #[tokio::test]
    async fn test_insert_string_command() {
        let mut editor = Editor::new();

        editor
            .execute_command(EditorInput::InsertString("Hello World".to_string()))
            .await;

        let info = editor.get_info();
        assert!(info.modified);
        assert_eq!(info.char_count, 11);

        let view = editor.get_render_data(0, 10);
        assert_eq!(view.lines[0], "Hello World");
    }

    #[tokio::test]
    async fn test_insert_newline() {
        let mut editor = Editor::new();

        editor
            .execute_command(EditorInput::InsertString("Line 1".to_string()))
            .await;
        editor.execute_command(EditorInput::InsertNewLine).await;
        editor
            .execute_command(EditorInput::InsertString("Line 2".to_string()))
            .await;

        let info = editor.get_info();
        assert_eq!(info.line_count, 2);

        let view = editor.get_render_data(0, 10);
        assert_eq!(view.lines[0], "Line 1");
        assert_eq!(view.lines[1], "Line 2");
    }

    #[tokio::test]
    async fn test_delete_and_backspace() {
        let mut editor = Editor::new();

        editor
            .execute_command(EditorInput::InsertString("Hello".to_string()))
            .await;

        // Backspace implementation: move_left() then delete_char()
        // So cursor is at 5, moves to 4, then deletes char at 4
        editor.execute_command(EditorInput::Backspace).await;

        let view = editor.get_render_data(0, 10);
        assert_eq!(view.lines[0], "Hell");
        assert_eq!(view.cursor.column, 4); // After move_left + delete

        // DeleteChar deletes at cursor without moving left
        editor.execute_command(EditorInput::MoveLeft(1)).await;
        editor.execute_command(EditorInput::MoveLeft(1)).await;
        editor.execute_command(EditorInput::DeleteChar).await;

        let view = editor.get_render_data(0, 10);
        assert_eq!(view.lines[0], "Hel");
        assert_eq!(view.cursor.column, 2);
    }

    #[tokio::test]
    async fn test_movement_commands() {
        let mut editor = Editor::new();

        editor
            .execute_command(EditorInput::InsertString(
                "Line 1\nLine 2\nLine 3".to_string(),
            ))
            .await;

        // Cursor starts at end of "Line 3"
        let info = editor.get_info();
        assert_eq!(info.cursor.line, 2);
        assert_eq!(info.cursor.column, 6);

        // Move up to Line 2 (column preserved at 6)
        editor.execute_command(EditorInput::MoveUp(1)).await;
        let info = editor.get_info();
        assert_eq!(info.cursor.line, 1);
        assert_eq!(info.cursor.column, 6);

        // Move down to Line 3
        // Note: move_down caps column to line_len.saturating_sub(1)
        // "Line 3" has 6 chars, so max column is 5
        editor.execute_command(EditorInput::MoveDown(1)).await;
        let info = editor.get_info();
        assert_eq!(info.cursor.line, 2);
        assert_eq!(info.cursor.column, 5); // Capped to line length

        // Move left from column 5
        editor.execute_command(EditorInput::MoveLeft(1)).await;
        let info = editor.get_info();
        assert_eq!(info.cursor.column, 4);

        // Move right back to column 5
        editor.execute_command(EditorInput::MoveRight(1)).await;
        let info = editor.get_info();
        assert_eq!(info.cursor.column, 5);
    }

    #[tokio::test]
    async fn test_get_buffer_view() {
        let mut editor = Editor::new();

        editor
            .execute_command(EditorInput::InsertString(
                "Line 1\nLine 2\nLine 3\nLine 4\nLine 5".to_string(),
            ))
            .await;

        // Get first 2 lines
        let view = editor.get_render_data(0, 2);
        assert_eq!(view.lines.len(), 2);
        assert_eq!(view.lines[0], "Line 1");
        assert_eq!(view.lines[1], "Line 2");
        assert_eq!(view.viewport_start, 0);
        assert_eq!(view.viewport_height, 2);

        // Get lines starting from line 2
        let view = editor.get_render_data(2, 3);
        assert_eq!(view.lines.len(), 3);
        assert_eq!(view.lines[0], "Line 3");
        assert_eq!(view.lines[1], "Line 4");
        assert_eq!(view.lines[2], "Line 5");
    }

    #[tokio::test]
    async fn test_buffer_isolation() {
        let mut editor = Editor::new();

        // Modify first buffer
        editor
            .execute_command(EditorInput::InsertString("Buffer 1".to_string()))
            .await;

        // Create and switch to second buffer
        editor.execute_command(EditorInput::NewBuffer).await;
        editor
            .execute_command(EditorInput::InsertString("Buffer 2".to_string()))
            .await;

        // Verify second buffer content
        let view = editor.get_render_data(0, 10);
        assert_eq!(view.lines[0], "Buffer 2");

        // Switch back to first buffer
        editor.execute_command(EditorInput::PreviousBuffer).await;

        // Verify first buffer content is unchanged
        let view = editor.get_render_data(0, 10);
        assert_eq!(view.lines[0], "Buffer 1");
    }

    #[tokio::test]
    async fn test_save_without_filepath() {
        let mut editor = Editor::new();

        editor
            .execute_command(EditorInput::InsertString("Test".to_string()))
            .await;

        // Try to save without a filepath set
        let events = editor.execute_command(EditorInput::Save).await;

        // Should generate an error event
        assert!(events.iter().any(|e| matches!(e, EditorEvent::Error(_))));
    }

    #[tokio::test]
    async fn test_save_as() {
        let mut editor = Editor::new();
        let temp_path = temp_path("iota_test_editor_save.txt");

        editor
            .execute_command(EditorInput::InsertString("Test content".to_string()))
            .await;

        // Save as
        let events = editor
            .execute_command(EditorInput::SaveAs(temp_path.to_string()))
            .await;

        // Should generate an info event
        assert!(events.iter().any(|e| matches!(e, EditorEvent::Info(_))));

        // Buffer should no longer be modified
        let info = editor.get_info();
        assert!(!info.modified);
        assert_eq!(info.filepath, Some(temp_path.to_string()));

        // Verify file was written
        let content = tokio::fs::read_to_string(&temp_path)
            .await
            .expect("read failed");
        assert_eq!(content, "Test content");

        // Cleanup
        let _ = tokio::fs::remove_file(&temp_path).await;
    }

    #[tokio::test]
    async fn test_open_file() {
        // Create a temp file
        let temp_path = temp_path("iota_test_editor_open.txt");
        let test_content = "File content\nLine 2";
        tokio::fs::write(&temp_path, test_content)
            .await
            .expect("write failed");

        let mut editor = Editor::new();

        // Open the file
        let events = editor
            .execute_command(EditorInput::OpenFile(temp_path.clone()))
            .await;

        // Should generate info and redraw events
        assert!(events.iter().any(|e| matches!(e, EditorEvent::Info(_))));
        assert!(events.iter().any(|e| matches!(e, EditorEvent::Redraw)));

        // Should have created a new buffer and switched to it
        assert_eq!(editor.buffers.len(), 2);

        // Verify content
        let view = editor.get_render_data(0, 10);
        assert_eq!(view.lines[0], "File content");
        assert_eq!(view.lines[1], "Line 2");

        let info = editor.get_info();
        assert_eq!(info.filepath, Some(temp_path.clone()));
        assert!(!info.modified);

        // Cleanup
        let _ = tokio::fs::remove_file(&temp_path).await;
    }

    #[tokio::test]
    async fn test_open_nonexistent_file() {
        let mut editor = Editor::new();

        let events = editor
            .execute_command(EditorInput::OpenFile("/nonexistent/file.txt".to_string()))
            .await;

        // Should generate an error event
        assert!(events.iter().any(|e| matches!(e, EditorEvent::Error(_))));

        // Should still have only one buffer
        assert_eq!(editor.buffers.len(), 1);
    }

    #[tokio::test]
    async fn test_with_file() {
        // Create a temp file
        let temp_path = temp_path("iota_test_editor_with_file.txt");
        let test_content = "Initial content";
        tokio::fs::write(&temp_path, test_content)
            .await
            .expect("write failed");

        // Create editor with file
        let editor = Editor::with_file(&temp_path)
            .await
            .expect("with_file failed");

        // Should have one buffer with the file loaded
        assert_eq!(editor.buffers.len(), 1);
        let info = editor.get_info();
        assert_eq!(info.filepath, Some(temp_path.clone()));
        assert!(!info.modified);
        assert_eq!(info.char_count, 15);

        // Cleanup
        let _ = tokio::fs::remove_file(&temp_path).await;
    }

    #[tokio::test]
    async fn test_modified_flag_across_buffers() {
        let mut editor = Editor::new();

        // Modify first buffer
        editor.execute_command(EditorInput::InsertChar('a')).await;
        assert!(editor.get_info().modified);

        // Create new buffer (unmodified)
        editor.execute_command(EditorInput::NewBuffer).await;
        assert!(!editor.get_info().modified);

        // Modify second buffer
        editor.execute_command(EditorInput::InsertChar('b')).await;
        assert!(editor.get_info().modified);

        // Switch back to first buffer (should still be modified)
        editor.execute_command(EditorInput::PreviousBuffer).await;
        assert!(editor.get_info().modified);
    }
}
