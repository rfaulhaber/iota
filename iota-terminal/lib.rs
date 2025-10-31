use anyhow::Result;
use interprocess::local_socket::{
    tokio::{prelude::*, Stream},
    GenericNamespaced,
};
use iota_protocol::{EditorEvent, EditorInfo, Message, RenderData};
use log::info;
use ratatui::layout;
use ratatui::{
    crossterm::event::{self, Event, KeyEvent, KeyModifiers},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use std::path::PathBuf;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc,
};

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
    conn: Stream,
    render_data: RenderData,
    info: EditorInfo,
    message: Option<(String, MessageType)>,
}

impl Terminal {
    pub async fn connect(socket_path: PathBuf) -> Result<Self> {
        let socket_name = socket_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid socket path"))?
            .to_ns_name::<GenericNamespaced>()?;

        info!("Connecting to server at: {:?}", socket_path);
        let conn = Stream::connect(socket_name).await?;
        info!("Connected to server");

        // Initialize with empty state - will be updated on first message
        let render_data = RenderData {
            lines: vec![],
            cursor: iota_protocol::Position { line: 0, column: 0 },
            viewport_start: 0,
            viewport_height: 0,
        };

        let info = EditorInfo {
            cursor: iota_protocol::Position { line: 0, column: 0 },
            filepath: None,
            name: None,
            modified: false,
            line_count: 0,
            char_count: 0,
        };

        Ok(Self {
            conn,
            render_data,
            info,
            message: None,
        })
    }

    /// Send a key press to the server and receive the updated state
    async fn send_key(&mut self, key: iota_input::EditorKey) -> Result<Vec<EditorEvent>> {
        // Create and encode message
        let message = Message::KeyPress { key };
        let encoded = message.encode()?;

        // Send to server
        self.conn.write_all(&encoded).await?;

        // Read response length
        let mut len_buf = [0u8; 4];
        self.conn.read_exact(&mut len_buf).await?;
        let msg_len = u32::from_be_bytes(len_buf) as usize;

        // Read response data
        let mut msg_buf = vec![0u8; msg_len];
        self.conn.read_exact(&mut msg_buf).await?;

        // Decode response
        let response = Message::decode(&msg_buf)?;

        match response {
            Message::StateUpdate {
                events,
                render_data,
                info,
            } => {
                self.render_data = render_data;
                self.info = info;
                Ok(events)
            }
            _ => Err(anyhow::anyhow!("Unexpected message type from server")),
        }
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

        let line_num_width = self.info.line_count.to_string().len().max(3);
        let viewport_start = self.render_data.viewport_start;

        // For now, we don't have horizontal scrolling in the client
        // The server could be extended to support this
        let scroll_column = 0;

        let lines_with_numbers: Vec<Line> = self
            .render_data
            .lines
            .iter()
            .enumerate()
            .map(|(idx, line_text)| {
                let line_num = viewport_start + idx + 1;
                let line_num_str = format!("{:>width$} ", line_num, width = line_num_width);

                Line::from(vec![
                    Span::styled(line_num_str, Style::default().fg(Color::DarkGray)),
                    Span::raw(line_text.clone()),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(lines_with_numbers);
        frame.render_widget(paragraph, editor_area);

        // Render status line
        let status_line = self.create_status_line();
        frame.render_widget(status_line, status_area);

        // Render message line
        let message_line = self.create_message_line();
        frame.render_widget(message_line, message_area);

        let cursor_line = self.render_data.cursor.line.saturating_sub(viewport_start);
        let cursor_col = self.render_data.cursor.column.saturating_sub(scroll_column);

        let cursor_pos =
            layout::Position::new((cursor_col + line_num_width + 1) as u16, cursor_line as u16);

        frame.set_cursor_position(cursor_pos);
    }

    fn create_status_line(&self) -> Paragraph<'_> {
        let modified_indicator = if self.info.modified { "[+]" } else { "   " };
        let filename = self.info.filepath.as_deref().unwrap_or("[No Name]");
        let position = format!(
            "Ln {}, Col {}",
            self.info.cursor.line + 1,
            self.info.cursor.column + 1
        );
        let stats = format!("{} lines, {} chars", self.info.line_count, self.info.char_count);

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

                // Send key to server and receive events
                let editor_events = self.send_key(editor_key).await?;

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
