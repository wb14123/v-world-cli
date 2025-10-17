use crate::chat::message::{ChatMessage, ErrorMessage, Message};
use crate::chat::room::Room;
use crate::llm::ROLE_USER;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::broadcast::error::TryRecvError;
use tokio::sync::watch;
use tui_textarea::TextArea;

pub struct CliUI {
    room: Arc<Room>,
    user_id: Arc<String>,
    username: Arc<String>,
}

struct ScrollState {
    vertical_scroll: usize,
    vertical_scroll_state: ScrollbarState,
}

impl CliUI {
    pub fn new(room: Arc<Room>, user_id: Arc<String>, username: Arc<String>) -> Self {
        Self {
            room,
            user_id,
            username,
        }
    }

    pub fn start(&self) -> Result<(), Box<dyn Error>> {
        let mut terminal = ratatui::init();
        terminal.clear()?;

        let mut textarea = TextArea::default();
        textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("Input (Press Enter to send, Esc to quit)")
        );

        let mut messages: Vec<Arc<ChatMessage>> = Vec::new();
        let mut message_receivers: Vec<watch::Receiver<(Arc<Vec<String>>, bool)>> = Vec::new();
        let mut errors: Vec<Arc<ErrorMessage>> = Vec::new();
        let mut receiver = self.room.subscribe();
        let mut scroll_state = ScrollState {
            vertical_scroll: 0,
            vertical_scroll_state: ScrollbarState::default(),
        };

        loop {
            // Try to receive new messages (non-blocking)
            let mut new_messages = false;
            loop {
                match receiver.try_recv() {
                    Ok(Message::Chat(chat_msg)) => {
                        messages.push(chat_msg.clone());
                        message_receivers.push(chat_msg.content_stream.subscribe());
                        new_messages = true;
                    }
                    Ok(Message::Error(err_msg)) => {
                        errors.push(err_msg);
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Lagged(_)) => {
                        // Messages were dropped, continue
                        break;
                    }
                    Err(TryRecvError::Closed) => {
                        ratatui::restore();
                        return Err("Channel closed".into());
                    }
                }
            }

            // Check for updates in all message receivers
            let mut content_updated = false;
            for receiver in &mut message_receivers {
                if receiver.has_changed().unwrap_or(false) {
                    receiver.mark_unchanged();
                    content_updated = true;
                }
            }

            // Auto-scroll to bottom when new messages arrive or content updates
            if new_messages || content_updated {
                scroll_state.vertical_scroll = usize::MAX; // Will be clamped in draw()
            }

            // Draw the UI
            terminal.draw(|frame| {
                self.draw(frame, &messages, &errors, &textarea, &mut scroll_state, &mut message_receivers);
            })?;

            // Handle input events
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Esc => {
                                ratatui::restore();
                                return Ok(());
                            }
                            KeyCode::Enter => {
                                let input = textarea.lines().join("\n");
                                if !input.trim().is_empty() {
                                    let (sender, _rx) = watch::channel((Arc::new(vec![input.clone()]), true));
                                    let msg = Arc::new(ChatMessage {
                                        from_user_id: (*self.user_id).clone(),
                                        from_username: (*self.username).clone(),
                                        role: ROLE_USER.into(),
                                        content_stream: Arc::new(sender),
                                    });
                                    self.room.send_chat(msg)?;
                                    textarea = TextArea::default();
                                    textarea.set_block(
                                        Block::default()
                                            .borders(Borders::ALL)
                                            .title("Input (Press Enter to send, Esc to quit)")
                                    );
                                }
                            }
                            KeyCode::Up => {
                                scroll_state.vertical_scroll = scroll_state.vertical_scroll.saturating_sub(1);
                                scroll_state.vertical_scroll_state = scroll_state.vertical_scroll_state.position(scroll_state.vertical_scroll);
                            }
                            KeyCode::Down => {
                                scroll_state.vertical_scroll = scroll_state.vertical_scroll.saturating_add(1);
                                scroll_state.vertical_scroll_state = scroll_state.vertical_scroll_state.position(scroll_state.vertical_scroll);
                            }
                            KeyCode::PageUp => {
                                scroll_state.vertical_scroll = scroll_state.vertical_scroll.saturating_sub(10);
                                scroll_state.vertical_scroll_state = scroll_state.vertical_scroll_state.position(scroll_state.vertical_scroll);
                            }
                            KeyCode::PageDown => {
                                scroll_state.vertical_scroll = scroll_state.vertical_scroll.saturating_add(10);
                                scroll_state.vertical_scroll_state = scroll_state.vertical_scroll_state.position(scroll_state.vertical_scroll);
                            }
                            _ => {
                                textarea.input(key);
                            }
                        }
                    }
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame, messages: &[Arc<ChatMessage>], errors: &[Arc<ErrorMessage>], textarea: &TextArea, scroll_state: &mut ScrollState, message_receivers: &mut [watch::Receiver<(Arc<Vec<String>>, bool)>]) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Percentage(60),
                Constraint::Percentage(10),
                Constraint::Percentage(30),
            ])
            .split(frame.area());

        // Messages area (top 60%)
        let mut message_text = Text::default();
        for (msg_index, msg) in messages.iter().enumerate() {
            let role_line = Line::from(vec![
                Span::styled(format!("{}(@{})", &msg.from_username, &msg.from_user_id),
                             Style::default().fg(Color::Cyan)),
                Span::raw(": "),
            ]);
            message_text.lines.push(role_line);

            // Get the accumulated content for this message from the watch receiver
            if let Some(receiver) = message_receivers.get(msg_index) {
                let (content_chunks, _is_complete) = &*receiver.borrow();
                let content = content_chunks.join("");
                // Split content into lines and add each as a separate line
                for content_line in content.lines() {
                    message_text.lines.push(Line::from(content_line.to_string()));
                }
            }

            // Add blank line between messages
            message_text.lines.push(Line::from(""));
        }

        let total_lines = message_text.lines.len();
        let visible_height = chunks[0].height.saturating_sub(2) as usize; // Subtract borders

        // Auto-scroll to bottom: set scroll to show the last page
        let max_scroll = total_lines.saturating_sub(visible_height);
        if scroll_state.vertical_scroll > max_scroll {
            scroll_state.vertical_scroll = max_scroll;
        }

        // Update scrollbar state
        scroll_state.vertical_scroll_state = scroll_state.vertical_scroll_state
            .content_length(total_lines)
            .position(scroll_state.vertical_scroll);

        let messages_paragraph = Paragraph::new(message_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Messages (Use Up/Down/PgUp/PgDown to scroll)")
            )
            .wrap(Wrap { trim: false })
            .scroll((scroll_state.vertical_scroll as u16, 0));

        frame.render_widget(messages_paragraph, chunks[0]);

        // Render scrollbar
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));

        let scrollbar_area = Rect {
            x: chunks[0].x + chunks[0].width.saturating_sub(1),
            y: chunks[0].y + 1,
            width: 1,
            height: chunks[0].height.saturating_sub(2),
        };

        frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scroll_state.vertical_scroll_state);

        // Error area (middle 10%)
        let mut error_text = Text::default();
        for err in errors {
            error_text.lines.push(Line::from(vec![
                Span::styled("ERROR", Style::default().fg(Color::Red)),
                Span::raw(": "),
                Span::styled(err.msg.as_str(), Style::default().fg(Color::Red)),
            ]));
        }

        let errors_paragraph = Paragraph::new(error_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Errors")
                    .style(Style::default().fg(Color::Red))
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(errors_paragraph, chunks[1]);

        // Input area (bottom 30%)
        frame.render_widget(textarea, chunks[2]);
    }
}