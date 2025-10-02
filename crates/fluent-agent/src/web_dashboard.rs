//! Web Dashboard for Agent Monitoring, Control, and Visualization
//!
//! This module provides a comprehensive web-based dashboard for monitoring,
//! controlling, and visualizing the mega super agent's activities, performance,
//! and decision-making processes.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{broadcast, RwLock};
use warp::Filter;

use crate::context::ExecutionContext;
use crate::goal::Goal;
use crate::human_collaboration::{CollaborationSession, HumanCollaborationCoordinator};
use crate::swarm_intelligence::SwarmCoordinator;

/// Web dashboard server
pub struct WebDashboard {
    /// Server configuration
    config: DashboardConfig,
    /// Real-time data streams
    data_streams: Arc<RwLock<DataStreams>>,
    /// Dashboard state
    state: Arc<RwLock<DashboardState>>,
    /// WebSocket connections
    websocket_connections: Arc<RwLock<HashMap<String, broadcast::Sender<DashboardEvent>>>>,
    /// Collaboration coordinator reference
    collaboration_coordinator: Option<Arc<HumanCollaborationCoordinator>>,
    /// Swarm coordinator reference
    swarm_coordinator: Option<Arc<SwarmCoordinator>>,
}

/// Dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    /// Server host
    pub host: String,
    /// Server port
    pub port: u16,
    /// Enable authentication
    pub enable_auth: bool,
    /// API keys for authentication
    pub api_keys: Vec<String>,
    /// Enable SSL
    pub enable_ssl: bool,
    /// SSL certificate path
    pub ssl_cert_path: Option<String>,
    /// SSL key path
    pub ssl_key_path: Option<String>,
    /// Update interval for real-time data
    pub update_interval_ms: u64,
}

/// Real-time data streams
pub struct DataStreams {
    /// Agent performance metrics
    performance_metrics: Vec<PerformanceMetric>,
    /// Active goals and progress
    active_goals: Vec<GoalProgress>,
    /// System health indicators
    system_health: SystemHealth,
    /// Collaboration sessions
    collaboration_sessions: Vec<CollaborationSession>,
    /// Swarm intelligence metrics
    swarm_metrics: Option<SwarmMetrics>,
    /// Ethical guardrails status
    ethical_status: EthicalStatus,
}

/// Dashboard state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardState {
    /// Dashboard version
    pub version: String,
    /// Server start time
    pub start_time: SystemTime,
    /// Total requests served
    pub total_requests: u64,
    /// Active connections
    pub active_connections: u32,
    /// System status
    pub system_status: SystemStatus,
}

/// Performance metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetric {
    pub timestamp: SystemTime,
    pub metric_type: String,
    pub value: f64,
    pub unit: String,
    pub agent_id: Option<String>,
}

/// Goal progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalProgress {
    pub goal_id: String,
    pub description: String,
    pub progress_percentage: f64,
    pub status: GoalStatus,
    pub start_time: SystemTime,
    pub estimated_completion: Option<SystemTime>,
    pub assigned_agents: Vec<String>,
}

/// Goal status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Paused,
}

/// System health indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub overall_status: HealthStatus,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_usage: f64,
    pub network_status: NetworkStatus,
    pub last_health_check: SystemTime,
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

/// Network status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkStatus {
    Connected,
    Degraded,
    Disconnected,
}

/// Swarm intelligence metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmMetrics {
    pub total_agents: u32,
    pub active_agents: u32,
    pub completed_tasks: u32,
    pub average_task_completion_time: Duration,
    pub collaboration_rate: f64,
    pub consensus_reach_rate: f64,
}

/// Ethical status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthicalStatus {
    pub overall_compliance: f64,
    pub active_violations: u32,
    pub resolved_violations: u32,
    pub bias_detected: bool,
    pub last_ethical_check: SystemTime,
}

/// System status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemStatus {
    Starting,
    Running,
    Maintenance,
    Stopping,
    Error,
}

/// Real-time dashboard events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DashboardEvent {
    /// Performance metric update
    PerformanceUpdate { metric: PerformanceMetric },
    /// Goal progress update
    GoalProgressUpdate { progress: GoalProgress },
    /// System health update
    HealthUpdate { health: SystemHealth },
    /// New collaboration session
    CollaborationStarted {
        session_id: String,
        participant_count: usize,
    },
    /// Agent message
    AgentMessage {
        agent_id: String,
        message: String,
        message_type: String,
    },
    /// Ethical alert
    EthicalAlert {
        alert_type: String,
        description: String,
        severity: String,
    },
    /// Swarm event
    SwarmEvent {
        event_type: String,
        description: String,
    },
    /// User action
    UserAction {
        user: String,
        action: String,
        target: String,
    },
}

/// Dashboard API endpoints
#[derive(Clone)]
pub struct DashboardAPI {
    dashboard: Arc<WebDashboard>,
}

impl WebDashboard {
    /// Create a new web dashboard
    pub fn new(config: DashboardConfig) -> Self {
        Self {
            config,
            data_streams: Arc::new(RwLock::new(DataStreams::new())),
            state: Arc::new(RwLock::new(DashboardState::new())),
            websocket_connections: Arc::new(RwLock::new(HashMap::new())),
            collaboration_coordinator: None,
            swarm_coordinator: None,
        }
    }

    /// Set collaboration coordinator reference
    pub fn with_collaboration_coordinator(
        mut self,
        coordinator: Arc<HumanCollaborationCoordinator>,
    ) -> Self {
        self.collaboration_coordinator = Some(coordinator);
        self
    }

    /// Set swarm coordinator reference
    pub fn with_swarm_coordinator(mut self, coordinator: Arc<SwarmCoordinator>) -> Self {
        self.swarm_coordinator = Some(coordinator);
        self
    }

    /// Start the dashboard server
    pub async fn start(self: Arc<Self>) -> Result<()> {
        let api = DashboardAPI {
            dashboard: self.clone(),
        };

        // Update dashboard state
        {
            let mut state = self.state.write().await;
            state.system_status = SystemStatus::Running;
        }

        // Create routes
        let routes = self.create_routes(api);

        // Start server
        let addr = format!("{}:{}", self.config.host, self.config.port);
        println!("🚀 Starting web dashboard on http://{}", addr);

        warp::serve(routes)
            .run(([0, 0, 0, 0], self.config.port))
            .await;

        Ok(())
    }

    /// Create warp routes
    fn create_routes(
        &self,
        api: DashboardAPI,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let dashboard_api = api.clone();
        let dashboard = warp::path("dashboard").and(warp::get()).and_then(move || {
            let api = dashboard_api.clone();
            async move { api.serve_dashboard().await }
        });

        let metrics_api = api.clone();
        let goals_api = api.clone();
        let health_api = api.clone();
        let sessions_api = api.clone();
        let ws_api = api.clone();

        let api_routes = warp::path("api").and(
            warp::path("metrics")
                .and(warp::get())
                .and_then(move || {
                    let api = metrics_api.clone();
                    async move { api.get_metrics().await }
                })
                .or(warp::path("goals").and(warp::get()).and_then(move || {
                    let api = goals_api.clone();
                    async move { api.get_goals().await }
                }))
                .or(warp::path("health").and(warp::get()).and_then(move || {
                    let api = health_api.clone();
                    async move { api.get_health().await }
                }))
                .or(warp::path("sessions").and(warp::get()).and_then(move || {
                    let api = sessions_api.clone();
                    async move { api.get_sessions().await }
                })),
        );

        let websocket = warp::path("ws")
            .and(warp::ws())
            .map(move |ws: warp::ws::Ws| {
                let api = ws_api.clone();
                ws.on_upgrade(move |websocket| async move {
                    if let Err(e) = api.handle_websocket(websocket).await {
                        eprintln!("WebSocket error: {}", e);
                    }
                })
            });

        dashboard.or(api_routes).or(websocket)
    }

    /// Update performance metrics
    pub async fn update_performance_metric(&self, metric: PerformanceMetric) -> Result<()> {
        let mut streams = self.data_streams.write().await;
        streams.performance_metrics.push(metric.clone());

        // Keep only recent metrics (last 1000)
        if streams.performance_metrics.len() > 1000 {
            streams.performance_metrics.remove(0);
        }

        // Broadcast update
        self.broadcast_event(DashboardEvent::PerformanceUpdate { metric })
            .await;

        Ok(())
    }

    /// Update goal progress
    pub async fn update_goal_progress(&self, progress: GoalProgress) -> Result<()> {
        let mut streams = self.data_streams.write().await;

        // Update or add goal progress
        if let Some(existing) = streams
            .active_goals
            .iter_mut()
            .find(|g| g.goal_id == progress.goal_id)
        {
            *existing = progress.clone();
        } else {
            streams.active_goals.push(progress.clone());
        }

        // Broadcast update
        self.broadcast_event(DashboardEvent::GoalProgressUpdate { progress })
            .await;

        Ok(())
    }

    /// Update system health
    pub async fn update_system_health(&self, health: SystemHealth) -> Result<()> {
        let mut streams = self.data_streams.write().await;
        streams.system_health = health.clone();

        // Broadcast update
        self.broadcast_event(DashboardEvent::HealthUpdate { health })
            .await;

        Ok(())
    }

    /// Broadcast event to all connected clients
    async fn broadcast_event(&self, event: DashboardEvent) {
        let connections = self.websocket_connections.read().await;

        for sender in connections.values() {
            let _ = sender.send(event.clone());
        }
    }
}

impl DashboardAPI {
    /// Serve the main dashboard HTML
    pub async fn serve_dashboard(&self) -> Result<impl warp::Reply, warp::Rejection> {
        let html = self.generate_dashboard_html().await;
        Ok(warp::reply::html(html))
    }

    /// Get performance metrics
    pub async fn get_metrics(&self) -> Result<impl warp::Reply, warp::Rejection> {
        let streams = self.dashboard.data_streams.read().await;
        Ok(warp::reply::json(&streams.performance_metrics))
    }

    /// Get active goals
    pub async fn get_goals(&self) -> Result<impl warp::Reply, warp::Rejection> {
        let streams = self.dashboard.data_streams.read().await;
        Ok(warp::reply::json(&streams.active_goals))
    }

    /// Get system health
    pub async fn get_health(&self) -> Result<impl warp::Reply, warp::Rejection> {
        let streams = self.dashboard.data_streams.read().await;
        Ok(warp::reply::json(&streams.system_health))
    }

    /// Get collaboration sessions
    pub async fn get_sessions(&self) -> Result<impl warp::Reply, warp::Rejection> {
        let streams = self.dashboard.data_streams.read().await;
        // Return a simplified version without complex nested types
        let session_summaries: Vec<serde_json::Value> = streams.collaboration_sessions.iter().map(|session| {
            serde_json::json!({
                "id": session.id.to_string(),
                "human_participants": session.human_participants,
                "agent_count": session.agent_participants.len(),
                "status": format!("{:?}", session.status),
                "started_at": session.started_at.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
                "message_count": session.message_history.len()
            })
        }).collect();
        Ok(warp::reply::json(&session_summaries))
    }

    /// Handle WebSocket connections
    pub async fn handle_websocket(&self, websocket: warp::ws::WebSocket) -> Result<()> {
        use futures_util::{SinkExt, StreamExt};

        let (mut ws_tx, mut ws_rx) = websocket.split();

        // Generate connection ID
        let connection_id = format!(
            "ws_{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_nanos()
        );

        // Create broadcast channel for this connection
        let (tx, mut rx) = broadcast::channel(100);

        // Store connection
        {
            let mut connections = self.dashboard.websocket_connections.write().await;
            connections.insert(connection_id.clone(), tx);
        }

        // Update active connections count
        {
            let mut state = self.dashboard.state.write().await;
            state.active_connections += 1;
        }

        // Send initial dashboard state
        let state = self.dashboard.state.read().await;
        let state_json = serde_json::to_string(&*state)?;
        ws_tx.send(warp::ws::Message::text(state_json)).await?;

        // Handle incoming messages
        let handle_rx = async {
            while let Some(result) = ws_rx.next().await {
                match result {
                    Ok(msg) => {
                        if msg.is_text() {
                            // Handle incoming messages from client
                            let _text = msg.to_str().unwrap_or("");
                            // Process client commands here
                        } else if msg.is_close() {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("WebSocket error: {}", e);
                        break;
                    }
                }
            }
        };

        // Handle outgoing messages
        let handle_tx = async {
            while let Ok(event) = rx.recv().await {
                let event_json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
                if ws_tx
                    .send(warp::ws::Message::text(event_json))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        };

        // Run both handlers concurrently
        tokio::select! {
            _ = handle_rx => {}
            _ = handle_tx => {}
        }

        // Clean up connection
        {
            let mut connections = self.dashboard.websocket_connections.write().await;
            connections.remove(&connection_id);
        }

        {
            let mut state = self.dashboard.state.write().await;
            state.active_connections = state.active_connections.saturating_sub(1);
        }

        Ok(())
    }

    /// Generate dashboard HTML
    async fn generate_dashboard_html(&self) -> String {
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Mega Super Agent Dashboard</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            margin: 0;
            padding: 20px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
        }}
        .container {{
            max-width: 1200px;
            margin: 0 auto;
            background: rgba(255, 255, 255, 0.1);
            border-radius: 10px;
            padding: 20px;
            backdrop-filter: blur(10px);
        }}
        .header {{
            text-align: center;
            margin-bottom: 30px;
        }}
        .metrics-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }}
        .metric-card {{
            background: rgba(255, 255, 255, 0.2);
            border-radius: 8px;
            padding: 20px;
            text-align: center;
        }}
        .metric-value {{
            font-size: 2em;
            font-weight: bold;
            margin: 10px 0;
        }}
        .chart-container {{
            background: rgba(255, 255, 255, 0.2);
            border-radius: 8px;
            padding: 20px;
            margin-bottom: 20px;
        }}
        .status-healthy {{ color: #4CAF50; }}
        .status-warning {{ color: #FF9800; }}
        .status-critical {{ color: #F44336; }}
        .goals-list {{
            background: rgba(255, 255, 255, 0.2);
            border-radius: 8px;
            padding: 20px;
        }}
        .goal-item {{
            margin-bottom: 10px;
            padding: 10px;
            background: rgba(255, 255, 255, 0.1);
            border-radius: 5px;
        }}
        .progress-bar {{
            width: 100%;
            height: 10px;
            background: rgba(255, 255, 255, 0.3);
            border-radius: 5px;
            overflow: hidden;
        }}
        .progress-fill {{
            height: 100%;
            background: linear-gradient(90deg, #4CAF50, #8BC34A);
            transition: width 0.3s ease;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🚀 Mega Super Agent Dashboard</h1>
            <p>Real-time monitoring and control interface</p>
        </div>

        <div class="metrics-grid">
            <div class="metric-card">
                <h3>System Health</h3>
                <div class="metric-value status-healthy" id="health-status">Healthy</div>
            </div>
            <div class="metric-card">
                <h3>Active Goals</h3>
                <div class="metric-value" id="active-goals">0</div>
            </div>
            <div class="metric-card">
                <h3>CPU Usage</h3>
                <div class="metric-value" id="cpu-usage">0%</div>
            </div>
            <div class="metric-card">
                <h3>Memory Usage</h3>
                <div class="metric-value" id="memory-usage">0%</div>
            </div>
        </div>

        <div class="chart-container">
            <h3>Performance Metrics</h3>
            <canvas id="performanceChart" width="400" height="200"></canvas>
        </div>

        <div class="goals-list">
            <h3>Active Goals</h3>
            <div id="goals-container">
                <p>Loading goals...</p>
            </div>
        </div>
    </div>

    <script>
        // WebSocket connection
        const ws = new WebSocket('ws://localhost:{}/ws');
        const performanceData = {{
            labels: [],
            datasets: [{{
                label: 'Performance',
                data: [],
                borderColor: 'rgb(75, 192, 192)',
                tension: 0.1
            }}]
        }};

        const performanceChart = new Chart(
            document.getElementById('performanceChart'),
            {{
                type: 'line',
                data: performanceData,
                options: {{
                    responsive: true,
                    scales: {{
                        y: {{
                            beginAtZero: true
                        }}
                    }}
                }}
            }}
        );

        ws.onmessage = function(event) {{
            const data = JSON.parse(event.data);

            if (data.PerformanceUpdate) {{
                const metric = data.PerformanceUpdate.metric;
                performanceData.labels.push(new Date(metric.timestamp).toLocaleTimeString());
                performanceData.datasets[0].data.push(metric.value);

                // Keep only last 50 points
                if (performanceData.labels.length > 50) {{
                    performanceData.labels.shift();
                    performanceData.datasets[0].data.shift();
                }}

                performanceChart.update();
            }} else if (data.GoalProgressUpdate) {{
                updateGoals(data.GoalProgressUpdate.progress);
            }} else if (data.HealthUpdate) {{
                updateHealth(data.HealthUpdate.health);
            }}
        }};

        ws.onopen = function() {{
            console.log('Connected to dashboard');
            loadInitialData();
        }};

        async function loadInitialData() {{
            try {{
                const [metrics, goals, health] = await Promise.all([
                    fetch('/api/metrics').then(r => r.json()),
                    fetch('/api/goals').then(r => r.json()),
                    fetch('/api/health').then(r => r.json())
                ]);

                // Update metrics
                if (metrics.length > 0) {{
                    metrics.slice(-50).forEach(metric => {{
                        performanceData.labels.push(new Date(metric.timestamp).toLocaleTimeString());
                        performanceData.datasets[0].data.push(metric.value);
                    }});
                    performanceChart.update();
                }}

                updateGoalsList(goals);
                updateHealth(health);
            }} catch (error) {{
                console.error('Error loading initial data:', error);
            }}
        }}

        function updateGoals(progress) {{
            const goalsContainer = document.getElementById('goals-container');
            const existingGoal = goalsContainer.querySelector(`[data-goal-id="${{progress.goal_id}}"]`);

            if (existingGoal) {{
                existingGoal.innerHTML = generateGoalHTML(progress);
            }} else {{
                const goalElement = document.createElement('div');
                goalElement.setAttribute('data-goal-id', progress.goal_id);
                goalElement.className = 'goal-item';
                goalElement.innerHTML = generateGoalHTML(progress);
                goalsContainer.appendChild(goalElement);
            }}

            document.getElementById('active-goals').textContent = goalsContainer.children.length;
        }}

        function updateGoalsList(goals) {{
            const goalsContainer = document.getElementById('goals-container');
            goalsContainer.innerHTML = '';

            goals.forEach(goal => {{
                const goalElement = document.createElement('div');
                goalElement.className = 'goal-item';
                goalElement.setAttribute('data-goal-id', goal.goal_id);
                goalElement.innerHTML = generateGoalHTML(goal);
                goalsContainer.appendChild(goalElement);
            }});

            document.getElementById('active-goals').textContent = goals.length;
        }}

        function generateGoalHTML(goal) {{
            return `
                <div>
                    <strong>${{goal.description}}</strong>
                    <div class="progress-bar">
                        <div class="progress-fill" style="width: ${{goal.progress_percentage}}%"></div>
                    </div>
                    <small>Status: ${{goal.status}} | Progress: ${{goal.progress_percentage.toFixed(1)}}%</small>
                </div>
            `;
        }}

        function updateHealth(health) {{
            document.getElementById('health-status').textContent = health.overall_status;
            document.getElementById('cpu-usage').textContent = health.cpu_usage.toFixed(1) + '%';
            document.getElementById('memory-usage').textContent = health.memory_usage.toFixed(1) + '%';

            const healthElement = document.getElementById('health-status');
            healthElement.className = 'metric-value';

            switch (health.overall_status) {{
                case 'Healthy':
                    healthElement.classList.add('status-healthy');
                    break;
                case 'Warning':
                    healthElement.classList.add('status-warning');
                    break;
                case 'Critical':
                    healthElement.classList.add('status-critical');
                    break;
            }}
        }}

        // Auto-refresh every 5 seconds
        setInterval(loadInitialData, 5000);
    </script>
</body>
</html>"#,
            self.dashboard.config.port
        )
    }
}

impl DataStreams {
    fn new() -> Self {
        Self {
            performance_metrics: Vec::new(),
            active_goals: Vec::new(),
            system_health: SystemHealth {
                overall_status: HealthStatus::Unknown,
                cpu_usage: 0.0,
                memory_usage: 0.0,
                disk_usage: 0.0,
                network_status: NetworkStatus::Disconnected,
                last_health_check: SystemTime::now(),
            },
            collaboration_sessions: Vec::new(),
            swarm_metrics: None,
            ethical_status: EthicalStatus {
                overall_compliance: 0.0,
                active_violations: 0,
                resolved_violations: 0,
                bias_detected: false,
                last_ethical_check: SystemTime::now(),
            },
        }
    }
}

impl DashboardState {
    fn new() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            start_time: SystemTime::now(),
            total_requests: 0,
            active_connections: 0,
            system_status: SystemStatus::Starting,
        }
    }
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            enable_auth: false,
            api_keys: Vec::new(),
            enable_ssl: false,
            ssl_cert_path: None,
            ssl_key_path: None,
            update_interval_ms: 1000,
        }
    }
}
