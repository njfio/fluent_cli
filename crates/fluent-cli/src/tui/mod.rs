//! Terminal User Interface for agentic operations
//!
//! This module provides a rich, interactive TUI for monitoring and controlling
//! agentic workflows with progress bars, status displays, and real-time updates.

// Sub-modules
pub mod approval_panel;
pub mod collaborative_tui;
pub mod conversation;
pub mod input_modal;
pub mod simple_tui;

// Re-export key components
pub use approval_panel::ApprovalPanel;
pub use collaborative_tui::{CollaborativeTui, TuiState};
pub use conversation::{ConversationMessage, ConversationPanel, MessageSender, MessageType};
pub use input_modal::{InputModal, InputMode};
pub use simple_tui::SimpleTui;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::{
    io::{self, IsTerminal},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

/// Agent execution status
#[derive(Debug, Clone)]
pub enum AgentStatus {
    Initializing,
    Running,
    Paused,
    Completed,
    Failed(String),
    Timeout,
}

/// Agent execution state
#[derive(Debug, Clone)]
pub struct AgentState {
    pub status: AgentStatus,
    pub current_iteration: u32,
    pub max_iterations: u32,
    pub current_action: String,
    pub progress_percentage: u32,
    pub logs: Vec<String>,
    pub start_time: Instant,
    pub goal_description: String,
    pub tools_enabled: bool,
    pub reflection_enabled: bool,
    pub human_interventions: Vec<HumanIntervention>,
    pub awaiting_approval: bool,
    pub last_human_input: Option<String>,
    pub log_filter: Option<String>, // Filter logs by keyword
    pub estimated_time_remaining: Option<Duration>, // Estimated time remaining
    pub average_iteration_time: Option<Duration>, // Average time per iteration
}

/// Human intervention types
#[derive(Debug, Clone)]
pub enum HumanIntervention {
    Pause,
    Resume,
    Input(String),
    Approve,
    Reject,
    GoalModification(String),
    ParameterChange(String),
}

impl Default for AgentState {
    fn default() -> Self {
        Self {
            status: AgentStatus::Initializing,
            current_iteration: 0,
            max_iterations: 0,
            current_action: "Initializing...".to_string(),
            progress_percentage: 0,
            logs: Vec::new(),
            start_time: Instant::now(),
            goal_description: String::new(),
            tools_enabled: false,
            reflection_enabled: false,
            human_interventions: Vec::new(),
            awaiting_approval: false,
            last_human_input: None,
            log_filter: None,
            estimated_time_remaining: None,
            average_iteration_time: None,
        }
    }
}

/// TUI Application
pub struct AgentTui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    state: AgentState,
    should_quit: Arc<AtomicBool>,
    log_scroll: usize,
}

impl AgentTui {
    /// Create a new TUI instance
    pub fn new() -> Result<Self> {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            terminal,
            state: AgentState::default(),
            should_quit: Arc::new(AtomicBool::new(false)),
            log_scroll: 0,
        })
    }

    /// Initialize the TUI
    pub fn init(&mut self) -> Result<()> {
        // Check if we're in a TTY
        let is_tty = std::io::stdout().is_terminal();

        // Get terminal information
        let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
        let term = std::env::var("TERM").unwrap_or_default();

        // Check for known incompatible terminals
        let is_incompatible = term_program == "Apple_Terminal" || term == "dumb" || !is_tty;
        if is_incompatible {
            return Err(anyhow::anyhow!("Terminal not compatible with full TUI"));
        }

        // Try to enable raw mode
        enable_raw_mode().map_err(|e| anyhow::anyhow!("Raw mode not supported: {}", e))?;

        // Try to enter alternate screen
        execute!(
            self.terminal.backend_mut(),
            EnterAlternateScreen,
            EnableMouseCapture
        ).map_err(|e| {
            let _ = disable_raw_mode();
            anyhow::anyhow!("Alternate screen not supported: {}", e)
        })?;

        // Try to hide cursor
        self.terminal.hide_cursor().map_err(|e| {
            let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture);
            let _ = disable_raw_mode();
            anyhow::anyhow!("Cursor control not supported: {}", e)
        })?;

        Ok(())
    }

    /// Clean up the TUI
    pub fn cleanup(&mut self) -> Result<()> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    /// Update the agent state
    pub fn update_state(&mut self, new_state: AgentState) {
        self.state = new_state;
        // Auto-scroll to bottom when new logs are added
        if self.state.logs.len() > 10 {
            self.log_scroll = self.state.logs.len().saturating_sub(10);
        }
    }

    /// Run the TUI event loop
    pub async fn run(&mut self) -> Result<()> {
        let should_quit = self.should_quit.clone();

        loop {
            if should_quit.load(Ordering::Relaxed) {
                break;
            }

            // Create a copy of the state for drawing
            let state = self.state.clone();
            self.terminal
                .draw(|f| Self::draw_ui(f, &state, self.log_scroll))?;

            if crossterm::event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            should_quit.store(true, Ordering::Relaxed);
                            break;
                        }
                        KeyCode::Char('p') => {
                            // Toggle pause (would need to be implemented in agent)
                            self.add_log("Pause/Resume not yet implemented".to_string());
                        }
                        KeyCode::Up => {
                            if self.log_scroll > 0 {
                                self.log_scroll -= 1;
                            }
                        }
                        KeyCode::Down => {
                            let max_scroll = self.state.logs.len().saturating_sub(10);
                            if self.log_scroll < max_scroll {
                                self.log_scroll += 1;
                            }
                        }
                        KeyCode::PageUp => {
                            self.log_scroll = self.log_scroll.saturating_sub(10);
                        }
                        KeyCode::PageDown => {
                            let max_scroll = self.state.logs.len().saturating_sub(10);
                            self.log_scroll = (self.log_scroll + 10).min(max_scroll);
                        }
                        KeyCode::Char('f') => {
                            // Filter logs (would need input modal for filter text)
                            self.add_log("Log filtering: Press '/' to filter logs".to_string());
                        }
                        KeyCode::Char('/') => {
                            // Clear filter
                            self.set_log_filter(None);
                            self.add_log("Log filter cleared".to_string());
                        }
                        _ => {}
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        Ok(())
    }

    /// Draw the TUI interface with provided state (static method)
    fn draw_ui(f: &mut Frame, state: &AgentState, log_scroll: usize) {
        let size = f.size();

        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Length(4), // Status
                Constraint::Length(3), // Progress
                Constraint::Min(10),   // Logs
                Constraint::Length(3), // Footer
            ])
            .split(size);

        Self::draw_header(f, chunks[0], state);
        Self::draw_status(f, chunks[1], state);
        Self::draw_progress(f, chunks[2], state);
        Self::draw_logs(f, chunks[3], state, log_scroll);
        Self::draw_footer(f, chunks[4], state);
    }

    /// Draw the header with goal information
    fn draw_header(f: &mut Frame, area: Rect, state: &AgentState) {
        let header = Paragraph::new(vec![
            Line::from(vec![Span::styled(
                "🤖 Fluent Agentic Mode",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("Goal: ", Style::default().fg(Color::White)),
                Span::styled(&state.goal_description, Style::default().fg(Color::Yellow)),
            ]),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Agent Overview"),
        )
        .wrap(Wrap { trim: true });

        f.render_widget(header, area);
    }

    /// Draw the status panel
    fn draw_status(f: &mut Frame, area: Rect, state: &AgentState) {
        let status_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
            ])
            .split(area);

        // Status
        let status_text = match &state.status {
            AgentStatus::Initializing => "🔄 Initializing",
            AgentStatus::Running => "🚀 Running",
            AgentStatus::Paused => "⏸️  Paused",
            AgentStatus::Completed => "✅ Completed",
            AgentStatus::Failed(_) => "❌ Failed",
            AgentStatus::Timeout => "⏰ Timeout",
        };

        let status_color = match &state.status {
            AgentStatus::Initializing => Color::Yellow,
            AgentStatus::Running => Color::Green,
            AgentStatus::Paused => Color::Yellow,
            AgentStatus::Completed => Color::Green,
            AgentStatus::Failed(_) => Color::Red,
            AgentStatus::Timeout => Color::Red,
        };

        let status = Paragraph::new(status_text)
            .style(Style::default().fg(status_color))
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .alignment(Alignment::Center);

        f.render_widget(status, status_chunks[0]);

        // Iteration
        let iteration_text = format!("{}/{}", state.current_iteration, state.max_iterations);
        let iteration = Paragraph::new(iteration_text)
            .block(Block::default().borders(Borders::ALL).title("Iteration"))
            .alignment(Alignment::Center);

        f.render_widget(iteration, status_chunks[1]);

        // Elapsed time
        let elapsed = state.start_time.elapsed();
        let elapsed_text = format!(
            "{:02}:{:02}",
            elapsed.as_secs() / 60,
            elapsed.as_secs() % 60
        );
        let time = Paragraph::new(elapsed_text)
            .block(Block::default().borders(Borders::ALL).title("Elapsed"))
            .alignment(Alignment::Center);

        f.render_widget(time, status_chunks[2]);

        // Tools/Reflection status
        let features = vec![
            if state.tools_enabled { "🔧" } else { "⚪" },
            if state.reflection_enabled {
                "🧠"
            } else {
                "⚪"
            },
        ];
        let features_text = features.join(" ");
        let features_para = Paragraph::new(features_text)
            .block(Block::default().borders(Borders::ALL).title("Features"))
            .alignment(Alignment::Center);

        f.render_widget(features_para, status_chunks[3]);
    }

    /// Draw the progress bar
    fn draw_progress(f: &mut Frame, area: Rect, state: &AgentState) {
        let progress_label = if state.max_iterations > 0 {
            format!(
                "Iteration {}/{} ({:.0}%)",
                state.current_iteration, state.max_iterations, state.progress_percentage as f32
            )
        } else {
            format!("Iteration {}", state.current_iteration)
        };

        let time_info = if let Some(remaining) = state.estimated_time_remaining {
            let elapsed = state.start_time.elapsed();
            format!(
                "Elapsed: {:.1}s | Est. remaining: {:.1}s",
                elapsed.as_secs_f64(),
                remaining.as_secs_f64()
            )
        } else {
            format!("Elapsed: {:.1}s", state.start_time.elapsed().as_secs_f64())
        };

        let progress_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(area);

        let progress = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Progress"))
            .gauge_style(Style::default().fg(Color::Green))
            .percent(state.progress_percentage as u16)
            .label(progress_label);

        let time_paragraph = Paragraph::new(time_info)
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center);

        f.render_widget(progress, progress_chunks[0]);
        f.render_widget(time_paragraph, progress_chunks[1]);
    }

    /// Draw the logs panel with filtering support
    fn draw_logs(f: &mut Frame, area: Rect, state: &AgentState, log_scroll: usize) {
        // Get filtered logs if filter is set
        let logs_to_display: Vec<String> = if let Some(ref filter) = state.log_filter {
            state.logs
                .iter()
                .filter(|log| log.to_lowercase().contains(&filter.to_lowercase()))
                .cloned()
                .collect()
        } else {
            state.logs.clone()
        };

        let log_items: Vec<ListItem> = logs_to_display
            .iter()
            .skip(log_scroll)
            .take(10)
            .map(|log| ListItem::new(log.as_str()))
            .collect();

        let title = if state.log_filter.is_some() {
            format!("Logs ({}/{} filtered)", logs_to_display.len(), state.logs.len())
        } else {
            format!("Logs ({})", state.logs.len())
        };

        let logs = List::new(log_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        f.render_widget(logs, area);
    }

    /// Draw the footer with controls
    fn draw_footer(f: &mut Frame, area: Rect, state: &AgentState) {
        let footer = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Controls: ", Style::default().fg(Color::White)),
                Span::styled("↑/↓", Style::default().fg(Color::Cyan)),
                Span::styled(" Scroll • ", Style::default().fg(Color::White)),
                Span::styled("PgUp/PgDn", Style::default().fg(Color::Cyan)),
                Span::styled(" Page • ", Style::default().fg(Color::White)),
                Span::styled("Q", Style::default().fg(Color::Cyan)),
                Span::styled(" Quit", Style::default().fg(Color::White)),
                Span::styled(" • ", Style::default().fg(Color::White)),
                Span::styled("/", Style::default().fg(Color::Cyan)),
                Span::styled(" Filter", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("Current Action: ", Style::default().fg(Color::White)),
                Span::styled(&state.current_action, Style::default().fg(Color::Yellow)),
            ]),
        ])
        .wrap(Wrap { trim: true });

        f.render_widget(footer, area);
    }

    /// Add a log message
    pub fn add_log(&mut self, message: String) {
        let timestamp = chrono::Utc::now().format("%H:%M:%S");
        let log_entry = format!("[{}] {}", timestamp, message);
        self.state.logs.push(log_entry);
        // Auto-scroll to show new messages
        if self.state.logs.len() > 10 {
            self.log_scroll = self.state.logs.len() - 10;
        }
    }

    /// Add a streaming content chunk (for real-time LLM response display)
    pub fn add_streaming_chunk(&mut self, chunk: &str) {
        // Append to the last log entry if it's a streaming message, otherwise create new
        if let Some(last_log) = self.state.logs.last_mut() {
            if last_log.contains("🤖 Streaming: ") {
                // Remove the timestamp prefix and append chunk
                if let Some(idx) = last_log.find("🤖 Streaming: ") {
                    let base = &last_log[..idx + 14];
                    *last_log = format!("{}{}", base, chunk);
                } else {
                    last_log.push_str(chunk);
                }
            } else {
                let timestamp = chrono::Utc::now().format("%H:%M:%S");
                self.state.logs.push(format!("[{}] 🤖 Streaming: {}", timestamp, chunk));
            }
        } else {
            let timestamp = chrono::Utc::now().format("%H:%M:%S");
            self.state.logs.push(format!("[{}] 🤖 Streaming: {}", timestamp, chunk));
        }
        
        // Keep logs manageable
        if self.state.logs.len() > 100 {
            self.state.logs.remove(0);
        }
    }

    /// Start a new streaming response (clears previous streaming content)
    pub fn start_streaming(&mut self) {
        let timestamp = chrono::Utc::now().format("%H:%M:%S");
        self.state.logs.push(format!("[{}] 🤖 Streaming response...", timestamp));
    }

    /// Set the current action
    pub fn set_current_action(&mut self, action: String) {
        self.state.current_action = action;
    }

    /// Update progress
    pub fn update_progress(&mut self, percentage: u32) {
        self.state.progress_percentage = percentage;
    }

    /// Update status
    pub fn update_status(&mut self, status: AgentStatus) {
        self.state.status = status;
    }

    /// Set log filter
    pub fn set_log_filter(&mut self, filter: Option<String>) {
        self.state.log_filter = filter;
    }

    /// Get filtered logs
    pub fn get_filtered_logs(&self) -> Vec<String> {
        if let Some(ref filter) = self.state.log_filter {
            self.state.logs
                .iter()
                .filter(|log| log.to_lowercase().contains(&filter.to_lowercase()))
                .cloned()
                .collect()
        } else {
            self.state.logs.clone()
        }
    }

    /// Calculate estimated time remaining
    pub fn calculate_time_remaining(&mut self) {
        if self.state.current_iteration > 0 && self.state.max_iterations > 0 {
            let elapsed = self.state.start_time.elapsed();
            let avg_time_per_iteration = elapsed / self.state.current_iteration as u32;
            let remaining_iterations = self.state.max_iterations - self.state.current_iteration;
            self.state.estimated_time_remaining = Some(avg_time_per_iteration * remaining_iterations);
            self.state.average_iteration_time = Some(avg_time_per_iteration);
        }
    }

    /// Update iteration with time estimation
    pub fn update_iteration(&mut self, current: u32, max: u32) {
        self.state.current_iteration = current;
        self.state.max_iterations = max;
        self.state.progress_percentage = if max > 0 {
            (current as f32 / max as f32 * 100.0) as u32
        } else {
            0
        };
        self.calculate_time_remaining();
    }

    /// Set goal description
    pub fn set_goal(&mut self, goal: String) {
        self.state.goal_description = goal;
    }

    /// Set feature flags
    pub fn set_features(&mut self, tools_enabled: bool, reflection_enabled: bool) {
        self.state.tools_enabled = tools_enabled;
        self.state.reflection_enabled = reflection_enabled;
    }

    /// Get quit flag
    pub fn should_quit(&self) -> bool {
        self.should_quit.load(Ordering::Relaxed)
    }

    /// Signal quit
    pub fn quit(&self) {
        self.should_quit.store(true, Ordering::Relaxed);
    }
}

/// ASCII-based fallback TUI for terminals that don't support raw mode
pub struct AsciiTui {
    state: AgentState,
    should_quit: Arc<AtomicBool>,
    last_update: Instant,
    use_ansi: bool,
}

impl AsciiTui {
    pub fn new() -> Self {
        // Detect ANSI support
        let use_ansi = Self::detect_ansi_support();

        Self {
            state: AgentState::default(),
            should_quit: Arc::new(AtomicBool::new(false)),
            last_update: Instant::now(),
            use_ansi,
        }
    }

    /// Detect if the terminal supports ANSI escape sequences
    fn detect_ansi_support() -> bool {
        // Check environment variables that indicate ANSI support
        if std::env::var("NO_COLOR").is_ok() {
            return false;
        }

        if std::env::var("TERM").unwrap_or_default() == "dumb" {
            return false;
        }

        // Check if stdout is a TTY
        if !std::io::stdout().is_terminal() {
            return false;
        }

        // Check for CI environments that might not support ANSI
        if std::env::var("CI").is_ok() || std::env::var("CONTINUOUS_INTEGRATION").is_ok() {
            return false;
        }

        // Default to ANSI support for most modern terminals
        true
    }

    pub fn update_state(&mut self, new_state: AgentState) {
        self.state = new_state;
    }

    pub fn add_log(&mut self, message: String) {
        let timestamp = chrono::Utc::now().format("%H:%M:%S");
        let log_entry = format!("[{}] {}", timestamp, message);
        self.state.logs.push(log_entry);
        // Keep only last 20 logs for ASCII display
        if self.state.logs.len() > 20 {
            self.state.logs.remove(0);
        }
    }

    /// Add a streaming content chunk (for real-time LLM response display)
    pub fn add_streaming_chunk(&mut self, chunk: &str) {
        // Append to the last log entry if it's a streaming message, otherwise create new
        if let Some(last_log) = self.state.logs.last_mut() {
            if last_log.contains("🤖 Streaming: ") {
                last_log.push_str(chunk);
            } else {
                let timestamp = chrono::Utc::now().format("%H:%M:%S");
                self.state.logs.push(format!("[{}] 🤖 Streaming: {}", timestamp, chunk));
            }
        } else {
            let timestamp = chrono::Utc::now().format("%H:%M:%S");
            self.state.logs.push(format!("[{}] 🤖 Streaming: {}", timestamp, chunk));
        }
        
        // Keep only last 20 logs for ASCII display
        if self.state.logs.len() > 20 {
            self.state.logs.remove(0);
        }
    }

    /// Start a new streaming response (clears previous streaming content)
    pub fn start_streaming(&mut self) {
        let timestamp = chrono::Utc::now().format("%H:%M:%S");
        self.state.logs.push(format!("[{}] 🤖 Streaming response...", timestamp));
    }

    pub fn set_current_action(&mut self, action: String) {
        self.state.current_action = action;
    }

    pub fn update_progress(&mut self, percentage: u32) {
        self.state.progress_percentage = percentage;
    }

    pub fn update_status(&mut self, status: AgentStatus) {
        self.state.status = status;
    }

    pub fn update_iteration(&mut self, current: u32, max: u32) {
        self.state.current_iteration = current;
        self.state.max_iterations = max;
        self.state.progress_percentage = if max > 0 {
            (current as f32 / max as f32 * 100.0) as u32
        } else {
            0
        };
        // Calculate time estimates
        if current > 0 && max > 0 {
            let elapsed = self.state.start_time.elapsed();
            let avg_time_per_iteration = elapsed / current as u32;
            let remaining_iterations = max - current;
            self.state.estimated_time_remaining = Some(avg_time_per_iteration * remaining_iterations);
            self.state.average_iteration_time = Some(avg_time_per_iteration);
        }
    }

    pub fn set_goal(&mut self, goal: String) {
        self.state.goal_description = goal;
    }

    pub fn set_features(&mut self, tools_enabled: bool, reflection_enabled: bool) {
        self.state.tools_enabled = tools_enabled;
        self.state.reflection_enabled = reflection_enabled;
    }

    /// Set log filter
    pub fn set_log_filter(&mut self, filter: Option<String>) {
        self.state.log_filter = filter;
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit.load(Ordering::Relaxed)
    }

    pub fn quit(&self) {
        self.should_quit.store(true, Ordering::Relaxed);
    }
}

/// TUI Manager for coordinating TUI updates from agent execution
pub struct TuiManager {
    full_tui: Option<AgentTui>,
    simple_tui: Option<SimpleTui>,
    ascii_tui: Option<AsciiTui>,
    control_channel: Option<std::sync::Arc<fluent_agent::AgentControlChannel>>,
    enabled: bool,
    fallback_mode: bool,
    use_simple: bool,
}

impl TuiManager {
    pub fn new(enabled: bool) -> Self {
        // Use simple TUI by default (it actually works)
        let use_simple = std::env::var("FLUENT_USE_OLD_TUI").is_err();

        Self {
            full_tui: None,
            simple_tui: None,
            ascii_tui: None,
            control_channel: None,
            enabled,
            fallback_mode: false,
            use_simple,
        }
    }

    pub fn init(&mut self) -> Result<()> {
        if self.enabled {
            // Try SimpleTUI first (it actually works!)
            if self.use_simple {
                let channel = std::sync::Arc::new(fluent_agent::AgentControlChannel::new());
                match SimpleTui::new(Some(channel.clone())) {
                    Ok(tui) => {
                        self.simple_tui = Some(tui);
                        self.control_channel = Some(channel);
                        self.fallback_mode = false;
                        println!("✅ Full TUI initialized - interactive controls available");
                        return Ok(());
                    }
                    Err(e) => {
                        eprintln!("SimpleTUI failed: {}, falling back", e);
                        // Fall through to try old TUI
                    }
                }
            }

            // Try full TUI (old version)
            match AgentTui::new() {
                Ok(mut tui) => {
                    match tui.init() {
                        Ok(_) => {
                            self.full_tui = Some(tui);
                            self.fallback_mode = false;
                            println!("✅ Full TUI initialized - interactive controls available");
                            return Ok(());
                        }
                        Err(_) => {
                            // Silently fall back to ASCII mode
                        }
                    }
                }
                Err(_) => {
                    // Silently fall back to ASCII mode
                }
            }

            // Fall back to ASCII mode
            self.ascii_tui = Some(AsciiTui::new());
            self.fallback_mode = true;
            let ansi_status = if self.ascii_tui.as_ref().unwrap().use_ansi { "with colors" } else { "plain text" };
            println!("✅ ASCII interface initialized ({}) - Q=quit, S=status, H=help", ansi_status);
        } else {
            println!("📝 TUI disabled - using standard output");
        }
        Ok(())
    }

    pub fn cleanup(&mut self) -> Result<()> {
        if let Some(tui) = &mut self.full_tui {
            tui.cleanup()?;
        }
        // ASCII TUI doesn't need cleanup
        Ok(())
    }

    pub fn update_state(&mut self, state: AgentState) {
        if let Some(tui) = &mut self.full_tui {
            tui.update_state(state);
        } else if let Some(ascii) = &mut self.ascii_tui {
            ascii.update_state(state);
        }
    }

    pub fn add_log(&mut self, message: String) {
        // Send to SimpleTUI via control channel
        if let Some(ref channel) = self.control_channel {
            let _ = channel.state_tx.try_send(fluent_agent::agent_control::StateUpdate::log(
                fluent_agent::agent_control::LogLevel::Info,
                message.clone()
            ));
        }

        if self.enabled {
            if let Some(tui) = &mut self.full_tui {
                tui.add_log(message);
            } else if let Some(ascii) = &mut self.ascii_tui {
                ascii.add_log(message);
            }
        } else {
            // Fallback to stdout if TUI is disabled
            println!("{}", message);
        }
    }

    /// Add a streaming content chunk (for real-time LLM response display)
    pub fn add_streaming_chunk(&mut self, chunk: &str) {
        if self.enabled {
            if let Some(tui) = &mut self.full_tui {
                tui.add_streaming_chunk(chunk);
            } else if let Some(ascii) = &mut self.ascii_tui {
                ascii.add_streaming_chunk(chunk);
            }
        } else {
            // Fallback to stdout if TUI is disabled - print immediately
            print!("{}", chunk);
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }
    }

    /// Start a new streaming response (clears previous streaming content)
    pub fn start_streaming(&mut self) {
        if self.enabled {
            if let Some(tui) = &mut self.full_tui {
                tui.start_streaming();
            } else if let Some(ascii) = &mut self.ascii_tui {
                ascii.start_streaming();
            }
        }
    }

    /// Spawn SimpleTUI in a separate task and return the task handle
    pub fn spawn_simple_tui(&mut self) -> Option<tokio::task::JoinHandle<()>> {
        if let Some(mut tui) = self.simple_tui.take() {
            Some(tokio::spawn(async move {
                if let Err(e) = tui.run().await {
                    eprintln!("TUI error: {}", e);
                }
            }))
        } else {
            None
        }
    }

    pub fn set_current_action(&mut self, action: String) {
        if let Some(tui) = &mut self.full_tui {
            tui.set_current_action(action);
        } else if let Some(ascii) = &mut self.ascii_tui {
            ascii.set_current_action(action);
        }
    }

    pub fn update_progress(&mut self, percentage: u32) {
        if let Some(tui) = &mut self.full_tui {
            tui.update_progress(percentage);
        } else if let Some(ascii) = &mut self.ascii_tui {
            ascii.update_progress(percentage);
        }
    }

    pub fn update_status(&mut self, status: AgentStatus) {
        // Send to SimpleTUI via control channel
        if let Some(ref channel) = self.control_channel {
            let agent_status = match &status {
                AgentStatus::Initializing => fluent_agent::agent_control::AgentStatus::Initializing,
                AgentStatus::Running => fluent_agent::agent_control::AgentStatus::Running,
                AgentStatus::Paused => fluent_agent::agent_control::AgentStatus::Paused,
                AgentStatus::Completed => fluent_agent::agent_control::AgentStatus::Completed,
                AgentStatus::Failed(msg) => fluent_agent::agent_control::AgentStatus::Failed(msg.clone()),
                AgentStatus::Timeout => fluent_agent::agent_control::AgentStatus::Timeout,
            };
            let _ = channel.state_tx.try_send(fluent_agent::agent_control::StateUpdate::status_change(agent_status));
        }

        // Also update old TUIs if they're active
        if let Some(tui) = &mut self.full_tui {
            tui.update_status(status);
        } else if let Some(ascii) = &mut self.ascii_tui {
            ascii.update_status(status);
        }
    }

    pub fn update_iteration(&mut self, current: u32, max: u32) {
        // Send to SimpleTUI via control channel
        if let Some(ref channel) = self.control_channel {
            let progress = if max > 0 {
                ((current as f32 / max as f32) * 100.0) as u32
            } else {
                0
            };
            let _ = channel.state_tx.try_send(fluent_agent::agent_control::StateUpdate::iteration_update(
                current, max, progress
            ));
        }

        // Also update old TUIs
        if let Some(tui) = &mut self.full_tui {
            tui.update_iteration(current, max);
        } else if let Some(ascii) = &mut self.ascii_tui {
            ascii.update_iteration(current, max);
        }
    }

    pub fn set_goal(&mut self, goal: String) {
        if let Some(tui) = &mut self.full_tui {
            tui.set_goal(goal);
        } else if let Some(ascii) = &mut self.ascii_tui {
            ascii.set_goal(goal);
        }
    }

    pub fn set_features(&mut self, tools_enabled: bool, reflection_enabled: bool) {
        if let Some(tui) = &mut self.full_tui {
            tui.set_features(tools_enabled, reflection_enabled);
        } else if let Some(ascii) = &mut self.ascii_tui {
            ascii.set_features(tools_enabled, reflection_enabled);
        }
    }

    pub fn set_log_filter(&mut self, filter: Option<String>) {
        if let Some(tui) = &mut self.full_tui {
            tui.set_log_filter(filter.clone());
        } else if let Some(ascii) = &mut self.ascii_tui {
            ascii.set_log_filter(filter);
        }
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        if let Some(tui) = &mut self.full_tui {
            tui.run().await?;
        }
        // AsciiTui doesn't have a run method - it's managed differently
        Ok(())
    }

    pub fn should_quit(&self) -> bool {
        if let Some(tui) = &self.full_tui {
            tui.should_quit()
        } else if let Some(ascii) = &self.ascii_tui {
            ascii.should_quit()
        } else {
            false
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn is_fallback_mode(&self) -> bool {
        self.fallback_mode
    }

    /// Force display of current state (for ASCII TUI)
    pub fn force_display(&mut self) -> Result<()> {
        // ASCII TUI updates automatically via add_log and update_state
        Ok(())
    }

    /// Add human intervention
    pub fn add_human_intervention(&mut self, intervention: HumanIntervention) {
        if let Some(ascii) = &mut self.ascii_tui {
            ascii.state.human_interventions.push(intervention.clone());
        }

        match intervention {
            HumanIntervention::Pause => {
                self.update_status(AgentStatus::Paused);
                self.add_log("⏸️ Agent execution paused by user".to_string());
            }
            HumanIntervention::Resume => {
                self.update_status(AgentStatus::Running);
                self.add_log("▶️ Agent execution resumed by user".to_string());
            }
            HumanIntervention::Input(text) => {
                if let Some(ascii) = &mut self.ascii_tui {
                    ascii.state.last_human_input = Some(text.clone());
                }
                self.add_log(format!("💬 Human input: {}", text));
            }
            HumanIntervention::Approve => {
                if let Some(ascii) = &mut self.ascii_tui {
                    ascii.state.awaiting_approval = false;
                }
                self.add_log("✅ User approved current action".to_string());
            }
            HumanIntervention::Reject => {
                if let Some(ascii) = &mut self.ascii_tui {
                    ascii.state.awaiting_approval = false;
                }
                self.add_log("❌ User rejected current action".to_string());
            }
            HumanIntervention::GoalModification(new_goal) => {
                self.set_goal(new_goal.clone());
                self.add_log(format!("🎯 Goal modified to: {}", new_goal));
            }
            HumanIntervention::ParameterChange(param) => {
                self.add_log(format!("⚙️ Parameter changed: {}", param));
            }
        }
    }

    /// Check if agent is awaiting approval
    pub fn is_awaiting_approval(&self) -> bool {
        if let Some(ascii) = &self.ascii_tui {
            ascii.state.awaiting_approval
        } else {
            false
        }
    }

    /// Set awaiting approval state
    pub fn set_awaiting_approval(&mut self, awaiting: bool) {
        if let Some(ascii) = &mut self.ascii_tui {
            ascii.state.awaiting_approval = awaiting;
        }
    }
}
