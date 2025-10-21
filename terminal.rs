use crate::editor::{Editor, EditorInput};
use anyhow::Result;
use ratatui::{
    Frame,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

#[derive(Debug)]
pub struct Terminal {
    editor: Editor,
    running: bool,
}

impl Terminal {
    pub fn new(editor: Editor) -> Result<Self> {
        Ok(Self {
            editor,
            running: true,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let mut term = ratatui::init();

        while self.running {
            term.draw(|frame| self.draw(frame))?;
            self.handle_input()?;
        }

        ratatui::restore();

        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();

        // Split the screen into editor area and status line
        let chunks = Layout::vertical([
            Constraint::Min(1),      // Editor area
            Constraint::Length(1),   // Status line
        ])
        .split(area);

        let editor_area = chunks[0];
        let status_area = chunks[1];

        // Get editor info and buffer view
        let info = self.editor.get_info();

        // No borders now, so viewport height is the full editor area
        let viewport_height = editor_area.height as usize;

        // Get buffer view centered around cursor line
        let viewport_start = info.cursor.line.saturating_sub(viewport_height / 2);
        let buffer_view = self.editor.get_buffer_view(viewport_start, viewport_height);

        // Calculate line number width (minimum 3 chars for padding)
        let line_num_width = info.line_count.to_string().len().max(3);

        // Format buffer text with line numbers
        let lines_with_numbers: Vec<Line> = buffer_view.lines.iter().enumerate()
            .map(|(idx, line_text)| {
                let line_num = viewport_start + idx + 1;
                let line_num_str = format!("{:>width$} ", line_num, width = line_num_width);

                Line::from(vec![
                    Span::styled(
                        line_num_str,
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw(line_text),
                ])
            })
            .collect();

        // Render editor content without borders
        let paragraph = Paragraph::new(lines_with_numbers);
        frame.render_widget(paragraph, editor_area);

        // Render status line
        let status_line = self.create_status_line(&info);
        frame.render_widget(status_line, status_area);

        // Set cursor position (accounting for line number column and viewport)
        let cursor_line = buffer_view.cursor.line.saturating_sub(viewport_start);
        let cursor_col = buffer_view.cursor.column;

        let cursor_pos = ratatui::layout::Position::new(
            (cursor_col + line_num_width + 1) as u16,  // +line_num_width+1 for line numbers and space
            cursor_line as u16,
        );

        frame.set_cursor_position(cursor_pos);
    }

    fn create_status_line(&self, info: &crate::editor::EditorInfo) -> Paragraph<'_> {
        let modified_indicator = if info.modified { "[+]" } else { "   " };
        let filename = info.filepath.as_deref().unwrap_or("[No Name]");
        let position = format!("Ln {}, Col {}", info.cursor.line + 1, info.cursor.column + 1);
        let stats = format!("{} lines, {} chars", info.line_count, info.char_count);

        let status_text = format!(
            "{} {} | {} | {}",
            modified_indicator, filename, position, stats
        );

        Paragraph::new(Line::from(vec![
            Span::styled(
                status_text,
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .style(Style::default().bg(Color::DarkGray))
    }

    fn handle_input(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key) => {
                let command = self.key_to_command(key);
                if let Some(cmd) = command {
                    self.editor.execute_command(cmd);
                } else if matches!(
                    (key.code, key.modifiers),
                    (KeyCode::Char('c'), KeyModifiers::CONTROL)
                ) {
                    self.handle_quit()?;
                }
            }
            _ => (),
        }

        Ok(())
    }

    fn key_to_command(&self, key: KeyEvent) -> Option<EditorInput> {
        match (key.code, key.modifiers) {
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
        }
    }

    fn handle_quit(&mut self) -> Result<()> {
        self.running = false;
        Ok(())
    }
}
