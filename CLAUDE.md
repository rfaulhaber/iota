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
# Build the entire workspace
cargo build

# Build specific crates
cargo build -p iota-server
cargo build -p iota-terminal

# Run the terminal client (auto-starts server if needed)
cargo run -p iota-terminal

# Or run the server manually (optional - terminal will auto-start it)
cargo run -p iota-server

# Build in release mode
cargo build --release

# Run tests
cargo test

# Run a specific test
cargo test <test_name>

# Check code without building
cargo check
```


### Socket Path Configuration

The server and client communicate via Unix domain sockets. The socket path is determined by environment variables in this priority order:

1. **`$IOTA_SERVER_SOCKET`** - Explicit override (highest priority)
2. **`$XDG_RUNTIME_DIR/iota-server.sock`** - XDG standard location (default on Linux)
3. **`/tmp/iota-server.sock`** - Cross-platform fallback

```bash
# Default: Uses XDG_RUNTIME_DIR if set, otherwise /tmp
cargo run -p iota-terminal

# Custom socket path (both client and server must use same path)
IOTA_SERVER_SOCKET=/path/to/custom.sock cargo run -p iota-terminal

# Explicit XDG directory
XDG_RUNTIME_DIR=/custom/runtime cargo run -p iota-terminal

# Force /tmp fallback
env -u IOTA_SERVER_SOCKET -u XDG_RUNTIME_DIR cargo run -p iota-terminal
```

### Key Dependencies
- `ropey`: Rope data structure for efficient text manipulation
- `ratatui`: Terminal UI framework (client only)
- `anyhow`/`thiserror`: Error handling
- `bincode`: Binary serialization for client/server protocol
- `interprocess`: Unix domain socket communication
- `tokio`: Async runtime

## Architecture

### Crate Structure

The project is organized as a Cargo workspace with the following crates:

**iota-core** - Core types
- `Position`: Line/column coordinates
- `Range`: Text range representation

**iota-input** - Input handling
- `EditorKey`: Platform-independent key representation
- `KeyCode`: Key codes (Char, Arrow keys, etc.)
- `KeyModifiers`: Ctrl, Alt, Shift, Meta

**iota-editor** - Core editor logic
- `Editor`: Orchestrates buffers and views
- `Buffer`: Text storage and operations
- `View`: Cursor and viewport state
- `EditorEvent`: Events sent to frontends

**iota-protocol** - Client/server protocol
- `Message`: Bincode-serializable protocol messages
  - `KeyPress`: Client → Server key events
  - `StateUpdate`: Server → Client state updates
- `Position`, `RenderData`, `EditorInfo`: Protocol DTOs
- `get_socket_path()`: Environment-based socket path resolution

**iota-server** - Server implementation
- `Server`: Manages editor state and client connections
- Listens on Unix domain socket
- Handles multiple concurrent clients
- Each client gets own async task with shared `Arc<RwLock<Editor>>`

**iota-terminal** - Terminal frontend client
- `Terminal`: ratatui-based TUI client
- Connects to server via Unix socket
- Sends key events, receives state updates
- Renders editor state in terminal

### Core Components

**Buffer (`iota-editor/buffer.rs`)**
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

**View (`iota-editor/view.rs`)** ⭐ CRITICAL ARCHITECTURE LAYER
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

**Editor (`iota-editor/editor.rs`)**
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

**Server (`iota-server/lib.rs`)**
- Manages the shared `Editor` instance wrapped in `Arc<RwLock<>>`
- Listens for client connections on Unix domain socket
- Spawns async task per client connection
- Processes `Message::KeyPress` from clients
- Responds with `Message::StateUpdate` containing events, render data, and editor info
- Socket path: `$IOTA_SERVER_SOCKET` → `$XDG_RUNTIME_DIR/iota-server.sock` → `/tmp/iota-server.sock`

**Terminal Client (`iota-terminal/lib.rs` and `main.rs`)**
- Frontend implementation using ratatui for terminal UI
- Connects to server via Unix domain socket
- Event loop: capture input → send to server → receive state → render
- Maps keyboard events to `EditorKey` protocol messages
- Renders received `RenderData` using ratatui's layout system
- Does NOT embed Editor directly - purely a presentation layer
- UI Layout:
  - Main editor area with line numbers (no borders)
  - Status line showing: modified indicator, filename, cursor position, buffer stats
  - Message line for errors/info
  - Line numbers are dynamically sized and right-aligned
  - Cursor positioned accounting for line number gutter
- Keybindings (sent to server as EditorKey messages):
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

**Location Types (`iota-core/location.rs`)**
- `Position`: Line and column coordinates (0-indexed)
- `Range`: Start and end positions for text ranges

### Data Flow (Client/Server Architecture)

```
┌─────────────────┐                           ┌─────────────────┐
│  Terminal UI    │                           │     Server      │
│  (iota-terminal)│                           │  (iota-server)  │
└─────────────────┘                           └─────────────────┘
        │                                              │
        │ 1. User types 'x'                            │
        │                                              │
        │ 2. KeyEvent captured                         │
        │    → Convert to EditorKey                    │
        │                                              │
        │ 3. Create Message::KeyPress                  │
        │    → Bincode encode                          │
        │                                              │
        │ 4. Send via Unix socket ────────────────────▶│
        │                                              │
        │                                              │ 5. Decode message
        │                                              │
        │                                              │ 6. Call editor.process_key()
        │                                              │    → Get view cursor
        │                                              │    → buffer.insert_char()
        │                                              │    → Update view cursor
        │                                              │
        │                                              │ 7. Collect EditorEvents
        │                                              │
        │                                              │ 8. Get RenderData
        │                                              │    (lines, cursor, viewport)
        │                                              │
        │                                              │ 9. Get EditorInfo
        │                                              │    (filepath, modified, etc.)
        │                                              │
        │                                              │ 10. Create Message::StateUpdate
        │                                              │     → Bincode encode
        │                                              │
        │ 11. Receive state update ◄───────────────────┤
        │                                              │
        │ 12. Update local state:                      │
        │     - render_data                            │
        │     - info                                   │
        │     - process events                         │
        │                                              │
        │ 13. Redraw UI if needed                      │
        │     (ratatui rendering)                      │
        │                                              │
        └──────────────────────────────────────────────┘

Message Format (bincode-encoded, length-prefixed):
┌────────────┬──────────────────────────────┐
│ 4 bytes    │ N bytes                      │
│ (u32 len)  │ (bincode-encoded Message)    │
└────────────┴──────────────────────────────┘
```

### Architecture Diagram

```
                    ┌─────────────────────────────┐
                    │    Terminal Client 1        │
                    │    (iota-terminal)          │
                    │  - ratatui UI               │
                    │  - Event capture            │
                    │  - Rendering                │
                    └──────────────┬──────────────┘
                                   │ Unix Socket
                                   │ (bincode msgs)
                                   │
    ┌──────────────────────────────┼──────────────────────────────┐
    │                              │                              │
    │                              ▼                              │
    │  ┌─────────────────────────────────────────────────────┐   │
    │  │           iota-server (Server Process)              │   │
    │  │                                                     │   │
    │  │  ┌────────────────────────────────────────────┐    │   │
    │  │  │    Arc<RwLock<Editor>>                     │    │   │
    │  │  │                                            │    │   │
    │  │  │  ┌────────────────┐  ┌──────────────────┐ │    │   │
    │  │  │  │ Buffers (Data) │  │  Views (State)   │ │    │   │
    │  │  │  │ ┌────────────┐ │  │ ┌──────────────┐ │ │    │   │
    │  │  │  │ │ Buffer 1   │◄┼──┼─┤ View 1       │ │ │    │   │
    │  │  │  │ │ (Rope)     │ │  │ │ cursor: 42   │ │ │    │   │
    │  │  │  │ └────────────┘ │  │ │ scroll: 0    │ │ │    │   │
    │  │  │  │ ┌────────────┐ │  │ └──────────────┘ │ │    │   │
    │  │  │  │ │ Buffer 2   │◄┼──┼─┤ View 2       │ │ │    │   │
    │  │  │  │ │ (Rope)     │ │  │ │ cursor: 100  │ │ │    │   │
    │  │  │  │ └────────────┘ │  │ │ scroll: 5    │ │ │    │   │
    │  │  │  └────────────────┘  │ └──────────────┘ │ │    │   │
    │  │  │                      └──────────────────┘ │    │   │
    │  │  └────────────────────────────────────────────┘    │   │
    │  │                                                     │   │
    │  │  Connection Tasks (one per client):                │   │
    │  │  - Decode Message::KeyPress                        │   │
    │  │  - Call editor.process_key()                       │   │
    │  │  - Encode Message::StateUpdate                     │   │
    │  └─────────────────────────────────────────────────────┘   │
    │                              │                              │
    │                              │ Unix Socket                  │
    └──────────────────────────────┼──────────────────────────────┘
                                   │ (bincode msgs)
                                   │
                    ┌──────────────┴──────────────┐
                    │    Terminal Client 2        │
                    │    (iota-terminal)          │
                    │  - Same server, different   │
                    │    socket connection        │
                    └─────────────────────────────┘

Key Features:
- Multiple clients can connect to same server
- Editor state shared via Arc<RwLock<>>
- Each client connection handled in separate async task
- Bincode-encoded messages over Unix domain sockets
- View → Buffer relationship enables concurrent editing
- Socket path: $IOTA_SERVER_SOCKET → $XDG_RUNTIME_DIR/iota-server.sock → /tmp/iota-server.sock
```

### Protocol Implementation Details

**Message Format:**
All messages are bincode-encoded with a 4-byte length prefix:
```
┌────────────┬──────────────────────────────┐
│ 4 bytes    │ N bytes                      │
│ (u32 len)  │ (bincode-encoded Message)    │
└────────────┴──────────────────────────────┘
```

**Message Types:**
```rust
// Client → Server
Message::KeyPress {
    key: EditorKey,
}

// Server → Client
Message::StateUpdate {
    events: Vec<EditorEvent>,     // Shutdown, Redraw, Error(String), Info(String)
    render_data: RenderData,      // lines, cursor, viewport_start, viewport_height
    info: EditorInfo,             // cursor, filepath, name, modified, line/char counts
}
```

**Communication Flow:**
1. Client reads keyboard event → converts to `EditorKey`
2. Client creates `Message::KeyPress` → encodes with length prefix → sends
3. Server reads length → reads message bytes → decodes
4. Server calls `editor.process_key(key)` → gets events
5. Server gets `RenderData` and `EditorInfo` from editor
6. Server creates `Message::StateUpdate` → encodes with length prefix → sends
7. Client reads length → reads message bytes → decodes
8. Client updates local state and redraws UI

**Socket Path Resolution:**
Implemented in `iota_protocol::get_socket_path()`:
1. `$IOTA_SERVER_SOCKET` (explicit override)
2. `$XDG_RUNTIME_DIR/iota-server.sock` (XDG standard)
3. `/tmp/iota-server.sock` (fallback)

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
   - **Server side**:
     - Socket listener runs async (tokio)
     - Each client connection handled in separate async task
     - Editor wrapped in `Arc<RwLock<>>` for concurrent access
     - File I/O is async: `save()`, `save_as()`, `from_file()` use tokio::fs
     - Text operations are sync (within RwLock critical sections)
   - **Client side**:
     - Event-driven rendering: Only redraws when server sends updates
     - Main loop blocks on `event_rx.recv().await` (zero CPU when idle)
     - Event polling thread uses 250ms timeout (checks for shutdown)
     - Redraws only when `EditorEvent::Redraw` is received from server
     - Socket I/O is async (tokio)
   - This design choice:
     - ✅ Non-blocking file operations
     - ✅ **Zero CPU usage when idle** (blocks waiting for input/socket)
     - ✅ Simple synchronous text manipulation
     - ✅ Clean shutdown on Ctrl-C (event thread exits within 250ms)
     - ✅ Efficient rendering (only draws when needed)
     - ✅ Multiple concurrent clients supported
     - ⏳ Future: Will need channels/actors for LSP and tree-sitter background tasks

9. **Multiple Frontends**: The project structure supports multiple frontend implementations:
   - `iota-terminal`: Terminal UI frontend using ratatui (✅ implemented)
   - `iota-server`: Headless server that owns the Editor state (✅ implemented)
   - Future frontends: GUI (egui/iced), web (WASM + WebSockets), etc.
   - All frontends share the same protocol (`iota-protocol`)
   - All share the same editor logic (`iota-editor`)

10. **Incomplete Features**: Some `EditorInput` variants are marked `todo!()` (undo/redo, range deletion, etc.)

## Code Organization

### Workspace Structure

The project is organized as a Cargo virtual workspace (no root package):

```
iota/                        - Virtual workspace root
├── Cargo.toml              - Workspace manifest
├── CLAUDE.md               - This file (project documentation)
│
├── iota-core/              - Core types
│   ├── lib.rs             - Module exports
│   └── location.rs        - Position and Range types
│
├── iota-input/             - Input handling
│   ├── lib.rs             - EditorKey, KeyCode, KeyModifiers
│   └── (key parsing)
│
├── iota-editor/            - Core editor logic
│   ├── lib.rs             - Module exports
│   ├── editor.rs          - Editor orchestration, view/buffer management
│   ├── buffer.rs          - Text buffer (Rope, file I/O, text operations)
│   └── view.rs            - View layer (cursor, viewport, scroll state)
│
├── iota-protocol/          - Client/server protocol
│   └── lib.rs             - Message types, bincode encoding, socket path resolution
│
├── iota-server/            - Server implementation
│   ├── lib.rs             - Server, connection handling, message processing
│   └── main.rs            - Server binary entrypoint
│
└── iota-terminal/          - Terminal frontend client
    ├── lib.rs             - Terminal UI using ratatui, socket communication
    └── main.rs            - Terminal client binary entrypoint
```

### Layer Responsibilities

```
┌─────────────────────────────────────────────────────┐
│ Client Layer (iota-terminal)                        │
│ - ratatui UI rendering                              │
│ - Keyboard event capture                            │
│ - Socket communication                              │
│ - UI layout (line numbers, status line, messages)  │
└───────────────────┬─────────────────────────────────┘
                    │ Unix Socket
                    │ Message::KeyPress ────────────▶
                    │ Message::StateUpdate ◀─────────
┌───────────────────▼─────────────────────────────────┐
│ Server Layer (iota-server)                          │
│ - Socket listener & connection management           │
│ - Message encoding/decoding (bincode)               │
│ - Owns Arc<RwLock<Editor>>                          │
└───────────────────┬─────────────────────────────────┘
                    │
┌───────────────────▼─────────────────────────────────┐
│ Orchestration Layer (iota-editor)                   │
│ Editor:                                             │
│ - Command routing (process_key)                     │
│ - View ↔ Buffer mapping                            │
│ - State management                                  │
│ - Generate RenderData/EditorInfo                    │
└──────┬───────────────────────┬──────────────────────┘
       │                       │
┌──────▼────────┐     ┌────────▼─────────┐
│ View Layer    │     │ Data Layer       │
│ (iota-editor) │     │ (iota-editor)    │
│               │     │                  │
│ - cursor      │────▶│ - Rope text      │
│ - scroll      │     │ - File I/O       │
│ - viewport    │     │ - Text ops       │
└───────────────┘     └──────────────────┘
         │                     │
         │                     │
         └──────────┬──────────┘
                    │
┌───────────────────▼─────────────────────────────────┐
│ Protocol Layer (iota-protocol)                      │
│ - Message types (KeyPress, StateUpdate)             │
│ - Position, RenderData, EditorInfo DTOs             │
│ - Bincode encoding/decoding                         │
│ - Socket path resolution                            │
└─────────────────────────────────────────────────────┘
```

## Development Notes

### Current State (As of Client/Server Implementation)
- The project uses Rust 2024 edition
- **✅ Client/server architecture complete** - Full Unix socket communication with bincode protocol
- **✅ View layer separation complete** - Proper Buffer/View/Editor architecture
- **✅ Excellent test coverage** - 58 tests passing (50-64% test code in core modules)
- **✅ Event-driven rendering** - Zero CPU idle, only redraws on changes, clean shutdown
- **✅ Multiple clients supported** - Multiple terminal clients can connect to same server
- **✅ Environment-based socket configuration** - IOTA_SERVER_SOCKET, XDG_RUNTIME_DIR, /tmp fallback
- **✅ Bincode message protocol** - Efficient binary serialization for client/server communication
- Multiple views are exposed via Ctrl-N/D/H/L keybindings
- Lua configuration support is planned but not implemented
- SaveAs currently hardcoded to "untitled.txt" - needs proper file path input UI
- No undo/redo functionality yet

### Architectural Priorities (Post-Client/Server Split)

**P0 - Critical Next Steps:**
1. Buffer versioning for LSP compatibility (track edit version)
2. Per-client views (each client should have own ViewId)
3. Configuration system (replace magic numbers like tab width)

**P1 - High Priority:**
4. Extend protocol for viewport size negotiation (client tells server terminal dimensions)
5. Add horizontal scrolling support in protocol
6. Helper methods for view management (split_view, close_view)
7. Add message queue infrastructure for async LSP responses

**P2 - Future Features:**
8. Implement mode system (NormalMode, InsertMode, language modes)
9. Add undo/redo with transaction support
10. Tree-sitter integration for syntax highlighting
11. LSP client integration
12. File path input UI (for Save As)
13. Additional frontends (GUI, web)

### Test Organization
- `iota-editor/buffer.rs`: Comprehensive unit tests for all text operations
- `iota-editor/editor.rs`: Integration tests for command execution and view/buffer management
- `iota-input`: Parser tests for key sequences and modifiers
- Tests use public API exclusively - no internal state exposure
- Client/server integration:
  - Terminal 1: `cargo run -p iota-server`
  - Terminal 2: `cargo run -p iota-terminal`

### Known Limitations
- SaveAs has hardcoded filename (needs UI for path input)
- No undo/redo yet (needs transaction log + buffer versioning)
- File I/O errors are shown but not categorized by severity
- No syntax highlighting (waiting for tree-sitter integration)
- No LSP support yet (needs async message infrastructure)
- Single terminal view per client (multi-window splitting not yet implemented)
- All clients share the same Editor state (per-client views not yet implemented)
- Fixed viewport height in server (24 lines) - needs viewport size negotiation protocol
- No horizontal scrolling in terminal client
