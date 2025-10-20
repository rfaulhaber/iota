use std::io::stdout;

use crate::editor::{Command, Editor};
use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, ClearType},
};

#[derive(Debug)]
pub struct Terminal {
    editor: Editor,
    running: bool,
    viewport_offset: usize,
    terminal_size: (u16, u16),
}

impl Terminal {
    pub fn new(editor: Editor) -> Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(stdout(), terminal::EnterAlternateScreen)?;

        let terminal_size = terminal::size()?;

        Ok(Self {
            editor,
            running: true,
            viewport_offset: 0,
            terminal_size,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        while self.running {
            self.draw()?;
            self.handle_input()?;
        }

        Ok(())
    }

    fn draw(&mut self) -> Result<()> {
        execute!(std::io::stdout(), terminal::Clear(ClearType::All))?;

        let (width, height) = self.terminal_size;
        let display_height = height.saturating_sub(2) as usize;

        // Get the buffer view from the editor
        let view = self
            .editor
            .get_buffer_view(self.viewport_offset, display_height);

        // Draw buffer content
        for (i, line) in view.lines.iter().enumerate() {
            execute!(stdout(), cursor::MoveTo(0, i as u16))?;

            // Truncate line if too long
            let display_line = if line.len() > width as usize {
                &line[..width as usize]
            } else {
                line
            };

            execute!(stdout(), Print(display_line))?;
        }

        // Position cursor
        let visual_line = view.cursor.line.saturating_sub(self.viewport_offset) as u16;
        let visual_col = view.cursor.column as u16;
        execute!(
            stdout(),
            cursor::MoveTo(visual_col, visual_line),
            cursor::Show
        )?;

        std::io::Write::flush(&mut stdout())?;
        Ok(())
    }

    fn handle_input(&mut self) -> Result<()> {
        if let Event::Key(key) = event::read()? {
            // Clear message on any key press
            let command = self.key_to_command(key);

            if let Some(cmd) = command {
                self.editor.execute_command(cmd);
                self.adjust_viewport();
            } else if matches!(
                (key.code, key.modifiers),
                (KeyCode::Char('c'), KeyModifiers::CONTROL)
            ) {
                self.handle_quit()?;
            }
        }

        Ok(())
    }

    fn key_to_command(&self, key: KeyEvent) -> Option<Command> {
        match (key.code, key.modifiers) {
            // File operations
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => Some(Command::Save),
            (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
                // Simplified - in real implementation, prompt for filename
                Some(Command::SaveAs("untitled.txt".to_string()))
            }

            // Navigation
            (KeyCode::Left, _) => Some(Command::MoveLeft(1)),
            (KeyCode::Right, _) => Some(Command::MoveRight(1)),
            (KeyCode::Up, _) => Some(Command::MoveUp(1)),
            (KeyCode::Down, _) => Some(Command::MoveDown(1)),

            // Editing
            (KeyCode::Backspace, _) => Some(Command::Backspace),
            (KeyCode::Delete, _) => Some(Command::DeleteChar),
            (KeyCode::Enter, _) => Some(Command::InsertNewLine),
            (KeyCode::Tab, _) => Some(Command::InsertString("    ".to_string())),

            // Character input
            (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                Some(Command::InsertChar(c))
            }

            _ => None,
        }
    }

    fn handle_quit(&mut self) -> Result<()> {
        self.running = false;
        Ok(())
    }

    fn adjust_viewport(&mut self) {
        let info = self.editor.get_info();
        let display_height = self.terminal_size.1.saturating_sub(2) as usize;

        // Scroll down if cursor is below visible area
        if info.cursor.line >= self.viewport_offset + display_height {
            self.viewport_offset = info.cursor.line.saturating_sub(display_height - 1);
        }

        // Scroll up if cursor is above visible area
        if info.cursor.line < self.viewport_offset {
            self.viewport_offset = info.cursor.line;
        }
    }

    pub fn resize(&mut self) -> Result<()> {
        self.terminal_size = terminal::size()?;
        Ok(())
    }
}
