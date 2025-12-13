//! Approval Panel Widget for Human-in-the-Loop Decision Making
//!
//! This module provides an interactive approval panel that displays pending
//! action requests with risk assessment, context, and approval/rejection controls.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use fluent_agent::agent_control::{ApprovalRequest, RiskLevel};

/// Approval panel widget
pub struct ApprovalPanel {
    pub pending_approval: Option<ApprovalRequest>,
    pub selected_action: usize, // 0=Approve, 1=Reject, 2=View Details
}

impl ApprovalPanel {
    pub fn new() -> Self {
        Self {
            pending_approval: None,
            selected_action: 0,
        }
    }

    pub fn set_approval(&mut self, approval: ApprovalRequest) {
        self.pending_approval = Some(approval);
        self.selected_action = 0;
    }

    pub fn clear_approval(&mut self) {
        self.pending_approval = None;
    }

    pub fn has_pending_approval(&self) -> bool {
        self.pending_approval.is_some()
    }

    pub fn next_action(&mut self) {
        if self.selected_action < 2 {
            self.selected_action += 1;
        }
    }

    pub fn previous_action(&mut self) {
        if self.selected_action > 0 {
            self.selected_action -= 1;
        }
    }

    /// Render the approval panel
    pub fn render(&self, f: &mut Frame, area: Rect) {
        if let Some(ref approval) = self.pending_approval {
            self.render_approval(f, area, approval);
        } else {
            self.render_empty(f, area);
        }
    }

    fn render_approval(&self, f: &mut Frame, area: Rect, approval: &ApprovalRequest) {
        // Split into header, content, and controls
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(5),    // Content
                Constraint::Length(3), // Controls
            ])
            .split(area);

        // Header with warning
        self.render_header(f, chunks[0], approval);

        // Content with details
        self.render_content(f, chunks[1], approval);

        // Control buttons
        self.render_controls(f, chunks[2]);
    }

    fn render_header(&self, f: &mut Frame, area: Rect, approval: &ApprovalRequest) {
        let risk_color = match approval.risk_level {
            RiskLevel::Minimal => Color::Green,
            RiskLevel::Low => Color::Cyan,
            RiskLevel::Medium => Color::Yellow,
            RiskLevel::High => Color::LightRed,
            RiskLevel::Critical => Color::Red,
        };

        let risk_text = format!("{:?}", approval.risk_level);

        let header = Paragraph::new(vec![Line::from(vec![
            Span::styled("⚠️  ", Style::default().fg(Color::Yellow)),
            Span::styled(
                "ACTION REQUIRES APPROVAL",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" - Risk: ", Style::default().fg(Color::White)),
            Span::styled(
                risk_text,
                Style::default().fg(risk_color).add_modifier(Modifier::BOLD),
            ),
        ])])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .alignment(Alignment::Center);

        f.render_widget(header, area);
    }

    fn render_content(&self, f: &mut Frame, area: Rect, approval: &ApprovalRequest) {
        // Split into left (details) and right (context)
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Left: Action details
        let details_lines = vec![
            Line::from(vec![
                Span::styled("Type: ", Style::default().fg(Color::Cyan)),
                Span::styled(&approval.action_type, Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Description: ",
                Style::default().fg(Color::Cyan),
            )]),
            Line::from(Span::styled(
                &approval.action_description,
                Style::default().fg(Color::White),
            )),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Risk Factors:",
                Style::default().fg(Color::Cyan),
            )]),
        ];

        let mut all_lines = details_lines;
        for factor in &approval.risk_factors {
            all_lines.push(Line::from(vec![
                Span::styled("  • ", Style::default().fg(Color::Yellow)),
                Span::styled(factor, Style::default().fg(Color::White)),
            ]));
        }

        let details = Paragraph::new(all_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Action Details"),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(details, chunks[0]);

        // Right: Context and reasoning
        let mut context_lines = vec![
            Line::from(vec![Span::styled(
                "Reasoning:",
                Style::default().fg(Color::Cyan),
            )]),
            Line::from(Span::styled(
                &approval.context.reasoning,
                Style::default().fg(Color::White),
            )),
            Line::from(""),
        ];

        if !approval.context.affected_files.is_empty() {
            context_lines.push(Line::from(vec![Span::styled(
                "Affected Files:",
                Style::default().fg(Color::Cyan),
            )]));
            for file in &approval.context.affected_files {
                context_lines.push(Line::from(vec![
                    Span::styled("  📄 ", Style::default()),
                    Span::styled(file, Style::default().fg(Color::Yellow)),
                ]));
            }
            context_lines.push(Line::from(""));
        }

        if let Some(ref cmd) = approval.context.command {
            context_lines.push(Line::from(vec![Span::styled(
                "Command:",
                Style::default().fg(Color::Cyan),
            )]));
            context_lines.push(Line::from(vec![
                Span::styled("  $ ", Style::default().fg(Color::Green)),
                Span::styled(cmd, Style::default().fg(Color::White)),
            ]));
            context_lines.push(Line::from(""));
        }

        context_lines.push(Line::from(vec![
            Span::styled("Agent Recommends: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                &approval.context.agent_recommendation,
                Style::default().fg(Color::Green),
            ),
        ]));

        let context = Paragraph::new(context_lines)
            .block(Block::default().borders(Borders::ALL).title("Context"))
            .wrap(Wrap { trim: true });

        f.render_widget(context, chunks[1]);
    }

    fn render_controls(&self, f: &mut Frame, area: Rect) {
        let actions = ["[A]pprove", "[R]eject", "[V]iew Details"];
        let mut items = Vec::new();

        for (i, action) in actions.iter().enumerate() {
            let style = if i == self.selected_action {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            items.push(ListItem::new(*action).style(style));
        }

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Controls"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        f.render_widget(list, area);
    }

    fn render_empty(&self, f: &mut Frame, area: Rect) {
        let empty = Paragraph::new(vec![Line::from(vec![Span::styled(
            "No pending approvals",
            Style::default().fg(Color::Gray),
        )])])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Approval Panel"),
        )
        .alignment(Alignment::Center);

        f.render_widget(empty, area);
    }
}

impl Default for ApprovalPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// Render a compact approval indicator for the main view
pub fn render_approval_indicator(f: &mut Frame, area: Rect, has_pending: bool) {
    if has_pending {
        let indicator = Paragraph::new(vec![Line::from(vec![
            Span::styled("⚠️  ", Style::default().fg(Color::Yellow)),
            Span::styled(
                "APPROVAL REQUIRED",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
            Span::styled(
                " - Press 'A' to approve or 'R' to reject",
                Style::default().fg(Color::White),
            ),
        ])])
        .style(Style::default().bg(Color::DarkGray))
        .alignment(Alignment::Center);

        f.render_widget(indicator, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_agent::agent_control::{ApprovalContext, DefaultAction};
    use std::time::SystemTime;
    use uuid::Uuid;

    #[test]
    fn test_approval_panel_creation() {
        let panel = ApprovalPanel::new();
        assert!(!panel.has_pending_approval());
        assert_eq!(panel.selected_action, 0);
    }

    #[test]
    fn test_set_approval() {
        let mut panel = ApprovalPanel::new();

        let approval = ApprovalRequest {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            action_type: "file_write".to_string(),
            action_description: "Write to config.toml".to_string(),
            risk_level: RiskLevel::Medium,
            risk_factors: vec!["Overwrites file".to_string()],
            context: ApprovalContext {
                affected_files: vec!["config.toml".to_string()],
                command: None,
                code_changes: None,
                reasoning: "Need to update configuration".to_string(),
                alternatives: vec![],
                agent_recommendation: "Proceed with caution".to_string(),
            },
            timeout: None,
            default_action: DefaultAction::Reject,
            response_tx: None,
        };

        panel.set_approval(approval);
        assert!(panel.has_pending_approval());
    }

    #[test]
    fn test_action_navigation() {
        let mut panel = ApprovalPanel::new();

        assert_eq!(panel.selected_action, 0);
        panel.next_action();
        assert_eq!(panel.selected_action, 1);
        panel.next_action();
        assert_eq!(panel.selected_action, 2);
        panel.next_action(); // Should stay at 2
        assert_eq!(panel.selected_action, 2);

        panel.previous_action();
        assert_eq!(panel.selected_action, 1);
        panel.previous_action();
        assert_eq!(panel.selected_action, 0);
        panel.previous_action(); // Should stay at 0
        assert_eq!(panel.selected_action, 0);
    }
}
