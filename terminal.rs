use crate::editor::{Editor, EditorEvent};
use anyhow::Result;
use ratatui::{
    Frame,
    crossterm::event::{self, Event},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
enum MessageType {
    Error,
    Info,
}

#[derive(Debug)]
pub struct Terminal {
    editor: Editor,
    message: Option<(String, MessageType)>,
}

impl Terminal {
    pub fn new(editor: Editor) -> Result<Self> {
        Ok(Self {
            editor,
            message: None,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut term = ratatui::init();

        // Create a channel for events
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();

        // Spawn a blocking task to read events in a tight loop
        // This prevents blocking the async runtime
        let _event_task = tokio::task::spawn_blocking(move || -> Result<()> {
            loop {
                match event::read() {
                    Ok(event) => {
                        if event_tx.send(event).is_err() {
                            log::info!("Event channel closed, stopping event reader");
                            break;
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to read event: {}", e);
                        return Err(e.into());
                    }
                }
            }
            Ok(())
        });

        let mut running = true;
        while running {
            term.draw(|frame| self.draw(frame))?;

            if let Some(should_shutdown) = self.handle_input(&mut event_rx).await? {
                running = !should_shutdown;
            }
        }

        ratatui::restore();

        drop(event_rx);

        std::process::exit(0);
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();

        // Split the screen into editor area, status line, and message line
        let chunks = Layout::vertical([
            Constraint::Min(1),    // Editor area
            Constraint::Length(1), // Status line
            Constraint::Length(1), // Message line
        ])
        .split(area);

        let editor_area = chunks[0];
        let status_area = chunks[1];
        let message_area = chunks[2];

        let info = self.editor.get_info();

        let viewport_height = editor_area.height as usize;

        let viewport_start = info.cursor.line.saturating_sub(viewport_height / 2);
        let buffer_view = self.editor.get_buffer_view(viewport_start, viewport_height);

        let line_num_width = info.line_count.to_string().len().max(3);

        let lines_with_numbers: Vec<Line> = buffer_view
            .lines
            .iter()
            .enumerate()
            .map(|(idx, line_text)| {
                let line_num = viewport_start + idx + 1;
                let line_num_str = format!("{:>width$} ", line_num, width = line_num_width);

                Line::from(vec![
                    Span::styled(line_num_str, Style::default().fg(Color::DarkGray)),
                    Span::raw(line_text),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(lines_with_numbers);
        frame.render_widget(paragraph, editor_area);

        // Render status line
        let status_line = self.create_status_line(&info);
        frame.render_widget(status_line, status_area);

        // Render message line
        let message_line = self.create_message_line();
        frame.render_widget(message_line, message_area);

        let cursor_line = buffer_view.cursor.line.saturating_sub(viewport_start);
        let cursor_col = buffer_view.cursor.column;

        let cursor_pos = ratatui::layout::Position::new(
            (cursor_col + line_num_width + 1) as u16,
            cursor_line as u16,
        );

        frame.set_cursor_position(cursor_pos);
    }

    fn create_status_line(&self, info: &crate::editor::EditorInfo) -> Paragraph<'_> {
        let modified_indicator = if info.modified { "[+]" } else { "   " };
        let filename = info.filepath.as_deref().unwrap_or("[No Name]");
        let position = format!(
            "Ln {}, Col {}",
            info.cursor.line + 1,
            info.cursor.column + 1
        );
        let stats = format!("{} lines, {} chars", info.line_count, info.char_count);

        let status_text = format!(
            "{} {} | {} | {}",
            modified_indicator, filename, position, stats
        );

        Paragraph::new(Line::from(vec![Span::styled(
            status_text,
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )]))
        .style(Style::default().bg(Color::DarkGray))
    }

    async fn handle_input(
        &mut self,
        event_rx: &mut mpsc::UnboundedReceiver<Event>,
    ) -> Result<Option<bool>> {
        let mut should_shutdown = false;

        if let Some(first_event) = event_rx.recv().await {
            if let Some(shutdown) = self.process_event(first_event).await? {
                should_shutdown = shutdown;
            }

            while let Ok(event) = event_rx.try_recv() {
                if let Some(shutdown) = self.process_event(event).await? {
                    should_shutdown = shutdown;
                }
            }
        }

        if should_shutdown {
            Ok(Some(true))
        } else {
            Ok(None)
        }
    }

    async fn process_event(&mut self, event: Event) -> Result<Option<bool>> {
        match event {
            Event::Key(key) => {
                let editor_events = self.editor.process_key_event(key).await;

                // Respond to events from the editor
                for editor_event in editor_events {
                    match editor_event {
                        EditorEvent::Shutdown => {
                            // TODO this is kind of dumb, do something else
                            return Ok(Some(true));
                        }
                        EditorEvent::Redraw => {
                            // Redraw will happen automatically on next loop iteration
                        }
                        EditorEvent::Error(msg) => {
                            self.message = Some((msg, MessageType::Error));
                        }
                        EditorEvent::Info(msg) => {
                            self.message = Some((msg, MessageType::Info));
                        }
                    }
                }
            }
            _ => (),
        }

        Ok(None)
    }

    fn create_message_line(&self) -> Paragraph<'_> {
        if let Some((msg, msg_type)) = &self.message {
            let (bg_color, fg_color) = match msg_type {
                MessageType::Error => (Color::Red, Color::White),
                MessageType::Info => (Color::Blue, Color::White),
            };

            Paragraph::new(Line::from(vec![Span::styled(
                msg.clone(),
                Style::default().bg(bg_color).fg(fg_color),
            )]))
            .style(Style::default().bg(bg_color))
        } else {
            Paragraph::new("")
        }
    }
}
