use anyhow::Result;
use iota_editor::editor::{Editor, EditorEvent, EditorInfo};
use ratatui::layout;
use ratatui::{
    Frame,
    crossterm::event::{self, Event, KeyEvent, KeyModifiers},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use tokio::sync::mpsc;

/// Convert a ratatui KeyEvent into our platform-independent EditorKey
fn key_event_to_editor_key(key: KeyEvent) -> Option<iota_input::EditorKey> {
    // Convert crossterm's KeyCode to our KeyCode
    let code = match key.code {
        event::KeyCode::Char(c) => iota_input::KeyCode::Char(c),
        event::KeyCode::Backspace => iota_input::KeyCode::Backspace,
        event::KeyCode::Enter => iota_input::KeyCode::Enter,
        event::KeyCode::Left => iota_input::KeyCode::Left,
        event::KeyCode::Right => iota_input::KeyCode::Right,
        event::KeyCode::Up => iota_input::KeyCode::Up,
        event::KeyCode::Down => iota_input::KeyCode::Down,
        event::KeyCode::Home => iota_input::KeyCode::Home,
        event::KeyCode::End => iota_input::KeyCode::End,
        event::KeyCode::PageUp => iota_input::KeyCode::PageUp,
        event::KeyCode::PageDown => iota_input::KeyCode::PageDown,
        event::KeyCode::Tab => iota_input::KeyCode::Tab,
        event::KeyCode::Delete => iota_input::KeyCode::Delete,
        event::KeyCode::Esc => iota_input::KeyCode::Escape,
        event::KeyCode::F(n) => iota_input::KeyCode::F(n),
        _ => return None, // Ignore unhandled keys
    };

    // Extract modifiers from crossterm's bitflags
    let modifiers = iota_input::KeyModifiers {
        ctrl: key.modifiers.contains(KeyModifiers::CONTROL),
        alt: key.modifiers.contains(KeyModifiers::ALT),
        shift: key.modifiers.contains(KeyModifiers::SHIFT),
        meta: key.modifiers.contains(KeyModifiers::SUPER),
    };

    Some(iota_input::EditorKey { code, modifiers })
}

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

        // Spawn a blocking task to read events
        // Uses poll with 250ms timeout - low CPU usage, responsive shutdown
        let event_task = tokio::task::spawn_blocking(move || -> Result<()> {
            loop {
                // Poll with timeout so we can check if channel closed
                // 250ms is long enough to avoid CPU waste, short enough for responsive exit
                if event::poll(std::time::Duration::from_millis(250))? {
                    match event::read() {
                        Ok(event) => {
                            if event_tx.send(event).is_err() {
                                // Channel closed, main loop has exited
                                log::info!("Event channel closed, stopping event reader");
                                break;
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to read event: {}", e);
                            return Err(e.into());
                        }
                    }
                } else {
                    // No event ready, check if channel is still open
                    if event_tx.is_closed() {
                        log::info!("Event channel closed, stopping event reader");
                        break;
                    }
                }
            }
            Ok(())
        });

        // Initial draw
        term.draw(|frame| self.draw(frame))?;

        let mut running = true;
        while running {
            // Wait for the next event (blocks until event arrives or channel closes)
            match self.handle_input(&mut event_rx).await? {
                Some((should_shutdown, needs_redraw)) => {
                    if should_shutdown {
                        running = false;
                    } else if needs_redraw {
                        term.draw(|frame| self.draw(frame))?;
                    }
                }
                None => {
                    // Channel closed, exit
                    running = false;
                }
            }
        }

        ratatui::restore();

        // Drop the receiver to signal the event task to stop
        drop(event_rx);

        // Wait for the event task to finish cleanly
        // With polling at 250ms intervals, it should exit within 500ms
        match tokio::time::timeout(std::time::Duration::from_millis(500), event_task).await {
            Ok(Ok(Ok(()))) => {
                log::debug!("Event task exited cleanly");
            }
            Ok(Ok(Err(e))) => {
                log::warn!("Event task exited with error: {:?}", e);
            }
            Ok(Err(e)) => {
                log::warn!("Event task panicked: {:?}", e);
            }
            Err(_) => {
                log::warn!("Event task did not exit within timeout (this shouldn't happen)");
            }
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
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
        let line_num_width = info.line_count.to_string().len().max(3);
        // Account for line numbers (width + space) when calculating text viewport width
        let viewport_width = (editor_area.width as usize).saturating_sub(line_num_width + 1);

        // Adjust scroll positions to keep cursor visible
        self.editor.adjust_scroll(viewport_width, viewport_height);

        // Get the view's scroll position (now properly adjusted)
        let view = self.editor.get_current_view().unwrap();
        let viewport_start = view.scroll_line();
        let scroll_column = view.scroll_column();
        let render_data = self.editor.get_render_data(viewport_start, viewport_height);

        let lines_with_numbers: Vec<Line> = render_data
            .lines
            .iter()
            .enumerate()
            .map(|(idx, line_text)| {
                let line_num = viewport_start + idx + 1;
                let line_num_str = format!("{:>width$} ", line_num, width = line_num_width);

                // Apply horizontal scrolling by skipping scroll_column characters
                let visible_text: String = line_text
                    .chars()
                    .skip(scroll_column)
                    .take(viewport_width)
                    .collect();

                Line::from(vec![
                    Span::styled(line_num_str, Style::default().fg(Color::DarkGray)),
                    Span::raw(visible_text),
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

        let cursor_line = render_data.cursor.line.saturating_sub(viewport_start);
        // Adjust cursor column for horizontal scroll
        let cursor_col = render_data.cursor.column.saturating_sub(scroll_column);

        let cursor_pos =
            layout::Position::new((cursor_col + line_num_width + 1) as u16, cursor_line as u16);

        frame.set_cursor_position(cursor_pos);
    }

    fn create_status_line(&self, info: &EditorInfo) -> Paragraph<'_> {
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
    ) -> Result<Option<(bool, bool)>> {
        let mut should_shutdown = false;
        let mut needs_redraw = false;

        // Block waiting for first event
        let event = match event_rx.recv().await {
            Some(event) => event,
            None => return Ok(None), // Channel closed
        };

        if let Some((shutdown, redraw)) = self.process_event(event).await? {
            should_shutdown = shutdown;
            needs_redraw = needs_redraw || redraw;
        }

        // TODO send back something more useful
        Ok(Some((should_shutdown, needs_redraw)))
    }

    async fn process_event(&mut self, event: Event) -> Result<Option<(bool, bool)>> {
        match event {
            Event::Key(key) => {
                // Convert terminal key event to editor key
                let editor_key = match key_event_to_editor_key(key) {
                    Some(k) => k,
                    None => return Ok(None), // Ignore unhandled keys
                };

                let editor_events = self.editor.process_key(editor_key).await;

                let mut should_shutdown = false;
                let mut needs_redraw = false;

                // Respond to events from the editor
                for editor_event in editor_events {
                    match editor_event {
                        EditorEvent::Shutdown => {
                            should_shutdown = true;
                        }
                        EditorEvent::Redraw => {
                            needs_redraw = true;
                        }
                        EditorEvent::Error(msg) => {
                            self.message = Some((msg, MessageType::Error));
                            needs_redraw = true;
                        }
                        EditorEvent::Info(msg) => {
                            self.message = Some((msg, MessageType::Info));
                            needs_redraw = true;
                        }
                    }
                }

                if should_shutdown || needs_redraw {
                    return Ok(Some((should_shutdown, needs_redraw)));
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
