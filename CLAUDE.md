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
- Handles all text operations: insert, delete, cursor movement
- Manages file I/O (load from file, save, save-as)
- Tracks modification state and filepath
- Converts between cursor positions (byte offset) and line/column positions
- Key methods:
  - `new()`: Create empty buffer
  - `from_file()`: Load buffer from file
  - `save()`, `save_as()`: Write buffer to disk
  - `insert_char()`, `insert_string()`: Add text at cursor
  - `delete_char()`, `backspace()`: Remove text
  - `move_up()`, `move_down()`, `move_left()`, `move_right()`: Cursor navigation
  - `cursor_to_position()`, `position_to_cursor()`: Convert between cursor formats
  - `get_lines()`: Extract lines for rendering

**Editor (`editor.rs`)**
- Orchestrates multiple buffers and manages the current buffer index
- Translates high-level `EditorInput` commands into buffer operations
- Provides views of buffer content for rendering (`BufferView`)
- Provides editor metadata for display (`EditorInfo`)
- Command pattern: All user actions are `EditorInput` enum variants processed by `execute_command()`
- Supports multiple buffers with switching commands
- Key methods:
  - `new()`: Create editor with one empty buffer
  - `with_file()`: Create editor with a file loaded
  - `execute_command()`: Process all user input commands
  - `get_buffer_view()`: Get rendered view of current buffer
  - `get_info()`: Get current buffer metadata (cursor, filepath, modified state, etc.)

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
          → Buffer operations
          → Modified buffer state
          → BufferView/EditorInfo
          → Terminal rendering (draw)
```

### Important Implementation Details

1. **Cursor Position Tracking**: The buffer maintains `cursor_pos` as a character index (not byte index). Use `rope.char_to_byte()` and `rope.byte_to_char()` for conversions when needed.

2. **Line/Column vs Cursor**:
   - `Position` uses line/column (user-facing coordinates, 0-indexed internally)
   - Buffer internally uses `cursor_pos` (character offset)
   - Use `cursor_to_position()` and `position_to_cursor()` to convert
   - Status line displays 1-indexed line/column for user-friendliness

3. **Rope Operations**: When deleting or inserting, rope operations work with byte ranges, but cursor tracking uses character positions. The buffer handles this conversion internally.

4. **Terminal Rendering**:
   - Uses ratatui's `Layout::vertical()` to split screen into editor area and status line
   - Line numbers are rendered as `Span` widgets with dark gray styling
   - Cursor position accounts for line number gutter width
   - Viewport scrolling centers cursor when possible

5. **Buffer Management**:
   - Editor maintains a `Vec<Buffer>` and `current_buffer` index
   - Buffer switching wraps around (circular navigation)
   - At least one buffer is always present (cannot delete last buffer)
   - Each buffer tracks its own cursor, filepath, and modified state

6. **File I/O**:
   - Files can be opened from command line: `cargo run --bin terminal file.txt`
   - `EditorInput::OpenFile` creates new buffer and switches to it
   - `Save` writes to buffer's current filepath (errors if no filepath set)
   - `SaveAs` writes to new path and updates buffer's filepath
   - File I/O errors are currently silently ignored (TODO: proper error handling)

7. **Multiple Binaries**: The project structure supports multiple frontends:
   - `bin/terminal.rs`: Terminal UI frontend binary
   - Main library exports core functionality via `lib.rs`

8. **Incomplete Features**: Some `EditorInput` variants are marked `todo!()` (undo/redo, range deletion, etc.)

## Code Organization

```
lib.rs              - Module exports
editor.rs           - Editor orchestration and command handling
buffer.rs           - Text buffer implementation using Rope
location.rs         - Position and Range types
terminal.rs         - Terminal UI frontend
bin/terminal.rs     - Terminal binary entrypoint
```

## Development Notes

- The project uses Rust 2024 edition
- No tests are currently implemented - this is an area that needs work
- The client/server architecture mentioned in README.org is not yet implemented
- Multiple buffers are now supported and exposed via Ctrl-N/D/H/L keybindings
- Lua configuration support is planned but not implemented
- Error handling for file I/O needs improvement (currently errors are ignored)
- SaveAs currently hardcoded to "untitled.txt" - needs proper file path input UI
- No undo/redo functionality yet
