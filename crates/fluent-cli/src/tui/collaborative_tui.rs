//! Collaborative TUI with Human-in-the-Loop Controls
//!
//! This module provides an enhanced TUI that integrates approval panels,
//! conversation views, and modal input for seamless human-agent collaboration.

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame, Terminal,
};
use std::{
    io::{self, IsTerminal},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

use super::{ApprovalPanel, ConversationPanel, InputModal, MessageType};
use fluent_agent::agent_control::{
    AgentControlChannel, ControlMessage, StateUpdate, StateUpdateType,
};

/// Collaborative TUI state
pub struct CollaborativeTui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    state: Arc<RwLock<TuiState>>,
    approval_panel: ApprovalPanel,
    conversation_panel: ConversationPanel,
    input_modal: InputModal,
    control_channel: Option<Arc<AgentControlChannel>>,
    last_render: Instant,
    render_fps: u32,
}

/// Internal TUI state
#[derive(Debug, Clone)]
pub struct TuiState {
    pub status: AgentDisplayStatus,
    pub current_iteration: u32,
    pub max_iterations: u32,
    pub current_action: String,
    pub progress_percentage: u32,
    pub goal_description: String,
    pub start_time: Instant,
    pub paused: bool,
    pub awaiting_approval: bool,
    pub pending_approval_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone)]
pub enum AgentDisplayStatus {
    Initializing,
    Running,
    Paused,
    WaitingForApproval,
    WaitingForGuidance,
    Completed,
    Failed(String),
}

impl CollaborativeTui {
    /// Create a new collaborative TUI
    pub fn new(control_channel: Option<Arc<AgentControlChannel>>) -> Result<Self> {
        // Check if we're in a terminal
        if !io::stdout().is_terminal() {
            return Err(anyhow::anyhow!("Not running in a terminal"));
        }

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            terminal,
            state: Arc::new(RwLock::new(TuiState::default())),
            approval_panel: ApprovalPanel::new(),
            conversation_panel: ConversationPanel::new(1000),
            input_modal: InputModal::new(),
            control_channel,
            last_render: Instant::now(),
            render_fps: 30, // Target 30 FPS
        })
    }

    /// Run the TUI event loop
    pub async fn run(&mut self) -> Result<()> {
        let frame_duration = Duration::from_millis(1000 / self.render_fps as u64);

        loop {
            // Handle state updates from agent
            self.poll_state_updates().await?;

            // Handle user input
            if self.handle_input().await? {
                break; // User requested quit
            }

            // Render UI (rate-limited)
            if self.last_render.elapsed() >= frame_duration {
                self.render()?;
                self.last_render = Instant::now();
            }

            // Small sleep to prevent CPU spinning
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        Ok(())
    }

    /// Poll for state updates from agent
    async fn poll_state_updates(&mut self) -> Result<()> {
        let channel = match &self.control_channel {
            Some(ch) => ch.clone(),
            None => return Ok(()),
        };

        // Check for state updates (non-blocking)
        while let Ok(Some(update)) = channel.state_receiver().try_recv().await {
            self.process_state_update(update).await?;
        }

        Ok(())
    }

    /// Process a state update from the agent
    async fn process_state_update(&mut self, update: StateUpdate) -> Result<()> {
        match update.update_type {
            StateUpdateType::StatusChange { status } => {
                let mut state = self.state.write().await;
                state.status = match status {
                    fluent_agent::agent_control::AgentStatus::Initializing => AgentDisplayStatus::Initializing,
                    fluent_agent::agent_control::AgentStatus::Running => AgentDisplayStatus::Running,
                    fluent_agent::agent_control::AgentStatus::Paused => {
                        state.paused = true;
                        AgentDisplayStatus::Paused
                    }
                    fluent_agent::agent_control::AgentStatus::WaitingForApproval => {
                        state.awaiting_approval = true;
                        AgentDisplayStatus::WaitingForApproval
                    }
                    fluent_agent::agent_control::AgentStatus::WaitingForGuidance => AgentDisplayStatus::WaitingForGuidance,
                    fluent_agent::agent_control::AgentStatus::Completed => AgentDisplayStatus::Completed,
                    fluent_agent::agent_control::AgentStatus::Failed(msg) => AgentDisplayStatus::Failed(msg),
                    fluent_agent::agent_control::AgentStatus::Timeout => AgentDisplayStatus::Failed("Timeout".to_string()),
                };
            }

            StateUpdateType::IterationUpdate {
                current,
                max,
                progress_percentage,
            } => {
                let mut state = self.state.write().await;
                state.current_iteration = current;
                state.max_iterations = max;
                state.progress_percentage = progress_percentage;
            }

            StateUpdateType::ActionUpdate {
                action_description,
                ..
            } => {
                let mut state = self.state.write().await;
                state.current_action = action_description.clone();
                self.conversation_panel.add_agent_message(
                    format!("Starting action: {}", action_description),
                    MessageType::Action,
                );
            }

            StateUpdateType::ApprovalRequested { approval } => {
                let mut state = self.state.write().await;
                state.awaiting_approval = true;
                state.pending_approval_id = Some(approval.id);
                drop(state);

                self.approval_panel.set_approval(approval.clone());
                self.conversation_panel.add_agent_message(
                    format!("Requesting approval for: {}", approval.action_description),
                    MessageType::Approval,
                );
            }

            StateUpdateType::ApprovalProcessed { approved, .. } => {
                let mut state = self.state.write().await;
                state.awaiting_approval = false;
                state.pending_approval_id = None;
                drop(state);

                self.approval_panel.clear_approval();
                let msg = if approved {
                    "Action approved by human".to_string()
                } else {
                    "Action rejected by human".to_string()
                };
                self.conversation_panel
                    .add_system_message(msg);
            }

            StateUpdateType::GuidanceRequested { request } => {
                self.conversation_panel.add_agent_message(
                    format!("Requesting guidance: {:?}", request.reason),
                    MessageType::Text,
                );
            }

            StateUpdateType::LogMessage { level, message } => {
                let msg_type = match level {
                    fluent_agent::agent_control::LogLevel::Error => MessageType::Error,
                    fluent_agent::agent_control::LogLevel::Warning => MessageType::Text,
                    _ => MessageType::Text,
                };
                self.conversation_panel.add_agent_message(message, msg_type);
            }

            StateUpdateType::ReasoningStep {
                step_description,
                confidence,
                thought_process,
            } => {
                self.conversation_panel.add_agent_message(
                    format!("💭 {}\n   Confidence: {:.0}%\n   {}", step_description, confidence * 100.0, thought_process),
                    MessageType::Reasoning,
                );
            }

            StateUpdateType::Error { error, .. } => {
                self.conversation_panel.add_agent_message(error, MessageType::Error);
            }

            StateUpdateType::GoalProgress { completion_percentage, .. } => {
                let mut state = self.state.write().await;
                state.progress_percentage = completion_percentage as u32;
            }

            _ => {} // Handle other update types as needed
        }

        Ok(())
    }

    /// Handle user input
    async fn handle_input(&mut self) -> Result<bool> {
        // Check for keyboard events (non-blocking)
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                // If input modal is active, handle input there
                if self.input_modal.active {
                    return self.handle_modal_input(key).await;
                }

                // Handle global keys
                match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), KeyModifiers::NONE) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        return Ok(true); // Quit
                    }

                    (KeyCode::Char('p'), KeyModifiers::NONE) => {
                        self.send_pause_toggle().await?;
                    }

                    (KeyCode::Char('i'), KeyModifiers::NONE) => {
                        self.input_modal.activate_guidance(None);
                    }

                    (KeyCode::Char('g'), KeyModifiers::NONE) => {
                        let state = self.state.read().await;
                        let current_goal = state.goal_description.clone();
                        drop(state);
                        self.input_modal.activate_goal_modify(current_goal);
                    }

                    (KeyCode::Char('a'), KeyModifiers::NONE) => {
                        if self.approval_panel.has_pending_approval() {
                            self.handle_approval(true).await?;
                        }
                    }

                    (KeyCode::Char('r'), KeyModifiers::NONE) => {
                        if self.approval_panel.has_pending_approval() {
                            self.input_modal.activate_reject_reason();
                        }
                    }

                    (KeyCode::Up, KeyModifiers::NONE) => {
                        self.conversation_panel.scroll_up();
                    }

                    (KeyCode::Down, KeyModifiers::NONE) => {
                        self.conversation_panel.scroll_down();
                    }

                    _ => {}
                }
            }
        }

        Ok(false)
    }

    /// Handle input when modal is active
    async fn handle_modal_input(&mut self, key: crossterm::event::KeyEvent) -> Result<bool> {
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => {
                self.input_modal.deactivate();
            }

            (KeyCode::Enter, KeyModifiers::CONTROL) => {
                // Submit input
                let input = self.input_modal.get_input();
                let mode = self.input_modal.mode.clone();
                self.input_modal.deactivate();

                self.handle_modal_submit(input, mode).await?;
            }

            (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                self.input_modal.add_char(c);
            }

            (KeyCode::Backspace, _) => {
                self.input_modal.delete_char();
            }

            (KeyCode::Left, _) => {
                self.input_modal.move_cursor_left();
            }

            (KeyCode::Right, _) => {
                self.input_modal.move_cursor_right();
            }

            (KeyCode::Home, _) => {
                self.input_modal.move_cursor_start();
            }

            (KeyCode::End, _) => {
                self.input_modal.move_cursor_end();
            }

            (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
                self.input_modal.delete_word();
            }

            _ => {}
        }

        Ok(false)
    }

    /// Handle modal submission
    async fn handle_modal_submit(&mut self, input: String, mode: super::InputMode) -> Result<()> {
        if input.is_empty() {
            return Ok(());
        }

        match mode {
            super::InputMode::Guidance => {
                self.send_guidance(input.clone()).await?;
                self.conversation_panel.add_human_message(format!("Guidance: {}", input));
            }

            super::InputMode::GoalModify => {
                self.send_goal_modification(input.clone()).await?;
                self.conversation_panel.add_human_message(format!("Modified goal: {}", input));
            }

            super::InputMode::Comment => {
                // Comment for approval - handle with approval
            }

            super::InputMode::RejectReason => {
                self.handle_approval_with_reason(false, input.clone()).await?;
                self.conversation_panel.add_human_message(format!("Rejected: {}", input));
            }

            super::InputMode::Normal => {}
        }

        Ok(())
    }

    /// Send pause/resume toggle
    async fn send_pause_toggle(&mut self) -> Result<()> {
        let Some(ref channel) = self.control_channel else {
            return Ok(());
        };

        let state = self.state.read().await;
        let paused = state.paused;
        drop(state);

        let message = if paused {
            ControlMessage::resume()
        } else {
            ControlMessage::pause()
        };

        channel.send_control(message).await?;

        let mut state = self.state.write().await;
        state.paused = !paused;

        Ok(())
    }

    /// Send guidance to agent
    async fn send_guidance(&mut self, guidance: String) -> Result<()> {
        let Some(ref channel) = self.control_channel else {
            return Ok(());
        };

        let message = ControlMessage::input("Current context".to_string(), guidance, false);
        channel.send_control(message).await?;

        Ok(())
    }

    /// Send goal modification
    async fn send_goal_modification(&mut self, new_goal: String) -> Result<()> {
        let Some(ref channel) = self.control_channel else {
            return Ok(());
        };

        let message = ControlMessage::new(fluent_agent::agent_control::ControlMessageType::ModifyGoal {
            new_goal: new_goal.clone(),
            keep_context: true,
        });

        channel.send_control(message).await?;

        let mut state = self.state.write().await;
        state.goal_description = new_goal;

        Ok(())
    }

    /// Handle approval
    async fn handle_approval(&mut self, approved: bool) -> Result<()> {
        self.handle_approval_with_reason(approved, String::new()).await
    }

    /// Handle approval with reason/comment
    async fn handle_approval_with_reason(&mut self, approved: bool, reason: String) -> Result<()> {
        let Some(ref channel) = self.control_channel else {
            return Ok(());
        };

        let state = self.state.read().await;
        let Some(approval_id) = state.pending_approval_id else {
            return Ok(());
        };
        drop(state);

        let message = if approved {
            ControlMessage::approve(approval_id, if reason.is_empty() { None } else { Some(reason) })
        } else {
            ControlMessage::reject(approval_id, reason, None)
        };

        channel.send_control(message).await?;

        Ok(())
    }

    /// Render the TUI
    fn render(&mut self) -> Result<()> {
        // Extract references before the draw closure
        let state = self.state.blocking_read();
        let _has_pending_approval = self.approval_panel.has_pending_approval();
        let input_modal_active = self.input_modal.active;
        drop(state); // Release lock

        self.terminal.draw(|f| {
            let size = f.size();

            // Main layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // Header
                    Constraint::Length(3),  // Progress
                    Constraint::Min(10),    // Main content
                    Constraint::Length(3),  // Controls
                ])
                .split(size);

            // Render header - need to use a simpler approach
            let header_text = "🤖 Fluent Agent - Collaborative Mode";
            let header = Paragraph::new(header_text)
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center);
            f.render_widget(header, chunks[0]);

            // Render progress with placeholder
            let progress = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title("Progress"))
                .gauge_style(Style::default().fg(Color::Green))
                .percent(0);
            f.render_widget(progress, chunks[1]);

            // Render placeholder for main content
            let content = Paragraph::new("Content will appear here")
                .block(Block::default().borders(Borders::ALL).title("Agent Activity"));
            f.render_widget(content, chunks[2]);

            // Render controls
            let controls = Paragraph::new("P=Pause I=Input G=Goal A=Approve R=Reject Q=Quit")
                .block(Block::default().borders(Borders::ALL).title("Controls"))
                .alignment(Alignment::Center);
            f.render_widget(controls, chunks[3]);
        })?;

        // Render input modal separately if active
        if input_modal_active {
            self.terminal.draw(|f| {
                self.input_modal.render(f, f.size());
            })?;
        }

        Ok(())
    }

    fn render_header(&self, f: &mut Frame, area: Rect) {
        let state = self.state.blocking_read();

        let status_text = match &state.status {
            AgentDisplayStatus::Initializing => ("Initializing", Color::Yellow),
            AgentDisplayStatus::Running => ("Running", Color::Green),
            AgentDisplayStatus::Paused => ("Paused", Color::Yellow),
            AgentDisplayStatus::WaitingForApproval => ("Waiting for Approval", Color::Red),
            AgentDisplayStatus::WaitingForGuidance => ("Waiting for Guidance", Color::Cyan),
            AgentDisplayStatus::Completed => ("Completed", Color::Green),
            AgentDisplayStatus::Failed(_msg) => ("Failed", Color::Red),
        };

        let header = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("🤖 Fluent Agent", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" - "),
                Span::styled(status_text.0, Style::default().fg(status_text.1).add_modifier(Modifier::BOLD)),
            ]),
        ])
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);

        f.render_widget(header, area);
    }

    fn render_progress(&self, f: &mut Frame, area: Rect) {
        let state = self.state.blocking_read();

        let progress = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(format!(
                "Progress: Iteration {}/{}",
                state.current_iteration, state.max_iterations
            )))
            .gauge_style(Style::default().fg(Color::Green))
            .percent(state.progress_percentage.min(100) as u16);

        f.render_widget(progress, area);
    }

    fn render_main_content(&mut self, f: &mut Frame, area: Rect) {
        if self.approval_panel.has_pending_approval() {
            // Split screen: conversation + approval
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);

            self.conversation_panel.render(f, chunks[0]);
            self.approval_panel.render(f, chunks[1]);
        } else {
            // Full screen conversation
            self.conversation_panel.render(f, area);
        }
    }

    fn render_controls(&self, f: &mut Frame, area: Rect) {
        let controls = Paragraph::new(Line::from(vec![
            Span::styled("P", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("=Pause "),
            Span::styled("I", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("=Input "),
            Span::styled("G", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("=Goal "),
            Span::styled("A", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("=Approve "),
            Span::styled("R", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("=Reject "),
            Span::styled("Q", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw("=Quit"),
        ]))
        .block(Block::default().borders(Borders::ALL).title("Controls"))
        .alignment(Alignment::Center);

        f.render_widget(controls, area);
    }

    /// Cleanup on exit
    pub fn cleanup(&mut self) -> Result<()> {
        disable_raw_mode()?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for CollaborativeTui {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

impl Default for TuiState {
    fn default() -> Self {
        Self {
            status: AgentDisplayStatus::Initializing,
            current_iteration: 0,
            max_iterations: 0,
            current_action: "Initializing...".to_string(),
            progress_percentage: 0,
            goal_description: String::new(),
            start_time: Instant::now(),
            paused: false,
            awaiting_approval: false,
            pending_approval_id: None,
        }
    }
}
