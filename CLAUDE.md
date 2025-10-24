# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`iota` is a text editor in early development, designed to bring the best features of Emacs into a modern, async-first architecture. The project aims to be:
- Async first and truly parallel (unlike Emacs)
- Fully configurable with Lua
- Buffer-based with self-contained settings
- Mode-based buffer behavior control
- Support for Emacs-like keychording
- Built with native tree-sitter and LSP support
- Client/server architecture with multiple frontend implementations

This is currently a very early-stage project with a barebones editor implementation.

## Build and Development Commands

### Building and Running
```bash
# Build the project
cargo build

# Run the terminal frontend
cargo run --bin terminal

# Run the terminal frontend with a file
cargo run --bin terminal /path/to/file.txt

# Build in release mode
cargo build --release

# Run tests
cargo test

# Run a specific test
cargo test <test_name>

# Check code without building
cargo check
```

### Key Dependencies
- `ropey`: Rope data structure for efficient text manipulation
- `ratatui`: Terminal UI framework
- `anyhow`/`thiserror`: Error handling

## Architecture

### Core Components

**Buffer (`buffer.rs`)**
- The fundamental text storage unit using the Rope data structure from `ropey`
- **STATELESS with respect to cursor position** - does not track cursor internally
- Handles all text operations: insert, delete, cursor movement calculations
- Manages file I/O (load from file, save, save-as)
- Tracks modification state and filepath
- Converts between cursor positions (character offset) and line/column positions
- Key methods:
  - `new()`: Create empty buffer
  - `from_file()`: Load buffer from file
  - `save()`, `save_as()`: Write buffer to disk
  - `insert_char(cursor, ch)`: Add text at cursor position, returns new cursor
  - `insert_string(cursor, s)`: Add string at cursor position, returns new cursor
  - `delete_char(cursor)`, `backspace(cursor)`: Remove text, returns (success, new_cursor)
  - `move_up(cursor)`, `move_down(cursor)`, `move_left(cursor)`, `move_right(cursor)`: Calculate new cursor position, returns `Option<usize>`
  - `cursor_to_position(cursor)`, `position_to_cursor(pos)`: Convert between cursor formats
  - `get_lines(start, count)`: Extract lines for rendering

**View (`view.rs`)** ⭐ NEW ARCHITECTURE LAYER
- Represents a view into a buffer with its own cursor and viewport state
- **Separates presentation state from data** - critical for multi-window and client/server architecture
- Each view can display the same buffer with different cursor positions and scroll positions
- Enables Emacs-style window splitting and multiple clients viewing the same buffer
- Components:
  - `ViewId`: Stable identifier for views (similar to `BufferId`)
  - `buffer_id`: Which buffer this view displays
  - `cursor`: Character position in the buffer (view-specific)
  - `scroll_line`, `scroll_column`: Viewport scroll position (view-specific)
  - `desired_column`: Sticky column for vertical movement (maintains column when moving up/down)
- Key methods:
  - `new(buffer_id)`: Create view for buffer with cursor at start
  - `with_cursor(buffer_id, cursor)`: Create view with specific cursor position
  - `cursor()`, `set_cursor(pos)`: Get/set cursor position
  - `update_cursor(pos)`: Update cursor while preserving desired column (for vertical movement)
  - `scroll_line()`, `set_scroll_line(line)`: Get/set viewport scroll position
  - `desired_column()`, `set_desired_column(col)`: Manage sticky column for vertical movement

**Editor (`editor.rs`)**
- **Orchestrates buffers AND views** - maintains separation between data and presentation
- Translates high-level `EditorInput` commands into buffer operations through views
- Architecture:
  - `buffers: HashMap<BufferId, Buffer>` - All text data
  - `views: HashMap<ViewId, View>` - All view states (cursor, viewport)
  - `view_order: Vec<ViewId>` - Order of views for cycling
  - `current_view: ViewId` - The currently active view
- Command pattern: All user actions are `EditorInput` enum variants processed by `execute_command()`
- Supports multiple views with switching commands (Ctrl-N/D/H/L switch views, not buffers)
- Views can be deleted independently of buffers (buffer deleted only when no views reference it)
- Key types:
  - `RenderData`: Rendering DTO - snapshot of buffer content prepared for display (lines, cursor position, viewport info)
  - `EditorInfo`: Metadata DTO - cursor position, filepath, modified state, line/char counts
- Key methods:
  - `new()`: Create editor with one buffer and one view
  - `with_file(path)`: Create editor with a file loaded
  - `execute_command(cmd)`: Process all user input commands
  - `get_buffer_view()`: Get rendered snapshot of current buffer for display (returns `RenderData`)
  - `get_info()`: Get current buffer metadata (uses current view's cursor, returns `EditorInfo`)
  - `get_current_view()`: Get the current view
  - `get_current_buffer()`: Get current buffer (from current view's buffer_id)

**Terminal (`terminal.rs`)**
- A frontend implementation using ratatui for terminal UI
- Handles the event loop: draw, handle input, repeat
- Maps keyboard events to `EditorInput` commands
- Renders the editor state using ratatui's layout system
- UI Layout:
  - Main editor area with line numbers (no borders)
  - Status line showing: modified indicator, filename, cursor position, buffer stats
  - Line numbers are dynamically sized and right-aligned
  - Cursor positioned accounting for line number gutter
- Keybindings:
  - **File Operations:**
    - `Ctrl-S`: Save current buffer
    - `Ctrl-W`: Save as (currently hardcoded to "untitled.txt")
  - **Buffer Management:**
    - `Ctrl-N`: Create new buffer
    - `Ctrl-D`: Delete current buffer (keeps at least one)
    - `Ctrl-H`: Switch to previous buffer
    - `Ctrl-L`: Switch to next buffer
  - **Navigation:**
    - Arrow keys: Move cursor
  - **Editing:**
    - Enter: Insert newline
    - Backspace: Delete character before cursor
    - Delete: Delete character at cursor
    - Tab: Insert 4 spaces
    - Printable characters: Insert at cursor
  - **System:**
    - `Ctrl-C`: Quit editor

**Location Types (`location.rs`)**
- `Position`: Line and column coordinates (0-indexed)
- `Range`: Start and end positions for text ranges

### Data Flow

```
User Input → Terminal (handle_input)
          → EditorInput command
          → Editor (execute_command)
          → View (get cursor/viewport state)
          → Buffer operations (via cursor from view)
          → Modified buffer state + updated view cursor
          → RenderData/EditorInfo (DTOs combining buffer data + view state)
          → Terminal rendering (draw)
```

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────┐
│                       Terminal                          │
│                  (Frontend/Rendering)                   │
└────────────────────┬────────────────────────────────────┘
                     │ EditorInput
                     │ EditorEvent
                     ▼
┌─────────────────────────────────────────────────────────┐
│                       Editor                            │
│                   (Orchestration)                       │
│  ┌──────────────────┐      ┌──────────────────┐       │
│  │  Buffers (Data)  │      │  Views (State)   │       │
│  │ ┌──────────────┐ │      │ ┌──────────────┐ │       │
│  │ │ Buffer 1     │ │◄─────┤ │ View 1       │ │       │
│  │ │ (text data)  │ │      │ │ cursor: 42   │ │       │
│  │ └──────────────┘ │      │ │ scroll: 0    │ │       │
│  │ ┌──────────────┐ │      │ └──────────────┘ │       │
│  │ │ Buffer 2     │ │◄─┐   │ ┌──────────────┐ │       │
│  │ │ (text data)  │ │  └───┤ │ View 2       │ │       │
│  │ └──────────────┘ │      │ │ cursor: 100  │ │       │
│  └──────────────────┘      │ │ scroll: 5    │ │       │
│                            │ └──────────────┘ │       │
│                            └──────────────────┘       │
└─────────────────────────────────────────────────────────┘

Key: View → Buffer relationship allows:
- Multiple views of same buffer (View 1 and View 2 → Buffer 2)
- Independent cursors per view
- Foundation for window splitting and client/server
```

### Important Implementation Details

1. **View Layer Architecture** ⭐ CRITICAL
   - **Buffer is stateless**: Does NOT track cursor position internally
   - **View tracks state**: Cursor, scroll position, desired column
   - **Editor orchestrates**: Maps views to buffers, routes commands through views
   - This separation is ESSENTIAL for:
     - Multiple views of same buffer (e.g., split windows)
     - Client/server architecture (each client has own ViewId)
     - Independent viewport states per view

   Example flow for insert operation:
   ```rust
   // 1. Get view to find cursor position
   let view = editor.views.get(&editor.current_view).unwrap();
   let cursor = view.cursor();
   let buffer_id = view.buffer_id();

   // 2. Perform buffer operation with cursor from view
   let buffer = editor.buffers.get_mut(&buffer_id).unwrap();
   let new_cursor = buffer.insert_char(cursor, 'x');

   // 3. Update view with new cursor position
   editor.views.get_mut(&editor.current_view).unwrap().set_cursor(new_cursor);
   ```

2. **Cursor Position Tracking**:
   - Cursors are **character indices** (not byte indices)
   - Views store cursor as `usize` (character offset into buffer)
   - Use `rope.char_to_byte()` and `rope.byte_to_char()` for conversions when needed
   - Buffer methods take cursor as parameter and return new cursor position

3. **Line/Column vs Cursor**:
   - `Position` uses line/column (user-facing coordinates, 0-indexed internally)
   - View stores cursor as character offset (0 = start of buffer)
   - Use `buffer.cursor_to_position(cursor)` and `buffer.position_to_cursor(pos)` to convert
   - Status line displays 1-indexed line/column for user-friendliness

4. **Rope Operations**: When deleting or inserting, rope operations work with byte ranges, but cursor tracking uses character positions. The buffer handles this conversion internally.

5. **Terminal Rendering**:
   - Uses ratatui's `Layout::vertical()` to split screen into editor area and status line
   - Line numbers are rendered as `Span` widgets with dark gray styling
   - Cursor position accounts for line number gutter width
   - Viewport scrolling centers cursor when possible

6. **View Management**:
   - Editor maintains `HashMap<ViewId, View>` and `HashMap<BufferId, Buffer>`
   - View switching wraps around (circular navigation via Ctrl-H/L)
   - At least one view is always present (cannot delete last view)
   - Buffers are deleted only when no views reference them
   - Multiple views can reference the same buffer (same data, different cursors)

7. **File I/O**:
   - Files can be opened from command line: `cargo run --bin terminal file.txt`
   - `EditorInput::OpenFile` creates new buffer AND new view, switches to that view
   - `Save` writes current view's buffer to its filepath (errors if no filepath set)
   - `SaveAs` writes to new path and updates buffer's filepath
   - File I/O is **async** (uses tokio::fs), other operations are synchronous
   - File I/O errors are logged and shown to user as EditorEvent::Error

8. **Async Architecture**:
   - **File I/O is async**: `save()`, `save_as()`, `from_file()` use tokio::fs
   - **Everything else is sync**: Text operations, cursor movement, rendering
   - **Event-driven rendering**: Only redraws when events trigger changes
     - Main loop blocks on `event_rx.recv().await` (zero CPU when idle)
     - Event polling thread uses 250ms timeout (checks for shutdown)
     - Redraws only when `EditorEvent::Redraw` is emitted
     - This architecture eliminates unnecessary redraws
   - This design choice:
     - ✅ Non-blocking file operations
     - ✅ **Zero CPU usage when idle** (blocks waiting for input)
     - ✅ Simple synchronous text manipulation
     - ✅ Clean shutdown on Ctrl-C (event thread exits within 250ms)
     - ✅ Efficient rendering (only draws when needed)
     - ⏳ Future: Will need channels/actors for LSP and tree-sitter background tasks

9. **Multiple Binaries**: The project structure supports multiple frontends:
   - `bin/terminal.rs`: Terminal UI frontend binary
   - Main library exports core functionality via `lib.rs`
   - Future: Could have GUI frontend, web frontend, etc. (all sharing same Editor/Buffer/View logic)

10. **Incomplete Features**: Some `EditorInput` variants are marked `todo!()` (undo/redo, range deletion, etc.)

## Code Organization

```
lib.rs              - Module exports
editor.rs           - Editor orchestration, view/buffer management, command handling
buffer.rs           - Text buffer implementation using Rope (stateless)
view.rs             - View layer: cursor, viewport, presentation state
location.rs         - Position and Range types
input.rs            - Input handling and key parsing (EditorKey, KeySequence)
terminal.rs         - Terminal UI frontend using ratatui
bin/terminal.rs     - Terminal binary entrypoint
```

### Layer Responsibilities

```
┌─────────────────────────────────────────────────────┐
│ Presentation Layer (terminal.rs)                    │
│ - Rendering with ratatui                           │
│ - Key event capture                                │
│ - UI layout (line numbers, status line)            │
└───────────────────┬─────────────────────────────────┘
                    │ EditorInput/EditorEvent
┌───────────────────▼─────────────────────────────────┐
│ Orchestration Layer (editor.rs)                     │
│ - Command routing                                   │
│ - View ↔ Buffer mapping                            │
│ - State management                                  │
└──────┬───────────────────────┬──────────────────────┘
       │                       │
┌──────▼────────┐     ┌────────▼─────────┐
│ View Layer    │     │ Data Layer       │
│ (view.rs)     │     │ (buffer.rs)      │
│               │     │                  │
│ - cursor      │────▶│ - Rope text      │
│ - scroll      │     │ - File I/O       │
│ - viewport    │     │ - Text ops       │
└───────────────┘     └──────────────────┘
```

## Development Notes

### Current State (As of View Layer Refactor)
- The project uses Rust 2024 edition
- **✅ View layer separation complete** - Proper Buffer/View/Editor architecture
- **✅ Excellent test coverage** - 58 tests passing (50-64% test code in core modules)
- **✅ Event-driven rendering** - Zero CPU idle, only redraws on changes, clean shutdown
- **✅ Multiple views supported** - Foundation for window splitting and client/server
- The client/server architecture is NOT yet implemented (but foundation is ready)
- Multiple views are exposed via Ctrl-N/D/H/L keybindings
- Lua configuration support is planned but not implemented
- SaveAs currently hardcoded to "untitled.txt" - needs proper file path input UI
- No undo/redo functionality yet

### Architectural Priorities (Post-View Layer)

**P0 - Critical Next Steps:**
1. Buffer versioning for LSP compatibility (track edit version)
2. Helper methods for view management (split_view, close_view)
3. Configuration system (replace magic numbers like tab width)

**P1 - High Priority:**
4. Define RPC protocol for client/server split
5. Extract editor into server component
6. Add message queue infrastructure for async LSP responses

**P2 - Future Features:**
7. Implement mode system (NormalMode, InsertMode, language modes)
8. Add undo/redo with transaction support
9. Tree-sitter integration for syntax highlighting
10. LSP client integration

### Test Organization
- `buffer.rs`: Comprehensive unit tests for all text operations
- `editor.rs`: Integration tests for command execution and view/buffer management
- `input.rs`: Parser tests for key sequences and modifiers
- Tests use public API exclusively - no internal state exposure

### Known Limitations
- SaveAs has hardcoded filename (needs UI for path input)
- No undo/redo yet (needs transaction log + buffer versioning)
- File I/O errors are shown but not categorized by severity
- No syntax highlighting (waiting for tree-sitter integration)
- No LSP support yet (needs async message infrastructure)
- Single terminal view (multi-window splitting not yet implemented in terminal.rs)
