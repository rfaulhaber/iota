/// View layer - separates cursor/selection/viewport from buffer content
/// This allows multiple views into the same buffer with different cursor positions,
/// selections, and scroll positions. This is essential for:
/// - Multiple windows viewing the same buffer (Emacs-style splitting)
/// - Client/server architecture (each client has its own view)
/// - Multiple cursors (future feature)
use crate::editor::BufferId;

/// Stable identifier for views
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ViewId(usize);

impl ViewId {
    pub(crate) fn new(id: usize) -> Self {
        Self(id)
    }
}

/// A view into a buffer with its own cursor position and viewport state
#[derive(Debug, Clone)]
pub struct View {
    /// The buffer this view is displaying
    buffer_id: BufferId,
    /// Cursor position in characters (not bytes)
    cursor: usize,
    /// Viewport scroll position (top line visible)
    scroll_line: usize,
    /// Viewport scroll position (left column visible, for horizontal scrolling)
    scroll_column: usize,
    /// Desired column for vertical movement (sticky column)
    /// When moving up/down, we try to stay in this column
    desired_column: Option<usize>,
}

impl View {
    /// Create a new view for a buffer with cursor at start
    pub fn new(buffer_id: BufferId) -> Self {
        Self {
            buffer_id,
            cursor: 0,
            scroll_line: 0,
            scroll_column: 0,
            desired_column: None,
        }
    }

    /// Create a view with a specific cursor position
    pub fn with_cursor(buffer_id: BufferId, cursor: usize) -> Self {
        Self {
            buffer_id,
            cursor,
            scroll_line: 0,
            scroll_column: 0,
            desired_column: None,
        }
    }

    /// Get the buffer ID this view is displaying
    pub fn buffer_id(&self) -> BufferId {
        self.buffer_id
    }

    /// Get the current cursor position
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Set the cursor position
    pub fn set_cursor(&mut self, cursor: usize) {
        self.cursor = cursor;
        // Clear desired column when cursor is explicitly set
        self.desired_column = None;
    }

    /// Update cursor position while preserving desired column
    /// Used for vertical movement to maintain column position
    pub fn update_cursor(&mut self, cursor: usize) {
        self.cursor = cursor;
    }

    /// Get the viewport scroll position (top visible line)
    pub fn scroll_line(&self) -> usize {
        self.scroll_line
    }

    /// Set the viewport scroll position
    pub fn set_scroll_line(&mut self, line: usize) {
        self.scroll_line = line;
    }

    /// Get the horizontal scroll position
    pub fn scroll_column(&self) -> usize {
        self.scroll_column
    }

    /// Set the horizontal scroll position
    pub fn set_scroll_column(&mut self, column: usize) {
        self.scroll_column = column;
    }

    /// Get the desired column for vertical movement
    pub fn desired_column(&self) -> Option<usize> {
        self.desired_column
    }

    /// Set the desired column for vertical movement
    pub fn set_desired_column(&mut self, column: usize) {
        self.desired_column = Some(column);
    }

    /// Clear the desired column
    pub fn clear_desired_column(&mut self) {
        self.desired_column = None;
    }
}
