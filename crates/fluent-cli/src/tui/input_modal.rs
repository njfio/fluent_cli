//! Modal Input System for Human Guidance
//!
//! This module provides an interactive modal input dialog for humans to provide
//! guidance, feedback, and instructions to the agent during execution.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Input mode types
#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,       // Not accepting input
    Guidance,     // Providing guidance
    GoalModify,   // Modifying goal
    Comment,      // Adding comment to approval
    RejectReason, // Providing rejection reason
}

/// Input modal state
pub struct InputModal {
    /// Whether the modal is active
    pub active: bool,
    /// Current input mode
    pub mode: InputMode,
    /// Current input buffer
    pub input: String,
    /// Cursor position in input
    pub cursor_position: usize,
    /// Prompt message
    pub prompt: String,
    /// Placeholder text
    pub placeholder: String,
    /// Context for the input
    pub context: Option<String>,
}

impl InputModal {
    pub fn new() -> Self {
        Self {
            active: false,
            mode: InputMode::Normal,
            input: String::new(),
            cursor_position: 0,
            prompt: String::new(),
            placeholder: String::new(),
            context: None,
        }
    }

    /// Activate the modal for guidance input
    pub fn activate_guidance(&mut self, context: Option<String>) {
        self.active = true;
        self.mode = InputMode::Guidance;
        self.input.clear();
        self.cursor_position = 0;
        self.prompt = "Provide guidance to the agent:".to_string();
        self.placeholder =
            "Enter guidance... (Ctrl+Enter=Send, Ctrl+Shift+Enter=Queue, Esc=Cancel)".to_string();
        self.context = context;
    }

    /// Activate the modal for goal modification
    pub fn activate_goal_modify(&mut self, current_goal: String) {
        self.active = true;
        self.mode = InputMode::GoalModify;
        self.input = current_goal;
        self.cursor_position = self.input.len();
        self.prompt = "Modify the agent's goal:".to_string();
        self.placeholder = "Enter new goal... (Ctrl+Enter=Apply, Esc=Cancel)".to_string();
        self.context = None;
    }

    /// Activate the modal for approval comment
    pub fn activate_comment(&mut self) {
        self.active = true;
        self.mode = InputMode::Comment;
        self.input.clear();
        self.cursor_position = 0;
        self.prompt = "Add comment (optional):".to_string();
        self.placeholder = "Enter comment... (Ctrl+Enter=Submit, Esc=Skip)".to_string();
        self.context = None;
    }

    /// Activate the modal for rejection reason
    pub fn activate_reject_reason(&mut self) {
        self.active = true;
        self.mode = InputMode::RejectReason;
        self.input.clear();
        self.cursor_position = 0;
        self.prompt = "Why are you rejecting this action?".to_string();
        self.placeholder = "Enter rejection reason... (Ctrl+Enter=Submit, Esc=Cancel)".to_string();
        self.context = None;
    }

    /// Deactivate the modal
    pub fn deactivate(&mut self) {
        self.active = false;
        self.mode = InputMode::Normal;
        self.input.clear();
        self.cursor_position = 0;
        self.context = None;
    }

    /// Add character to input
    pub fn add_char(&mut self, c: char) {
        self.input.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }

    /// Remove character before cursor
    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            self.input.remove(self.cursor_position - 1);
            self.cursor_position -= 1;
        }
    }

    /// Move cursor left
    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    /// Move cursor right
    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.input.len() {
            self.cursor_position += 1;
        }
    }

    /// Move cursor to start
    pub fn move_cursor_start(&mut self) {
        self.cursor_position = 0;
    }

    /// Move cursor to end
    pub fn move_cursor_end(&mut self) {
        self.cursor_position = self.input.len();
    }

    /// Delete word before cursor
    pub fn delete_word(&mut self) {
        if self.cursor_position == 0 {
            return;
        }

        let mut new_pos = self.cursor_position - 1;

        // Skip whitespace
        while new_pos > 0 && self.input.chars().nth(new_pos).unwrap().is_whitespace() {
            new_pos -= 1;
        }

        // Delete word
        while new_pos > 0 && !self.input.chars().nth(new_pos - 1).unwrap().is_whitespace() {
            new_pos -= 1;
        }

        self.input.drain(new_pos..self.cursor_position);
        self.cursor_position = new_pos;
    }

    /// Get the current input value
    pub fn get_input(&self) -> String {
        self.input.clone()
    }

    /// Clear the input
    pub fn clear_input(&mut self) {
        self.input.clear();
        self.cursor_position = 0;
    }

    /// Check if input is empty
    pub fn is_empty(&self) -> bool {
        self.input.is_empty()
    }

    /// Render the modal
    pub fn render(&self, f: &mut Frame, area: Rect) {
        if !self.active {
            return;
        }

        // Center the modal
        let modal_area = Self::centered_rect(80, 60, area);

        // Clear the background
        f.render_widget(Clear, modal_area);

        // Split into sections
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(5), // Context (if any)
                Constraint::Length(3), // Prompt
                Constraint::Min(5),    // Input area
                Constraint::Length(3), // Help text
            ])
            .split(modal_area);

        // Render title
        self.render_title(f, chunks[0]);

        // Render context if available
        if self.context.is_some() {
            self.render_context(f, chunks[1]);
        }

        // Render prompt
        self.render_prompt(f, chunks[2]);

        // Render input area
        self.render_input(f, chunks[3]);

        // Render help
        self.render_help(f, chunks[4]);
    }

    fn render_title(&self, f: &mut Frame, area: Rect) {
        let title = match self.mode {
            InputMode::Guidance => "💡 Provide Guidance",
            InputMode::GoalModify => "🎯 Modify Goal",
            InputMode::Comment => "💬 Add Comment",
            InputMode::RejectReason => "❌ Rejection Reason",
            InputMode::Normal => "Input",
        };

        let title_widget = Paragraph::new(title)
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            );

        f.render_widget(title_widget, area);
    }

    fn render_context(&self, f: &mut Frame, area: Rect) {
        if let Some(ref context) = self.context {
            let context_widget = Paragraph::new(context.as_str())
                .style(Style::default().fg(Color::Cyan))
                .wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::ALL).title("Context"));

            f.render_widget(context_widget, area);
        }
    }

    fn render_prompt(&self, f: &mut Frame, area: Rect) {
        let prompt_widget = Paragraph::new(self.prompt.as_str())
            .style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Left)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(prompt_widget, area);
    }

    fn render_input(&self, f: &mut Frame, area: Rect) {
        let input_text = if self.input.is_empty() {
            Span::styled(&self.placeholder, Style::default().fg(Color::DarkGray))
        } else {
            Span::styled(&self.input, Style::default().fg(Color::White))
        };

        let input_widget = Paragraph::new(Line::from(vec![input_text]))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green))
                    .title("Input"),
            );

        f.render_widget(input_widget, area);

        // Render cursor
        if !self.input.is_empty() {
            // Calculate cursor position (simplified - actual implementation would need proper positioning)
            let cursor_x = area.x + 2 + self.cursor_position as u16; // 2 for border
            let cursor_y = area.y + 1; // 1 for border

            if cursor_x < area.x + area.width - 1 && cursor_y < area.y + area.height - 1 {
                f.set_cursor(cursor_x.min(area.x + area.width - 2), cursor_y);
            }
        }
    }

    fn render_help(&self, f: &mut Frame, area: Rect) {
        let help_lines = vec![Line::from(vec![
            Span::styled(
                "Ctrl+Enter",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Send  "),
            Span::styled(
                "Ctrl+Shift+Enter",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Queue  "),
            Span::styled(
                "Esc",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Cancel  "),
            Span::styled(
                "Ctrl+W",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Delete Word"),
        ])];

        let help_widget = Paragraph::new(help_lines)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(help_widget, area);
    }

    /// Helper function to create a centered rectangle
    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }
}

impl Default for InputModal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_modal_creation() {
        let modal = InputModal::new();
        assert!(!modal.active);
        assert_eq!(modal.mode, InputMode::Normal);
        assert!(modal.input.is_empty());
    }

    #[test]
    fn test_activate_guidance() {
        let mut modal = InputModal::new();
        modal.activate_guidance(Some("Agent needs help with X".to_string()));

        assert!(modal.active);
        assert_eq!(modal.mode, InputMode::Guidance);
        assert_eq!(modal.context, Some("Agent needs help with X".to_string()));
    }

    #[test]
    fn test_input_operations() {
        let mut modal = InputModal::new();
        modal.activate_guidance(None);

        // Add characters
        modal.add_char('H');
        modal.add_char('i');
        assert_eq!(modal.input, "Hi");
        assert_eq!(modal.cursor_position, 2);

        // Delete character
        modal.delete_char();
        assert_eq!(modal.input, "H");
        assert_eq!(modal.cursor_position, 1);

        // Move cursor
        modal.move_cursor_left();
        assert_eq!(modal.cursor_position, 0);

        modal.add_char('W');
        assert_eq!(modal.input, "WH");
    }

    #[test]
    fn test_cursor_movement() {
        let mut modal = InputModal::new();
        modal.input = "Hello World".to_string();
        modal.cursor_position = 5;

        modal.move_cursor_right();
        assert_eq!(modal.cursor_position, 6);

        modal.move_cursor_left();
        assert_eq!(modal.cursor_position, 5);

        modal.move_cursor_start();
        assert_eq!(modal.cursor_position, 0);

        modal.move_cursor_end();
        assert_eq!(modal.cursor_position, 11);
    }

    #[test]
    fn test_delete_word() {
        let mut modal = InputModal::new();
        modal.input = "Hello World Test".to_string();
        modal.cursor_position = 16; // End

        modal.delete_word();
        assert_eq!(modal.input, "Hello World ");
        assert_eq!(modal.cursor_position, 12);

        modal.delete_word();
        assert_eq!(modal.input, "Hello ");
        assert_eq!(modal.cursor_position, 6);
    }

    #[test]
    fn test_deactivate() {
        let mut modal = InputModal::new();
        modal.activate_guidance(Some("Context".to_string()));
        modal.add_char('X');

        modal.deactivate();

        assert!(!modal.active);
        assert_eq!(modal.mode, InputMode::Normal);
        assert!(modal.input.is_empty());
        assert_eq!(modal.cursor_position, 0);
        assert!(modal.context.is_none());
    }

    #[test]
    fn test_mode_switching() {
        let mut modal = InputModal::new();

        modal.activate_guidance(None);
        assert_eq!(modal.mode, InputMode::Guidance);

        modal.activate_goal_modify("Current goal".to_string());
        assert_eq!(modal.mode, InputMode::GoalModify);
        assert_eq!(modal.input, "Current goal");

        modal.activate_comment();
        assert_eq!(modal.mode, InputMode::Comment);
        assert!(modal.input.is_empty());

        modal.activate_reject_reason();
        assert_eq!(modal.mode, InputMode::RejectReason);
    }
}
