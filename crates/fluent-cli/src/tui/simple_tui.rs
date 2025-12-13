//! Simple Working TUI for Agent Monitoring
//!
//! A streamlined TUI that actually displays agent state in real-time

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Terminal,
};
use std::fs;
use std::path::PathBuf;
use std::{
    io::{self, IsTerminal},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

use fluent_agent::agent_control::{
    AgentControlChannel, AgentStatus, ControlMessage, StateUpdate, StateUpdateType,
};

/// Simple TUI state
#[derive(Clone)]
pub struct SimpleTuiState {
    pub status: String,
    pub status_color: Color,
    pub current_iteration: u32,
    pub max_iterations: u32,
    pub progress_percentage: u32,
    pub current_action: String,
    pub logs: Vec<String>,
    pub paused: bool,
    pub show_help: bool,
    pub filter: Option<String>,
    pub input_mode: bool,
    pub input_buffer: String,
}

impl Default for SimpleTuiState {
    fn default() -> Self {
        Self {
            status: "Initializing".to_string(),
            status_color: Color::Yellow,
            current_iteration: 0,
            max_iterations: 0,
            progress_percentage: 0,
            current_action: "Waiting...".to_string(),
            logs: Vec::new(),
            paused: false,
            show_help: false,
            filter: None,
            input_mode: false,
            input_buffer: String::new(),
        }
    }
}

pub struct SimpleTui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    state: Arc<RwLock<SimpleTuiState>>,
    control_channel: Option<Arc<AgentControlChannel>>,
    last_render: Instant,
    max_logs: usize,
    log_persist_path: Option<PathBuf>,
    last_frame_ms: u32,
    run_id: String,
}

impl SimpleTui {
    pub fn new(control_channel: Option<Arc<AgentControlChannel>>) -> Result<Self> {
        if !io::stdout().is_terminal() {
            return Err(anyhow::anyhow!("Not running in a terminal"));
        }

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let max_logs = std::env::var("FLUENT_TUI_MAX_LOGS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v >= 10)
            .unwrap_or(200);

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
        let mut path = PathBuf::from(base_dir);
        path.push("agent_logs");
        let _ = fs::create_dir_all(&path);
        path.push(format!("{}.log", run_id));
        let log_persist_path = Some(path);

        Ok(Self {
            terminal,
            state: Arc::new(RwLock::new(SimpleTuiState::default())),
            control_channel,
            last_render: Instant::now(),
            max_logs,
            log_persist_path,
            last_frame_ms: 0,
            run_id,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let frame_duration = Duration::from_millis(33); // ~30 FPS

        loop {
            // Poll for state updates
            self.poll_state_updates().await?;

            // Handle input
            if self.handle_input().await? {
                break; // Quit requested
            }

            // Render (rate limited)
            if self.last_render.elapsed() >= frame_duration {
                self.render()?;
                self.last_render = Instant::now();
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        self.cleanup()?;
        Ok(())
    }

    async fn poll_state_updates(&mut self) -> Result<()> {
        let Some(ref channel) = self.control_channel else {
            return Ok(());
        };

        while let Ok(Some(update)) = channel.state_receiver().try_recv().await {
            self.process_update(update).await;
        }

        Ok(())
    }

    async fn process_update(&self, update: StateUpdate) {
        let mut state = self.state.write().await;

        match update.update_type {
            StateUpdateType::StatusChange { status } => {
                let (status_text, color) = match status {
                    AgentStatus::Initializing => ("Initializing", Color::Yellow),
                    AgentStatus::Running => ("Running", Color::Green),
                    AgentStatus::Paused => {
                        state.paused = true;
                        ("Paused", Color::Yellow)
                    }
                    AgentStatus::Completed => ("Completed", Color::Green),
                    AgentStatus::Failed(ref msg) => {
                        state.logs.push(format!("ERROR: {}", msg));
                        ("Failed", Color::Red)
                    }
                    _ => ("Unknown", Color::Gray),
                };
                state.status = status_text.to_string();
                state.status_color = color;
            }

            StateUpdateType::IterationUpdate {
                current,
                max,
                progress_percentage,
            } => {
                state.current_iteration = current;
                state.max_iterations = max;
                state.progress_percentage = progress_percentage;
            }

            StateUpdateType::ActionUpdate {
                action_description, ..
            } => {
                state.current_action = action_description.clone();
                state.logs.push(format!("→ {}", action_description));

                let len = state.logs.len();
                if len > self.max_logs {
                    state.logs.drain(0..len - self.max_logs);
                }
            }

            StateUpdateType::LogMessage { level, message } => {
                let prefix = match level {
                    fluent_agent::agent_control::LogLevel::Error => "❌",
                    fluent_agent::agent_control::LogLevel::Warning => "⚠️ ",
                    fluent_agent::agent_control::LogLevel::Info => "ℹ️ ",
                    _ => "  ",
                };
                state.logs.push(format!("{} {}", prefix, message));

                let len = state.logs.len();
                if len > self.max_logs {
                    state.logs.drain(0..len - self.max_logs);
                }
            }

            StateUpdateType::ReasoningStep {
                step_description,
                confidence,
                ..
            } => {
                state.logs.push(format!(
                    "💭 {} (confidence: {:.0}%)",
                    step_description,
                    confidence * 100.0
                ));

                let len = state.logs.len();
                if len > self.max_logs {
                    state.logs.drain(0..len - self.max_logs);
                }
            }

            _ => {}
        }
    }

    async fn handle_input(&self) -> Result<bool> {
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        return Ok(true); // Quit
                    }

                    (KeyCode::Char('p'), _) => {
                        if let Some(ref channel) = self.control_channel {
                            let mut state = self.state.write().await;
                            let msg = if state.paused {
                                state.paused = false;
                                ControlMessage::resume()
                            } else {
                                state.paused = true;
                                ControlMessage::pause()
                            };
                            let _ = channel.send_control(msg).await;
                        }
                    }

                    (KeyCode::Char('h'), _) | (KeyCode::Char('?'), _) => {
                        let mut state = self.state.write().await;
                        state.show_help = !state.show_help;
                    }

                    (KeyCode::Char('/'), _) => {
                        let mut state = self.state.write().await;
                        state.input_mode = true;
                        state.input_buffer.clear();
                    }

                    (KeyCode::Char('n'), _) => {
                        let mut state = self.state.write().await;
                        state.filter = None;
                    }

                    (KeyCode::Backspace, _) => {
                        let mut state = self.state.write().await;
                        if state.input_mode {
                            state.input_buffer.pop();
                        }
                    }

                    (KeyCode::Enter, _) => {
                        let mut state = self.state.write().await;
                        if state.input_mode {
                            if !state.input_buffer.is_empty() {
                                state.filter = Some(state.input_buffer.clone());
                            }
                            state.input_mode = false;
                        }
                    }

                    (KeyCode::Esc, _) => {
                        let mut state = self.state.write().await;
                        if state.input_mode {
                            state.input_mode = false;
                            state.input_buffer.clear();
                        }
                    }

                    (KeyCode::Char(ch), _) => {
                        let mut state = self.state.write().await;
                        if state.input_mode {
                            state.input_buffer.push(ch);
                        }
                    }

                    _ => {}
                }
            }
        }

        Ok(false)
    }

    fn render(&mut self) -> Result<()> {
        let state = self.state.blocking_read().clone();

        let render_start = Instant::now();
        self.terminal.draw(|f| {
            let size = f.size();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Header
                    Constraint::Length(3), // Progress
                    Constraint::Min(10),   // Logs
                    Constraint::Length(3), // Controls
                ])
                .split(size);

            // Header
            let header_text = format!(
                "🤖 Fluent Agent (Run {}) - Status: {}",
                self.run_id, state.status
            );
            let header = Paragraph::new(header_text)
                .style(
                    Style::default()
                        .fg(state.status_color)
                        .add_modifier(Modifier::BOLD),
                )
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center);
            f.render_widget(header, chunks[0]);

            // Progress
            let progress_title = if state.max_iterations > 0 {
                format!(
                    "Progress: {}/{} iterations - {}",
                    state.current_iteration, state.max_iterations, state.current_action
                )
            } else {
                format!("Current Action: {}", state.current_action)
            };

            let progress = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title(progress_title))
                .gauge_style(Style::default().fg(Color::Green))
                .percent(state.progress_percentage.min(100) as u16);
            f.render_widget(progress, chunks[1]);

            // Logs or Help overlay
            if state.show_help {
                let help_lines = vec![
                    Line::from(Span::styled(
                        "Controls:",
                        Style::default().add_modifier(Modifier::BOLD),
                    )),
                    Line::from("  P = Pause / Resume"),
                    Line::from("  Q = Quit (or Ctrl-C)"),
                    Line::from("  H / ? = Toggle Help"),
                    Line::from("  / = Enter Filter • N = Clear Filter"),
                    Line::from(""),
                ];
                let help = Paragraph::new(help_lines)
                    .style(Style::default().fg(Color::Cyan))
                    .block(Block::default().borders(Borders::ALL).title("Help"))
                    .alignment(Alignment::Left);
                f.render_widget(help, chunks[2]);
            } else {
                let filtered: Vec<&String> = if let Some(ref q) = state.filter {
                    state.logs.iter().filter(|l| l.contains(q)).collect()
                } else {
                    state.logs.iter().collect()
                };

                let log_items: Vec<ListItem> = filtered
                    .iter()
                    .rev()
                    .take(chunks[2].height as usize - 2)
                    .rev()
                    .map(|log| ListItem::new((*log).clone()))
                    .collect();

                let logs_widget =
                    List::new(log_items).block(Block::default().borders(Borders::ALL).title(
                        match &state.filter {
                            Some(q) => format!(
                                "Activity Log ({} messages) • Filter: {}",
                                filtered.len(),
                                q
                            ),
                            None => format!("Activity Log ({} messages)", state.logs.len()),
                        },
                    ));
                f.render_widget(logs_widget, chunks[2]);
            }

            // Controls
            let mut control_text = if state.paused {
                "P=Resume | H=Help | Q=Quit".to_string()
            } else {
                "P=Pause | H=Help | Q=Quit".to_string()
            };
            if state.input_mode {
                control_text = format!("Filter: {}_ (Enter=Apply Esc=Cancel)", state.input_buffer);
            } else {
                control_text = format!("{} • Frame {}ms", control_text, self.last_frame_ms);
            }

            let controls = Paragraph::new(control_text)
                .style(Style::default().fg(Color::Cyan))
                .block(Block::default().borders(Borders::ALL).title("Controls"))
                .alignment(Alignment::Center);
            f.render_widget(controls, chunks[3]);
        })?;

        let elapsed = render_start.elapsed();
        self.last_frame_ms = elapsed.as_millis() as u32;

        Ok(())
    }

    fn cleanup(&mut self) -> Result<()> {
        if let Some(path) = &self.log_persist_path {
            if let Ok(state_guard) = self.state.try_read() {
                let state = state_guard.clone();
                let parent_dir = path
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| PathBuf::from("."));
                let _ = fs::create_dir_all(parent_dir);
                let content = state.logs.join("\n");
                let _ = fs::write(path, content);
            }
        }
        disable_raw_mode()?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for SimpleTui {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
