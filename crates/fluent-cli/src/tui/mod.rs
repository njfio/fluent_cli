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
        }
    }
}

/// TUI Application
pub struct AgentTui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    state: AgentState,
    should_quit: Arc<AtomicBool>,
    log_scroll: usize,
    show_help: bool,
    last_frame_ms: u32,
    run_id: String,
    log_persist_path: Option<std::path::PathBuf>,
    max_logs: usize,
    control_channel: Option<std::sync::Arc<fluent_agent::AgentControlChannel>>,
}

impl AgentTui {
    /// Create a new TUI instance
    pub fn new(
        control_channel: Option<std::sync::Arc<fluent_agent::AgentControlChannel>>,
    ) -> Result<Self> {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        let run_id = std::env::var("FLUENT_RUN_ID")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                format!("{}-{}", ts, std::process::id())
            });
        let base_dir = std::env::var("FLUENT_STATE_STORE")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "./agent_logs".to_string());
        let mut path = std::path::PathBuf::from(base_dir);
        path.push("agent_logs");
        let _ = std::fs::create_dir_all(&path);
        path.push(format!("{}.log", run_id));
        let max_logs = std::env::var("FLUENT_TUI_MAX_LOGS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v >= 10)
            .unwrap_or(200);

        Ok(Self {
            terminal,
            state: AgentState::default(),
            should_quit: Arc::new(AtomicBool::new(false)),
            log_scroll: 0,
            show_help: false,
            last_frame_ms: 0,
            run_id,
            log_persist_path: Some(path),
            max_logs,
            control_channel,
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
        )
        .map_err(|e| {
            let _ = disable_raw_mode();
            anyhow::anyhow!("Alternate screen not supported: {}", e)
        })?;

        // Try to hide cursor
        self.terminal.hide_cursor().map_err(|e| {
            let _ = execute!(
                self.terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            );
            let _ = disable_raw_mode();
            anyhow::anyhow!("Cursor control not supported: {}", e)
        })?;

        Ok(())
    }

    /// Clean up the TUI
    pub fn cleanup(&mut self) -> Result<()> {
        if let Some(path) = &self.log_persist_path {
            let content = self.state.logs.join("\n");
            let _ = std::fs::write(path, content);
        }
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
            let render_start = Instant::now();
            self.terminal.draw(|f| {
                Self::draw_ui(
                    f,
                    &state,
                    self.log_scroll,
                    self.show_help,
                    self.last_frame_ms,
                    &self.run_id,
                )
            })?;
            let elapsed = render_start.elapsed();
            self.last_frame_ms = elapsed.as_millis() as u32;

            if crossterm::event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            should_quit.store(true, Ordering::Relaxed);
                            break;
                        }
                        KeyCode::Char('p') => {
                            if let Some(ref channel) = self.control_channel {
                                match self.state.status {
                                    AgentStatus::Paused => {
                                        let _ = channel
                                            .send_control(
                                                fluent_agent::agent_control::ControlMessage::resume(
                                                ),
                                            )
                                            .await;
                                    }
                                    _ => {
                                        let _ = channel
                                            .send_control(
                                                fluent_agent::agent_control::ControlMessage::pause(
                                                ),
                                            )
                                            .await;
                                    }
                                }
                            }
                        }
                        KeyCode::Char('h') => {
                            self.show_help = !self.show_help;
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
                        _ => {}
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        Ok(())
    }

    /// Draw the TUI interface with provided state (static method)
    fn draw_ui(
        f: &mut Frame,
        state: &AgentState,
        log_scroll: usize,
        show_help: bool,
        frame_ms: u32,
        run_id: &str,
    ) {
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

        Self::draw_header(f, chunks[0], state, run_id);
        Self::draw_status(f, chunks[1], state, frame_ms);
        Self::draw_progress(f, chunks[2], state);
        if show_help {
            Self::draw_help(f, chunks[3]);
        } else {
            Self::draw_logs(f, chunks[3], state, log_scroll);
        }
        Self::draw_footer(f, chunks[4], state, frame_ms);
    }

    /// Draw the header with goal information
    fn draw_header(f: &mut Frame, area: Rect, state: &AgentState, run_id: &str) {
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
            Line::from(vec![
                Span::styled("Run: ", Style::default().fg(Color::White)),
                Span::styled(run_id, Style::default().fg(Color::Magenta)),
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
    fn draw_status(f: &mut Frame, area: Rect, state: &AgentState, frame_ms: u32) {
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
        let fps = if frame_ms > 0 { 1000 / frame_ms } else { 0 };
        let perf_text = format!("{} • {}ms (~{} FPS)", elapsed_text, frame_ms, fps);
        let time = Paragraph::new(perf_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Elapsed • Perf"),
            )
            .alignment(Alignment::Center);

        f.render_widget(time, status_chunks[2]);

        // Tools/Reflection status
        let features = [
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
        let progress = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Progress"))
            .gauge_style(Style::default().fg(Color::Green))
            .percent(state.progress_percentage as u16)
            .label(format!("{}%", state.progress_percentage));

        f.render_widget(progress, area);
    }

    /// Draw the logs panel
    fn draw_logs(f: &mut Frame, area: Rect, state: &AgentState, log_scroll: usize) {
        let log_items: Vec<ListItem> = state
            .logs
            .iter()
            .skip(log_scroll)
            .take(10)
            .map(|log| ListItem::new(log.as_str()))
            .collect();

        let logs = List::new(log_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Logs ({})", state.logs.len())),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        f.render_widget(logs, area);
    }

    fn draw_help(f: &mut Frame, area: Rect) {
        let para = Paragraph::new(vec![
            Line::from(vec![Span::raw("Controls:")]),
            Line::from(vec![Span::raw("  ↑/↓ Scroll • PgUp/PgDn Page")]),
            Line::from(vec![Span::raw("  P Pause/Resume • Q Quit • H Help")]),
        ])
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .alignment(Alignment::Left);
        f.render_widget(para, area);
    }

    /// Draw the footer with controls
    fn draw_footer(f: &mut Frame, area: Rect, state: &AgentState, frame_ms: u32) {
        let fps = if frame_ms > 0 { 1000 / frame_ms } else { 0 };
        let footer = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Controls: ", Style::default().fg(Color::White)),
                Span::styled("↑/↓", Style::default().fg(Color::Cyan)),
                Span::styled(" Scroll • ", Style::default().fg(Color::White)),
                Span::styled("PgUp/PgDn", Style::default().fg(Color::Cyan)),
                Span::styled(" Page • ", Style::default().fg(Color::White)),
                Span::styled("Q", Style::default().fg(Color::Cyan)),
                Span::styled(" Quit", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("Current Action: ", Style::default().fg(Color::White)),
                Span::styled(&state.current_action, Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::styled("Frame: ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("{}ms (~{} FPS)", frame_ms, fps),
                    Style::default().fg(Color::Cyan),
                ),
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
        let len = self.state.logs.len();
        if len > self.max_logs {
            let remove = len - self.max_logs;
            self.state.logs.drain(0..remove);
        }
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

    /// Update iteration
    pub fn update_iteration(&mut self, current: u32, max: u32) {
        self.state.current_iteration = current;
        self.state.max_iterations = max;
        self.state.progress_percentage = if max > 0 {
            (current as f32 / max as f32 * 100.0) as u32
        } else {
            0
        };
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
    run_id: String,
    log_persist_path: Option<std::path::PathBuf>,
    max_logs: usize,
}

impl Default for AsciiTui {
    fn default() -> Self {
        Self::new()
    }
}

impl AsciiTui {
    pub fn new() -> Self {
        // Detect ANSI support
        let use_ansi = Self::detect_ansi_support();

        let run_id = std::env::var("FLUENT_RUN_ID")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                format!("{}-{}", ts, std::process::id())
            });
        let base_dir = std::env::var("FLUENT_STATE_STORE")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "./agent_logs".to_string());
        let mut path = std::path::PathBuf::from(base_dir);
        path.push("agent_logs");
        let _ = std::fs::create_dir_all(&path);
        path.push(format!("{}.log", run_id));

        let max_logs = std::env::var("FLUENT_TUI_MAX_LOGS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v >= 10)
            .unwrap_or(200);

        Self {
            state: AgentState::default(),
            should_quit: Arc::new(AtomicBool::new(false)),
            last_update: Instant::now(),
            use_ansi,
            run_id,
            log_persist_path: Some(path),
            max_logs,
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
        let len = self.state.logs.len();
        if len > self.max_logs {
            let remove = len - self.max_logs;
            self.state.logs.drain(0..remove);
        }
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
    }

    pub fn set_goal(&mut self, goal: String) {
        self.state.goal_description = goal;
    }

    pub fn set_features(&mut self, tools_enabled: bool, reflection_enabled: bool) {
        self.state.tools_enabled = tools_enabled;
        self.state.reflection_enabled = reflection_enabled;
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit.load(Ordering::Relaxed)
    }

    pub fn quit(&self) {
        self.should_quit.store(true, Ordering::Relaxed);
    }

    pub async fn run(&mut self) -> Result<()> {
        let should_quit = self.should_quit.clone();

        // Initial state already printed by run_event_loop
        self.last_update = Instant::now();

        loop {
            if should_quit.load(Ordering::Relaxed) {
                break;
            }

            // Update status periodically
            if self.last_update.elapsed() >= Duration::from_millis(1000) {
                self.print_status_update(false)?;
                self.last_update = Instant::now();
            }

            // Check for keyboard input (non-blocking)
            if crossterm::event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            should_quit.store(true, Ordering::Relaxed);
                            break;
                        }
                        KeyCode::Char('p') => {
                            // Toggle pause/resume - this will be handled by the TuiManager
                            // For now, just show a message
                            self.add_log("⏸️ Pause/Resume requested (will be implemented in agent integration)".to_string());
                            self.print_status_update(false)?;
                        }
                        KeyCode::Char('i') => {
                            // Human input
                            self.handle_human_input()?;
                            self.print_status_update(false)?;
                        }
                        KeyCode::Char('a') => {
                            // Approve current action
                            self.state
                                .human_interventions
                                .push(HumanIntervention::Approve);
                            self.state.awaiting_approval = false;
                            self.add_log("✅ User approved current action".to_string());
                            self.print_status_update(false)?;
                        }
                        KeyCode::Char('r') => {
                            // Reject current action
                            self.state
                                .human_interventions
                                .push(HumanIntervention::Reject);
                            self.state.awaiting_approval = false;
                            self.add_log("❌ User rejected current action".to_string());
                            self.print_status_update(false)?;
                        }
                        KeyCode::Char('m') => {
                            // Modify goal/parameters
                            self.handle_goal_modification()?;
                            self.print_status_update(false)?;
                        }
                        KeyCode::Char('s') => {
                            // Force status update
                            self.print_status_update(false)?;
                        }
                        KeyCode::Char('h') | KeyCode::Char('?') => {
                            self.show_help()?;
                        }
                        _ => {}
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        if let Some(path) = &self.log_persist_path {
            let content = self.state.logs.join("\n");
            let _ = std::fs::write(path, content);
        }
        // Print final status
        println!("\n🤖 Agent execution completed or interrupted.");
        Ok(())
    }

    fn print_status_update(&self, is_initial: bool) -> Result<()> {
        // Clear screen for initial display (only if ANSI supported)
        if is_initial && self.use_ansi {
            print!("\x1B[2J\x1B[H");
        } else if is_initial {
            println!("\n=== FLUENT AGENTIC MODE ===");
        }

        // Color codes (or empty strings if ANSI disabled)
        let (reset, bold, cyan, green, yellow, red, blue, magenta) = if self.use_ansi {
            (
                "\x1B[0m", "\x1B[1m", "\x1B[36m", "\x1B[32m", "\x1B[33m", "\x1B[31m", "\x1B[34m",
                "\x1B[35m",
            )
        } else {
            ("", "", "", "", "", "", "", "")
        };

        if is_initial {
            println!(
                "{}┌────────────────────────────────────────────────────────────────┐{}",
                cyan, reset
            );
            println!(
                "{}│{}🤖 FLUENT AGENTIC MODE                                        {}│{}",
                bold, reset, cyan, reset
            );
            println!(
                "{}├────────────────────────────────────────────────────────────────┤{}",
                cyan, reset
            );
            println!(
                "{}│ Goal: {}{:<55}{}│{}",
                yellow, self.state.goal_description, reset, cyan, reset
            );
            println!(
                "{}│ Run:  {}{:<55}{}│{}",
                yellow, self.run_id, reset, cyan, reset
            );
            println!(
                "{}└────────────────────────────────────────────────────────────────┘{}",
                cyan, reset
            );
            println!();
        }

        let status_emoji = match &self.state.status {
            AgentStatus::Initializing => "🔄",
            AgentStatus::Running => "🚀",
            AgentStatus::Paused => "⏸️",
            AgentStatus::Completed => "✅",
            AgentStatus::Failed(_) => "❌",
            AgentStatus::Timeout => "⏰",
        };

        let status_color = match &self.state.status {
            AgentStatus::Initializing => yellow,
            AgentStatus::Running => green,
            AgentStatus::Paused => yellow,
            AgentStatus::Completed => green,
            AgentStatus::Failed(_) => red,
            AgentStatus::Timeout => red,
        };

        let elapsed = self.state.start_time.elapsed();
        let elapsed_str = format!(
            "{:02}:{:02}",
            elapsed.as_secs() / 60,
            elapsed.as_secs() % 60
        );

        // Status box
        println!(
            "{}┌─ STATUS ──────────────────────────────────────────────────────┐{}",
            blue, reset
        );
        println!(
            "{}│ {}{} {}{:<12} │ Iteration: {}{:>2}/{:<2}{} │ Elapsed: {}{:<5}{} │{}",
            status_color,
            status_emoji,
            self.status_text(),
            reset,
            blue,
            self.state.current_iteration,
            self.state.max_iterations,
            reset,
            green,
            elapsed_str,
            reset,
            blue,
            reset
        );
        println!(
            "{}│ Run: {}{}{:>54}{} │{}",
            blue, yellow, self.run_id, "", reset, blue
        );
        println!(
            "{}├─ PROGRESS ─────────────────────────────────────────────────────┤{}",
            blue, reset
        );

        // Progress bar with percentage
        let bar_width = 50;
        let filled = (self.state.progress_percentage as f32 / 100.0 * bar_width as f32) as usize;
        let bar =
            format!("{}{}{}", green, "█".repeat(filled), reset) + &"░".repeat(bar_width - filled);
        println!(
            "{}│ {}{:>3}%{} [{}] {}│{}",
            blue, green, self.state.progress_percentage, reset, bar, blue, reset
        );

        // Features
        let tools_status = if self.state.tools_enabled {
            format!("{}🔧 Tools{}", green, reset)
        } else {
            format!("{}⚪ No Tools{}", yellow, reset)
        };
        let reflection_status = if self.state.reflection_enabled {
            format!("{}🧠 Reflection{}", green, reset)
        } else {
            format!("{}⚪ No Reflection{}", yellow, reset)
        };
        println!(
            "{}│ Features: {} │ {} {}│{}",
            blue, tools_status, reflection_status, blue, reset
        );
        println!(
            "{}└────────────────────────────────────────────────────────────────┘{}",
            blue, reset
        );
        println!();

        // Current action
        if self.state.awaiting_approval {
            println!(
                "{}🎯 CURRENT ACTION:{} {} {}⏳ AWAITING APPROVAL{}",
                cyan, reset, self.state.current_action, yellow, reset
            );
        } else {
            println!(
                "{}🎯 CURRENT ACTION:{} {}",
                cyan, reset, self.state.current_action
            );
        }
        println!();

        // Recent logs
        if !self.state.logs.is_empty() {
            println!(
                "{}📝 RECENT ACTIVITY{} (last {} entries):",
                magenta,
                reset,
                self.state.logs.len().min(8)
            );
            println!(
                "{}┌────────────────────────────────────────────────────────────────┐{}",
                magenta, reset
            );

            let recent_logs = if is_initial {
                let start = if self.state.logs.len() > 8 {
                    self.state.logs.len() - 8
                } else {
                    0
                };
                &self.state.logs[start..]
            } else {
                // Show only the last 3 logs for updates
                let start = if self.state.logs.len() > 3 {
                    self.state.logs.len() - 3
                } else {
                    0
                };
                &self.state.logs[start..]
            };

            for (i, log) in recent_logs.iter().enumerate() {
                let line_num = if is_initial {
                    i + 1
                } else {
                    self.state.logs.len() - recent_logs.len() + i + 1
                };
                println!("{}│{:>2}: {}{}", magenta, line_num, log, reset);
            }

            if self.state.logs.len() > 8 && is_initial {
                println!(
                    "{}│ ... ({} more entries, use ↑/↓ in full TUI){}",
                    magenta,
                    self.state.logs.len() - 8,
                    reset
                );
            }

            println!(
                "{}└────────────────────────────────────────────────────────────────┘{}",
                magenta, reset
            );
        }

        // Controls
        println!();
        if self.state.awaiting_approval {
            println!(
                "{}🎮 CONTROLS:{} Q/Esc=Quit | A=Approve | R=Reject | I=Input | M=Modify | H/?=Help",
                green, reset
            );
            println!(
                "{}⚠️  ACTION AWAITING APPROVAL:{} Press 'A' to approve or 'R' to reject",
                red, reset
            );
        } else {
            println!(
                "{}🎮 CONTROLS:{} Q/Esc=Quit | P=Pause/Resume | I=Input | A=Approve | M=Modify | H/?=Help",
                green, reset
            );
            println!(
                "{}💡 TIP:{} Press 'I' to provide input or 'P' to pause execution",
                yellow, reset
            );
        }

        if !is_initial {
            println!(
                "{}─────────────────────────────────────────────────────────────────────{}",
                cyan, reset
            );
        }

        use std::io::Write;
        std::io::stdout().flush()?;

        Ok(())
    }

    fn show_help(&self) -> Result<()> {
        if self.use_ansi {
            print!("\x1B[2J\x1B[H");
        } else {
            println!("\n=== FLUENT AGENTIC MODE HELP ===");
        }

        let (reset, bold, cyan, green, yellow, blue, magenta) = if self.use_ansi {
            (
                "\x1B[0m", "\x1B[1m", "\x1B[36m", "\x1B[32m", "\x1B[33m", "\x1B[34m", "\x1B[35m",
            )
        } else {
            ("", "", "", "", "", "", "")
        };

        println!(
            "{}┌─ FLUENT AGENTIC MODE HELP ──────────────────────────────────────┐{}",
            cyan, reset
        );
        println!(
            "{}│{}🤖 ASCII Interface with Human-in-the-Loop Capabilities         {}│{}",
            bold, reset, cyan, reset
        );
        println!(
            "{}├──────────────────────────────────────────────────────────────────┤{}",
            cyan, reset
        );
        println!("{}│ This interface provides real-time monitoring and control of agent execution with human intervention capabilities. {}│", blue, reset);
        println!(
            "{}├─ CONTROLS ───────────────────────────────────────────────────────┤{}",
            cyan, reset
        );
        println!(
            "{}│ Q{} or {}Esc{}    - Quit and return to terminal                   {}│{}",
            green, reset, green, reset, blue, reset
        );
        println!(
            "{}│ P{}          - Pause/Resume agent execution                     {}│{}",
            green, reset, blue, reset
        );
        println!(
            "{}│ I{}          - Provide human input/advice to agent              {}│{}",
            green, reset, blue, reset
        );
        println!(
            "{}│ A{}          - Approve current agent action                     {}│{}",
            green, reset, blue, reset
        );
        println!(
            "{}│ R{}          - Reject current agent action                      {}│{}",
            green, reset, blue, reset
        );
        println!(
            "{}│ M{}          - Modify agent goal or parameters                  {}│{}",
            green, reset, blue, reset
        );
        println!(
            "{}│ H{} or {}?{}     - Show this help screen                          {}│{}",
            green, reset, green, reset, blue, reset
        );
        println!(
            "{}├─ DISPLAY INFORMATION ─────────────────────────────────────────────┤{}",
            cyan, reset
        );
        println!(
            "{}│ • {}Status{}: Current execution state with color coding            {}│{}",
            blue, yellow, reset, blue, reset
        );
        println!(
            "{}│ • {}Progress{}: Visual progress bar with percentage                {}│{}",
            blue, green, reset, blue, reset
        );
        println!(
            "{}│ • {}Features{}: Tool and reflection capability indicators          {}│{}",
            blue, magenta, reset, blue, reset
        );
        println!(
            "{}│ • {}Action{}: Current agent activity description                   {}│{}",
            blue, cyan, reset, blue, reset
        );
        println!(
            "{}│ • {}Activity{}: Recent execution logs and decisions                {}│{}",
            blue, yellow, reset, blue, reset
        );
        println!(
            "{}├─ HUMAN-IN-THE-LOOP FEATURES ──────────────────────────────────────┤{}",
            cyan, reset
        );
        println!(
            "{}│ • {}Pause/Resume{}: Stop agent execution for review                {}│{}",
            blue, green, reset, blue, reset
        );
        println!(
            "{}│ • {}Human Input{}: Provide guidance or additional context          {}│{}",
            blue, yellow, reset, blue, reset
        );
        println!(
            "{}│ • {}Action Approval{}: Review and approve/reject decisions         {}│{}",
            blue, magenta, reset, blue, reset
        );
        println!(
            "{}│ • {}Goal Modification{}: Change objectives mid-execution           {}│{}",
            blue, cyan, reset, blue, reset
        );
        println!(
            "{}├─ TIPS ────────────────────────────────────────────────────────────┤{}",
            cyan, reset
        );
        println!(
            "{}│ • Interface updates automatically every second                     {}│",
            blue, reset
        );
        println!(
            "{}│ • Use P to pause for complex decisions                             {}│",
            blue, reset
        );
        println!(
            "{}│ • Press I when agent seems stuck or needs guidance                {}│",
            blue, reset
        );
        println!(
            "{}│ • A/R for safety-critical actions                                 {}│",
            blue, reset
        );
        println!(
            "{}│ • Compatible with all terminals and environments                  {}│",
            blue, reset
        );
        println!(
            "{}└──────────────────────────────────────────────────────────────────┘{}",
            cyan, reset
        );
        println!();
        println!(
            "{}Press any key to return to the main interface...{}",
            yellow, reset
        );

        use std::io::Write;
        std::io::stdout().flush()?;

        // Wait for any key
        if crossterm::event::poll(std::time::Duration::from_secs(10))? {
            let _ = event::read();
        }

        Ok(())
    }

    fn handle_human_input(&mut self) -> Result<()> {
        if self.use_ansi {
            print!("\x1B[2J\x1B[H");
        } else {
            println!("\n=== HUMAN INPUT ===");
        }

        let (cyan, green, yellow, reset) = if self.use_ansi {
            ("\x1B[36m", "\x1B[32m", "\x1B[33m", "\x1B[0m")
        } else {
            ("", "", "", "")
        };

        println!(
            "{}┌─ HUMAN INPUT ───────────────────────────────────────────────────┐{}",
            cyan, reset
        );
        println!(
            "{}│{}🤖 Provide guidance or additional context to the agent         {}│{}",
            green, reset, cyan, reset
        );
        println!(
            "{}├──────────────────────────────────────────────────────────────────┤{}",
            cyan, reset
        );
        println!(
            "{}│ Current Goal: {}{:<48}{}│{}",
            yellow, self.state.goal_description, reset, cyan, reset
        );
        println!(
            "{}│ Current Action: {}{:<45}{}│{}",
            yellow, self.state.current_action, reset, cyan, reset
        );
        println!(
            "{}├──────────────────────────────────────────────────────────────────┤{}",
            cyan, reset
        );
        println!(
            "{}│ Enter your input (press Enter when done, Esc to cancel):         {}│",
            cyan, reset
        );
        println!(
            "{}└──────────────────────────────────────────────────────────────────┘{}",
            cyan, reset
        );
        println!();

        // For now, simulate human input since we don't have interactive input in this context
        let sample_input = "Please be more careful with file operations and ask for confirmation before making changes.";
        println!(
            "{}💬 Simulated human input: {}{}",
            green, sample_input, reset
        );
        println!();
        println!("{}Press any key to continue...{}", yellow, reset);

        use std::io::Write;
        std::io::stdout().flush()?;

        // Wait for any key
        if crossterm::event::poll(std::time::Duration::from_secs(5))? {
            let _ = event::read();
        }

        // Record the human intervention
        self.state
            .human_interventions
            .push(HumanIntervention::Input(sample_input.to_string()));
        self.state.last_human_input = Some(sample_input.to_string());
        self.add_log(format!("💬 Human input: {}", sample_input));

        Ok(())
    }

    fn handle_goal_modification(&mut self) -> Result<()> {
        if self.use_ansi {
            print!("\x1B[2J\x1B[H");
        } else {
            println!("\n=== GOAL MODIFICATION ===");
        }

        let (cyan, green, yellow, red, reset) = if self.use_ansi {
            ("\x1B[36m", "\x1B[32m", "\x1B[33m", "\x1B[31m", "\x1B[0m")
        } else {
            ("", "", "", "", "")
        };

        println!(
            "{}┌─ GOAL MODIFICATION ─────────────────────────────────────────────┐{}",
            cyan, reset
        );
        println!(
            "{}│{}🎯 Modify agent goal or execution parameters                   {}│{}",
            green, reset, cyan, reset
        );
        println!(
            "{}├──────────────────────────────────────────────────────────────────┤{}",
            cyan, reset
        );
        println!(
            "{}│ Current Goal:                                                   {}│",
            cyan, reset
        );
        println!(
            "{}│ {}{:<62}{}│{}",
            yellow, self.state.goal_description, reset, cyan, reset
        );
        println!(
            "{}├──────────────────────────────────────────────────────────────────┤{}",
            cyan, reset
        );
        println!(
            "{}│ Options:                                                        {}│",
            cyan, reset
        );
        println!(
            "{}│ 1. Modify goal description{}                                   {}│{}",
            green, reset, cyan, reset
        );
        println!(
            "{}│ 2. Change max iterations{}                                     {}│{}",
            green, reset, cyan, reset
        );
        println!(
            "{}│ 3. Toggle tool usage{}                                         {}│{}",
            green, reset, cyan, reset
        );
        println!(
            "{}│ 4. Toggle reflection{}                                         {}│{}",
            green, reset, cyan, reset
        );
        println!(
            "{}│ 0. Cancel{}                                                    {}│{}",
            red, reset, cyan, reset
        );
        println!(
            "{}├──────────────────────────────────────────────────────────────────┤{}",
            cyan, reset
        );
        println!(
            "{}│ Enter choice (0-4):                                             {}│",
            cyan, reset
        );
        println!(
            "{}└──────────────────────────────────────────────────────────────────┘{}",
            cyan, reset
        );
        println!();
        println!(
            "{}💡 Goal modification will affect ongoing execution{}",
            yellow, reset
        );
        println!("{}Press any key to continue...{}", green, reset);

        use std::io::Write;
        std::io::stdout().flush()?;

        // Wait for any key
        if crossterm::event::poll(std::time::Duration::from_secs(5))? {
            let _ = event::read();
        }

        // Simulate goal modification
        let new_goal = format!("{} (modified by user)", self.state.goal_description);
        self.state.goal_description = new_goal.clone();
        self.state
            .human_interventions
            .push(HumanIntervention::GoalModification(new_goal.clone()));
        self.add_log(format!("🎯 Goal modified to: {}", new_goal));

        Ok(())
    }

    fn status_text(&self) -> &str {
        match &self.state.status {
            AgentStatus::Initializing => "Initializing",
            AgentStatus::Running => "Running",
            AgentStatus::Paused => "Paused",
            AgentStatus::Completed => "Completed",
            AgentStatus::Failed(_) => "Failed",
            AgentStatus::Timeout => "Timeout",
        }
    }
}

/// TUI Manager for coordinating TUI updates from agent execution
pub struct TuiManager {
    full_tui: Option<AgentTui>,
    simple_tui: Option<SimpleTui>,
    ascii_tui: Option<AsciiTui>,
    collaborative_tui: Option<CollaborativeTui>,
    control_channel: Option<std::sync::Arc<fluent_agent::AgentControlChannel>>,
    enabled: bool,
    fallback_mode: bool,
    use_simple: bool,
    simple_handle: Option<tokio::task::JoinHandle<()>>,
    collab_handle: Option<tokio::task::JoinHandle<()>>,
}

impl TuiManager {
    pub fn new(enabled: bool) -> Self {
        // Use simple TUI by default (it actually works)
        let use_simple = std::env::var("FLUENT_USE_OLD_TUI").is_err();

        Self {
            full_tui: None,
            simple_tui: None,
            ascii_tui: None,
            collaborative_tui: None,
            control_channel: None,
            enabled,
            fallback_mode: false,
            use_simple,
            simple_handle: None,
            collab_handle: None,
        }
    }

    pub fn init(&mut self) -> Result<()> {
        if self.enabled {
            if std::env::var("FLUENT_FORCE_ASCII")
                .ok()
                .map(|v| v == "1")
                .unwrap_or(false)
            {
                let ascii_tui = AsciiTui::new();
                let ansi_status = if ascii_tui.use_ansi {
                    "with colors"
                } else {
                    "plain text"
                };
                self.ascii_tui = Some(ascii_tui);
                self.fallback_mode = true;
                println!(
                    "✅ ASCII interface initialized ({}) - Q=quit, S=status, H=help",
                    ansi_status
                );
                return Ok(());
            }
            let channel = std::sync::Arc::new(fluent_agent::AgentControlChannel::new());
            self.control_channel = Some(channel.clone());

            // Prefer collaborative TUI when explicitly requested
            if std::env::var("FLUENT_USE_COLLAB_TUI")
                .ok()
                .map(|v| v == "1")
                .unwrap_or(false)
            {
                match CollaborativeTui::new(Some(channel.clone())) {
                    Ok(tui) => {
                        self.collaborative_tui = Some(tui);
                        self.fallback_mode = false;
                        println!("✅ Collaborative TUI initialized - interactive chat available");
                        self.collab_handle = self.spawn_collab_tui();
                        return Ok(());
                    }
                    Err(e) => {
                        eprintln!("CollaborativeTui failed: {}, falling back", e);
                    }
                }
            }
            // Try SimpleTUI first (it actually works!)
            if self.use_simple {
                match SimpleTui::new(Some(channel.clone())) {
                    Ok(tui) => {
                        self.simple_tui = Some(tui);
                        self.fallback_mode = false;
                        println!("✅ Full TUI initialized - interactive controls available");
                        // Start SimpleTUI rendering in background
                        self.simple_handle = self.spawn_simple_tui();
                        return Ok(());
                    }
                    Err(e) => {
                        eprintln!("SimpleTUI failed: {}, falling back", e);
                        // Fall through to try old TUI
                    }
                }
            }

            // Try collaborative TUI if enabled by env
            if std::env::var("FLUENT_USE_COLLAB_TUI")
                .ok()
                .map(|v| v == "1")
                .unwrap_or(false)
            {
                match CollaborativeTui::new(Some(channel.clone())) {
                    Ok(tui) => {
                        self.collaborative_tui = Some(tui);
                        self.fallback_mode = false;
                        println!("✅ Collaborative TUI initialized - interactive chat available");
                        return Ok(());
                    }
                    Err(e) => {
                        eprintln!("CollaborativeTui failed: {}, falling back", e);
                    }
                }
            }

            // Try full TUI (old version)
            match AgentTui::new(Some(channel.clone())) {
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
            let ascii_tui = AsciiTui::new();
            let ansi_status = if ascii_tui.use_ansi {
                "with colors"
            } else {
                "plain text"
            };
            self.ascii_tui = Some(ascii_tui);
            self.fallback_mode = true;
            println!(
                "✅ ASCII interface initialized ({}) - Q=quit, S=status, H=help",
                ansi_status
            );
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
            let _ = channel
                .state_tx
                .try_send(fluent_agent::agent_control::StateUpdate::log(
                    fluent_agent::agent_control::LogLevel::Info,
                    message.clone(),
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

    /// Spawn SimpleTUI in a separate task and return the task handle
    pub fn spawn_simple_tui(&mut self) -> Option<tokio::task::JoinHandle<()>> {
        self.simple_tui.take().map(|mut tui| {
            tokio::spawn(async move {
                if let Err(e) = tui.run().await {
                    eprintln!("TUI error: {}", e);
                }
            })
        })
    }

    pub fn spawn_collab_tui(&mut self) -> Option<tokio::task::JoinHandle<()>> {
        self.collaborative_tui.take().map(|mut tui| {
            tokio::spawn(async move {
                if let Err(e) = tui.run().await {
                    eprintln!("Collaborative TUI error: {}", e);
                }
            })
        })
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
                AgentStatus::Failed(msg) => {
                    fluent_agent::agent_control::AgentStatus::Failed(msg.clone())
                }
                AgentStatus::Timeout => fluent_agent::agent_control::AgentStatus::Timeout,
            };
            let _ =
                channel
                    .state_tx
                    .try_send(fluent_agent::agent_control::StateUpdate::status_change(
                        agent_status,
                    ));
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
            let _ = channel.state_tx.try_send(
                fluent_agent::agent_control::StateUpdate::iteration_update(current, max, progress),
            );
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

    pub async fn run_event_loop(&mut self) -> Result<()> {
        if let Some(tui) = &mut self.full_tui {
            tui.run().await?;
        } else if let Some(handle) = &mut self.collab_handle {
            let _ = handle.await;
        } else if let Some(ascii) = &mut self.ascii_tui {
            // Display current state immediately
            ascii.print_status_update(true)?;
            ascii.run().await?;
        } else if let Some(handle) = &mut self.simple_handle {
            // SimpleTUI is running; wait until it exits
            let _ = handle.await;
        }
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

    pub fn control_receiver(&self) -> Option<fluent_agent::agent_control::ControlRxHandle> {
        self.control_channel.as_ref().map(|c| c.control_receiver())
    }

    /// Force display of current state (for ASCII TUI)
    pub fn force_display(&mut self) -> Result<()> {
        if let Some(ascii) = &mut self.ascii_tui {
            ascii.print_status_update(true)?;
        }
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
