//! Conversation Panel for Human-Agent Interaction
//!
//! This module provides a chat-style conversation panel that displays
//! the dialogue between human and agent with timestamps and context.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use std::time::SystemTime;

/// Message in the conversation
#[derive(Debug, Clone)]
pub struct ConversationMessage {
    pub timestamp: SystemTime,
    pub sender: MessageSender,
    pub content: String,
    pub message_type: MessageType,
}

/// Who sent the message
#[derive(Debug, Clone, PartialEq)]
pub enum MessageSender {
    Human,
    Agent,
    System,
}

/// Type of message
#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    Text,
    Action,
    Reasoning,
    Approval,
    Error,
    Success,
}

/// Conversation panel widget
pub struct ConversationPanel {
    messages: Vec<ConversationMessage>,
    scroll_offset: usize,
    max_messages: usize,
}

impl ConversationPanel {
    pub fn new(max_messages: usize) -> Self {
        Self {
            messages: Vec::new(),
            scroll_offset: 0,
            max_messages,
        }
    }

    pub fn add_message(&mut self, message: ConversationMessage) {
        self.messages.push(message);

        // Keep only recent messages
        if self.messages.len() > self.max_messages {
            self.messages
                .drain(0..self.messages.len() - self.max_messages);
        }

        // Auto-scroll to bottom
        self.scroll_to_bottom();
    }

    pub fn add_human_message(&mut self, content: String) {
        self.add_message(ConversationMessage {
            timestamp: SystemTime::now(),
            sender: MessageSender::Human,
            content,
            message_type: MessageType::Text,
        });
    }

    pub fn add_agent_message(&mut self, content: String, message_type: MessageType) {
        self.add_message(ConversationMessage {
            timestamp: SystemTime::now(),
            sender: MessageSender::Agent,
            content,
            message_type,
        });
    }

    pub fn add_system_message(&mut self, content: String) {
        self.add_message(ConversationMessage {
            timestamp: SystemTime::now(),
            sender: MessageSender::System,
            content,
            message_type: MessageType::Text,
        });
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        let visible = 10; // Approximate visible lines
        if self.scroll_offset + visible < self.messages.len() {
            self.scroll_offset += 1;
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.messages.len().saturating_sub(10);
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.scroll_offset = 0;
    }

    /// Render the conversation panel
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)])
            .split(area);

        // Messages area
        self.render_messages(f, chunks[0]);

        // Status/hint area
        self.render_status(f, chunks[1]);
    }

    fn render_messages(&self, f: &mut Frame, area: Rect) {
        let visible_messages: Vec<&ConversationMessage> = self
            .messages
            .iter()
            .skip(self.scroll_offset)
            .take(20) // Show up to 20 messages
            .collect();

        let mut items = Vec::new();

        for msg in visible_messages {
            let (prefix, style) = self.get_message_style(&msg.sender, &msg.message_type);
            let timestamp_str = self.format_timestamp(msg.timestamp);
            let content_lines = self.wrap_text(&msg.content, 70);

            for (i, line) in content_lines.iter().enumerate() {
                if i == 0 {
                    // First line with timestamp and prefix
                    items.push(ListItem::new(Line::from(vec![
                        Span::styled(timestamp_str.clone(), Style::default().fg(Color::DarkGray)),
                        Span::raw(" "),
                        Span::styled(prefix.clone(), style),
                        Span::raw(" "),
                        Span::styled(line.clone(), style),
                    ])));
                } else {
                    // Continuation lines
                    items.push(ListItem::new(Line::from(vec![
                        Span::raw("       "), // Indent
                        Span::styled(line.clone(), style),
                    ])));
                }
            }
        }

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Conversation ({} messages)", self.messages.len())),
        );

        f.render_widget(list, area);
    }

    fn render_status(&self, f: &mut Frame, area: Rect) {
        let status_text = if self.messages.is_empty() {
            "No messages yet. Agent activity will appear here."
        } else {
            "↑/↓ to scroll • Press 'I' to provide input"
        };

        let status = Paragraph::new(status_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(status, area);
    }

    fn get_message_style(&self, sender: &MessageSender, msg_type: &MessageType) -> (String, Style) {
        match (sender, msg_type) {
            (MessageSender::Human, _) => (
                "👤 [You]".to_string(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            (MessageSender::Agent, MessageType::Text) => {
                ("🤖 [Agent]".to_string(), Style::default().fg(Color::Green))
            }
            (MessageSender::Agent, MessageType::Action) => {
                ("🔧 [Agent]".to_string(), Style::default().fg(Color::Yellow))
            }
            (MessageSender::Agent, MessageType::Reasoning) => (
                "💭 [Agent]".to_string(),
                Style::default().fg(Color::Magenta),
            ),
            (MessageSender::Agent, MessageType::Approval) => (
                "⚠️  [Agent]".to_string(),
                Style::default().fg(Color::LightRed),
            ),
            (MessageSender::Agent, MessageType::Error) => {
                ("❌ [Agent]".to_string(), Style::default().fg(Color::Red))
            }
            (MessageSender::Agent, MessageType::Success) => (
                "✅ [Agent]".to_string(),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            (MessageSender::System, _) => {
                ("ℹ️  [System]".to_string(), Style::default().fg(Color::Gray))
            }
        }
    }

    fn format_timestamp(&self, timestamp: SystemTime) -> String {
        use std::time::UNIX_EPOCH;

        if let Ok(duration) = timestamp.duration_since(UNIX_EPOCH) {
            let secs = duration.as_secs();
            let hours = (secs % 86400) / 3600;
            let minutes = (secs % 3600) / 60;
            let seconds = secs % 60;
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            "??:??:??".to_string()
        }
    }

    fn wrap_text(&self, text: &str, max_width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut current_line = String::new();

        for word in words {
            let separator_len = if current_line.is_empty() { 0 } else { 1 };
            if current_line.len() + word.len() + separator_len <= max_width {
                if !current_line.is_empty() {
                    current_line.push(' ');
                }
                current_line.push_str(word);
            } else {
                if !current_line.is_empty() {
                    lines.push(current_line);
                }
                current_line = word.to_string();
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        if lines.is_empty() {
            lines.push(text.to_string());
        }

        lines
    }
}

impl Default for ConversationPanel {
    fn default() -> Self {
        Self::new(1000) // Keep last 1000 messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_panel_creation() {
        let panel = ConversationPanel::new(100);
        assert_eq!(panel.messages.len(), 0);
        assert_eq!(panel.scroll_offset, 0);
    }

    #[test]
    fn test_add_messages() {
        let mut panel = ConversationPanel::new(100);

        panel.add_human_message("Hello agent!".to_string());
        assert_eq!(panel.messages.len(), 1);
        assert_eq!(panel.messages[0].sender, MessageSender::Human);

        panel.add_agent_message("Hello! How can I help?".to_string(), MessageType::Text);
        assert_eq!(panel.messages.len(), 2);
        assert_eq!(panel.messages[1].sender, MessageSender::Agent);

        panel.add_system_message("Agent initialized".to_string());
        assert_eq!(panel.messages.len(), 3);
        assert_eq!(panel.messages[2].sender, MessageSender::System);
    }

    #[test]
    fn test_max_messages_limit() {
        let mut panel = ConversationPanel::new(10);

        for i in 0..20 {
            panel.add_human_message(format!("Message {}", i));
        }

        assert_eq!(panel.messages.len(), 10);
        // Should keep most recent messages
        assert!(panel.messages[0].content.contains("10"));
    }

    #[test]
    fn test_scroll() {
        let mut panel = ConversationPanel::new(100);

        for i in 0..20 {
            panel.add_human_message(format!("Message {}", i));
        }

        assert_eq!(panel.scroll_offset, 10); // Auto-scrolled to bottom

        panel.scroll_up();
        assert_eq!(panel.scroll_offset, 9);

        panel.scroll_down();
        assert_eq!(panel.scroll_offset, 10);

        panel.scroll_to_bottom();
        assert_eq!(panel.scroll_offset, 10);
    }

    #[test]
    fn test_wrap_text() {
        let panel = ConversationPanel::new(100);

        let short_text = "Hello world";
        let wrapped = panel.wrap_text(short_text, 20);
        assert_eq!(wrapped.len(), 1);

        let long_text = "This is a very long message that should be wrapped into multiple lines";
        let wrapped = panel.wrap_text(long_text, 20);
        assert!(wrapped.len() > 1);
    }
}
