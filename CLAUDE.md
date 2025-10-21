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
- Tracks modification state
- Converts between cursor positions (byte offset) and line/column positions
- Key methods:
  - `insert_char()`, `insert_string()`: Add text at cursor
  - `delete_char()`, `backspace()`: Remove text
  - `move_up()`, `move_down()`, `move_left()`, `move_right()`: Cursor navigation
  - `cursor_to_position()`, `position_to_cursor()`: Convert between cursor formats
  - `get_lines()`: Extract lines for rendering

**Editor (`editor.rs`)**
- Orchestrates multiple buffers and manages the current buffer
- Translates high-level `EditorInput` commands into buffer operations
- Provides views of buffer content for rendering (`BufferView`)
- Provides editor metadata for display (`EditorInfo`)
- Command pattern: All user actions are `EditorInput` enum variants processed by `execute_command()`
- Currently supports single buffer; multi-buffer support is planned

**Terminal (`terminal.rs`)**
- A frontend implementation using ratatui for terminal UI
- Handles the event loop: draw, handle input, repeat
- Maps keyboard events to `EditorInput` commands
- Renders the editor state using ratatui widgets
- Keybindings:
  - `Ctrl-C`: Quit
  - `Ctrl-S`: Save
  - `Ctrl-W`: Save as
  - Arrow keys, Enter, Backspace, Delete, Tab: Standard editing

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
   - `Position` uses line/column (user-facing coordinates)
   - Buffer internally uses `cursor_pos` (character offset)
   - Use `cursor_to_position()` and `position_to_cursor()` to convert

3. **Rope Operations**: When deleting or inserting, rope operations work with byte ranges, but cursor tracking uses character positions. The buffer handles this conversion internally.

4. **Multiple Binaries**: The project structure supports multiple frontends:
   - `bin/terminal.rs`: Terminal UI frontend
   - Main library exports core functionality via `lib.rs`

5. **Incomplete Features**: Many `EditorInput` variants are marked `todo!()` in the editor (undo/redo, range deletion, etc.)

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
- Multiple buffers are supported by the Editor struct but not yet exposed in the UI
- Lua configuration support is planned but not implemented
