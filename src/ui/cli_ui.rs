use std::error::Error;
use std::sync::Arc;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use tui_textarea::TextArea;
use tokio::sync::broadcast::error::TryRecvError;
use crate::chat::message::{ChatMessage, ErrorMessage, Message};
use crate::chat::room::Room;
use crate::llm::ROLE_USER;

pub struct CliUI {
    room: Arc<Room>,
    user_id: Arc<String>,
    username: Arc<String>,
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
        let mut errors: Vec<Arc<ErrorMessage>> = Vec::new();
        let mut receiver = self.room.subscribe();

        loop {
            // Try to receive new messages (non-blocking)
            loop {
                match receiver.try_recv() {
                    Ok(Message::Chat(chat_msg)) => {
                        messages.push(chat_msg);
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

            // Draw the UI
            terminal.draw(|frame| {
                self.draw(frame, &messages, &errors, &textarea);
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
                                    let msg = Arc::new(ChatMessage {
                                        from_user_id: (*self.user_id).clone(),
                                        role: ROLE_USER.into(),
                                        content: Arc::new(input),
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
                            _ => {
                                textarea.input(key);
                            }
                        }
                    }
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame, messages: &[Arc<ChatMessage>], errors: &[Arc<ErrorMessage>], textarea: &TextArea) {
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
        for msg in messages {
            let role_line = Line::from(vec![
                Span::styled(&msg.role, Style::default().fg(Color::Cyan)),
                Span::raw(": "),
            ]);
            message_text.lines.push(role_line);

            // Split content into lines and add each as a separate line
            for content_line in msg.content.lines() {
                message_text.lines.push(Line::from(content_line.to_string()));
            }

            // Add blank line between messages
            message_text.lines.push(Line::from(""));
        }

        let messages_paragraph = Paragraph::new(message_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Messages")
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(messages_paragraph, chunks[0]);

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