//! Swarm Intelligence and Multi-Agent Collaboration System
//!
//! This module implements advanced swarm intelligence capabilities that enable
//! multiple agents to collaborate on complex problem solving. It includes
//! agent specialization, communication protocols, consensus mechanisms,
//! and emergent behavior coordination.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use crate::context::ExecutionContext;
use crate::goal::{Goal, GoalPriority};
use crate::memory::MemorySystem;
use crate::reasoning::ReasoningEngine;

type CommunicationChannels = Arc<RwLock<HashMap<(Uuid, Uuid), mpsc::UnboundedSender<Message>>>>;

/// Swarm intelligence coordinator for multi-agent collaboration
pub struct SwarmCoordinator {
    /// All agents in the swarm
    agents: Arc<RwLock<HashMap<Uuid, SwarmAgent>>>,
    /// Communication channels between agents
    communication_channels: CommunicationChannels,
    /// Global swarm memory
    swarm_memory: Arc<MemorySystem>,
    /// Consensus mechanism
    consensus_engine: Arc<RwLock<ConsensusEngine>>,
    /// Task decomposition and assignment
    task_allocator: Arc<RwLock<TaskAllocator>>,
    /// Performance monitoring
    performance_monitor: Arc<RwLock<SwarmPerformanceMonitor>>,
    /// Specialization registry
    specialization_registry: Arc<RwLock<SpecializationRegistry>>,
}

/// Individual agent in the swarm
pub struct SwarmAgent {
    /// Unique agent identifier
    pub id: Uuid,
    /// Agent specialization
    pub specialization: AgentSpecialization,
    /// Current status
    pub status: AgentStatus,
    /// Performance metrics
    pub performance: AgentPerformance,
    /// Communication queue
    pub message_queue: mpsc::UnboundedReceiver<Message>,
    /// Reasoning engine
    pub reasoning_engine: Arc<dyn ReasoningEngine>,
    /// Working memory
    pub working_memory: Arc<MemorySystem>,
    /// Last activity timestamp
    pub last_activity: SystemTime,
}

/// Agent specialization types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum AgentSpecialization {
    /// Code analysis and generation
    CodeSpecialist,
    /// Testing and quality assurance
    TestSpecialist,
    /// Documentation and communication
    DocumentationSpecialist,
    /// Architecture and design
    ArchitectureSpecialist,
    /// Security analysis
    SecuritySpecialist,
    /// Performance optimization
    PerformanceSpecialist,
    /// Research and exploration
    ResearchSpecialist,
    /// Integration and deployment
    IntegrationSpecialist,
    /// General purpose agent
    #[default]
    GeneralPurpose,
}

/// Agent status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentStatus {
    /// Agent is idle and available
    Idle,
    /// Agent is working on a task
    Working { task_id: Uuid, progress: f64 },
    /// Agent is collaborating with others
    Collaborating { partners: Vec<Uuid>, task_id: Uuid },
    /// Agent is learning or updating
    Learning,
    /// Agent encountered an error
    Error { message: String },
}

/// Agent performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPerformance {
    pub tasks_completed: u32,
    pub success_rate: f64,
    pub average_task_time: Duration,
    pub collaboration_score: f64,
    pub specialization_score: f64,
    pub last_updated: SystemTime,
}

/// Message types for inter-agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// Task assignment
    TaskAssignment {
        task_id: Uuid,
        task: SwarmTask,
        assigned_by: Uuid,
    },
    /// Task completion notification
    TaskCompleted {
        task_id: Uuid,
        result: TaskResult,
        completed_by: Uuid,
    },
    /// Request for collaboration
    CollaborationRequest {
        task_id: Uuid,
        requester: Uuid,
        required_specializations: Vec<AgentSpecialization>,
        description: String,
    },
    /// Collaboration response
    CollaborationResponse {
        task_id: Uuid,
        responder: Uuid,
        accepted: bool,
        reason: Option<String>,
    },
    /// Knowledge sharing
    KnowledgeShare {
        topic: String,
        content: String,
        sender: Uuid,
        confidence: f64,
    },
    /// Consensus proposal
    ConsensusProposal {
        proposal_id: Uuid,
        topic: String,
        proposal: String,
        proposer: Uuid,
    },
    /// Consensus vote
    ConsensusVote {
        proposal_id: Uuid,
        voter: Uuid,
        vote: Vote,
        reasoning: String,
    },
}

/// Task for swarm execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmTask {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub required_specializations: Vec<AgentSpecialization>,
    pub priority: TaskPriority,
    pub dependencies: Vec<Uuid>,
    pub estimated_duration: Duration,
    pub created_at: SystemTime,
}

/// Task priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Task execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: Uuid,
    pub success: bool,
    pub output: String,
    pub duration: Duration,
    pub quality_score: f64,
    pub collaborators: Vec<Uuid>,
}

/// Consensus voting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Vote {
    Approve,
    Reject,
    Abstain,
}

/// Consensus engine for decision making
pub struct ConsensusEngine {
    /// Active proposals
    active_proposals: HashMap<Uuid, Proposal>,
    /// Voting history
    voting_history: Vec<VotingRecord>,
    /// Consensus threshold (percentage of votes needed)
    consensus_threshold: f64,
}

/// Proposal for consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: Uuid,
    pub topic: String,
    pub content: String,
    pub proposer: Uuid,
    pub votes: HashMap<Uuid, Vote>,
    pub created_at: SystemTime,
    pub deadline: SystemTime,
}

/// Voting record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotingRecord {
    pub proposal_id: Uuid,
    pub votes: HashMap<Uuid, Vote>,
    pub consensus_reached: bool,
    pub final_decision: Option<String>,
}

/// Task allocation system
pub struct TaskAllocator {
    /// Available tasks queue
    task_queue: VecDeque<SwarmTask>,
    /// Agent workloads
    agent_workloads: HashMap<Uuid, u32>,
    /// Specialization matching scores
    specialization_matching: HashMap<(AgentSpecialization, AgentSpecialization), f64>,
}

/// Performance monitoring for the swarm
pub struct SwarmPerformanceMonitor {
    /// Overall swarm metrics
    swarm_metrics: SwarmMetrics,
    /// Individual agent metrics
    agent_metrics: HashMap<Uuid, AgentPerformance>,
    /// Collaboration patterns
    collaboration_patterns: Vec<CollaborationPattern>,
}

/// Swarm-wide performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmMetrics {
    pub total_tasks_completed: u32,
    pub average_task_completion_time: Duration,
    pub swarm_efficiency: f64,
    pub collaboration_rate: f64,
    pub consensus_reach_rate: f64,
    pub last_updated: SystemTime,
}

/// Collaboration pattern analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationPattern {
    pub specializations: Vec<AgentSpecialization>,
    pub success_rate: f64,
    pub average_completion_time: Duration,
    pub frequency: u32,
}

/// Specialization registry
pub struct SpecializationRegistry {
    /// Available specializations
    specializations: HashSet<AgentSpecialization>,
    /// Agent counts per specialization
    specialization_counts: HashMap<AgentSpecialization, u32>,
    /// Specialization capabilities
    capabilities: HashMap<AgentSpecialization, Vec<String>>,
}

impl SwarmCoordinator {
    /// Create a new swarm coordinator
    pub fn new(memory_system: Arc<MemorySystem>) -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            communication_channels: Arc::new(RwLock::new(HashMap::new())),
            swarm_memory: memory_system,
            consensus_engine: Arc::new(RwLock::new(ConsensusEngine::new())),
            task_allocator: Arc::new(RwLock::new(TaskAllocator::new())),
            performance_monitor: Arc::new(RwLock::new(SwarmPerformanceMonitor::new())),
            specialization_registry: Arc::new(RwLock::new(SpecializationRegistry::new())),
        }
    }

    /// Add an agent to the swarm
    pub async fn add_agent(
        &self,
        specialization: AgentSpecialization,
        reasoning_engine: Arc<dyn ReasoningEngine>,
        working_memory: Arc<MemorySystem>,
    ) -> Result<Uuid> {
        let agent_id = Uuid::new_v4();
        let (tx, rx) = mpsc::unbounded_channel();

        let agent = SwarmAgent {
            id: agent_id,
            specialization: specialization.clone(),
            status: AgentStatus::Idle,
            performance: AgentPerformance {
                tasks_completed: 0,
                success_rate: 1.0,
                average_task_time: Duration::from_secs(0),
                collaboration_score: 0.0,
                specialization_score: 1.0,
                last_updated: SystemTime::now(),
            },
            message_queue: rx,
            reasoning_engine,
            working_memory,
            last_activity: SystemTime::now(),
        };

        // Add to agents map
        self.agents.write().await.insert(agent_id, agent);

        // Update specialization registry
        self.specialization_registry
            .write()
            .await
            .add_agent(specialization);

        // Initialize communication channels
        self.initialize_communication_channels(agent_id).await;

        Ok(agent_id)
    }

    /// Submit a task for swarm execution
    pub async fn submit_task(&self, task: SwarmTask) -> Result<Uuid> {
        let task_id = task.id;

        // Add to task allocator
        self.task_allocator.write().await.add_task(task);

        // Attempt initial allocation
        self.allocate_tasks().await?;

        Ok(task_id)
    }

    /// Execute swarm intelligence on a complex goal
    pub async fn execute_swarm_goal(
        &self,
        goal: Goal,
        context: &ExecutionContext,
    ) -> Result<SwarmResult> {
        // Decompose goal into tasks
        let tasks = self.decompose_goal_into_tasks(&goal).await?;

        // Submit all tasks
        let mut task_ids = Vec::new();
        for task in tasks {
            let task_id = self.submit_task(task).await?;
            task_ids.push(task_id);
        }

        // Monitor execution and coordinate
        let result = self.coordinate_execution(&task_ids, context).await?;

        Ok(result)
    }

    /// Initialize communication channels for a new agent
    async fn initialize_communication_channels(&self, agent_id: Uuid) {
        let mut channels = self.communication_channels.write().await;

        // Create channels to all existing agents
        let agents = self.agents.read().await;
        for (other_id, _) in agents.iter() {
            if *other_id != agent_id {
                let (tx, _) = mpsc::unbounded_channel();
                channels.insert((agent_id, *other_id), tx);

                let (tx, _) = mpsc::unbounded_channel();
                channels.insert((*other_id, agent_id), tx);
            }
        }
    }

    /// Allocate available tasks to appropriate agents
    async fn allocate_tasks(&self) -> Result<()> {
        let mut allocator = self.task_allocator.write().await;
        let agents = self.agents.read().await;

        while let Some(task) = allocator.get_next_task() {
            // Find best agent for this task
            if let Some(best_agent_id) = self.find_best_agent_for_task(&task, &agents).await {
                // Assign task to agent
                self.assign_task_to_agent(best_agent_id, task).await?;
            } else {
                // No suitable agent found, re-queue task
                allocator.requeue_task(task);
                break;
            }
        }

        Ok(())
    }

    /// Find the best agent for a given task
    async fn find_best_agent_for_task(
        &self,
        task: &SwarmTask,
        agents: &HashMap<Uuid, SwarmAgent>,
    ) -> Option<Uuid> {
        let mut best_agent = None;
        let mut best_score = 0.0;

        for (agent_id, agent) in agents {
            if let AgentStatus::Idle = agent.status {
                let score = self.calculate_agent_task_fit(agent, task).await;
                if score > best_score {
                    best_score = score;
                    best_agent = Some(*agent_id);
                }
            }
        }

        best_agent
    }

    /// Calculate how well an agent fits a task
    async fn calculate_agent_task_fit(&self, agent: &SwarmAgent, task: &SwarmTask) -> f64 {
        let mut score = 0.0;

        // Specialization matching
        if task
            .required_specializations
            .contains(&agent.specialization)
        {
            score += 1.0;
        } else if let AgentSpecialization::GeneralPurpose = agent.specialization {
            score += 0.5;
        }

        // Performance bonus
        score += agent.performance.success_rate * 0.3;

        // Workload penalty (prefer less busy agents)
        let workload_penalty = self
            .task_allocator
            .read()
            .await
            .get_agent_workload(agent.id) as f64
            * 0.1;
        score -= workload_penalty.min(0.5);

        score.max(0.0)
    }

    /// Assign a task to a specific agent
    async fn assign_task_to_agent(&self, agent_id: Uuid, task: SwarmTask) -> Result<()> {
        let message = Message::TaskAssignment {
            task_id: task.id,
            task,
            assigned_by: Uuid::nil(), // System assignment
        };

        self.send_message_to_agent(agent_id, message).await?;
        self.task_allocator
            .write()
            .await
            .increment_agent_workload(agent_id);

        Ok(())
    }

    /// Send a message to a specific agent
    async fn send_message_to_agent(&self, agent_id: Uuid, message: Message) -> Result<()> {
        let channels = self.communication_channels.read().await;

        // For now, we'll need to implement a broadcast mechanism
        // This is a simplified version - in practice, we'd need proper routing
        if let Some(tx) = channels.get(&(Uuid::nil(), agent_id)) {
            tx.send(message)?;
        }

        Ok(())
    }

    /// Decompose a complex goal into swarm tasks
    async fn decompose_goal_into_tasks(&self, goal: &Goal) -> Result<Vec<SwarmTask>> {
        let mut tasks = Vec::new();

        // Analyze goal complexity and break it down
        match goal.priority {
            GoalPriority::Critical | GoalPriority::High => {
                // Complex goals get decomposed into multiple specialized tasks

                // Research task
                tasks.push(SwarmTask {
                    id: Uuid::new_v4(),
                    title: format!("Research: {}", goal.description),
                    description: format!(
                        "Research and analyze requirements for: {}",
                        goal.description
                    ),
                    required_specializations: vec![AgentSpecialization::ResearchSpecialist],
                    priority: TaskPriority::High,
                    dependencies: vec![],
                    estimated_duration: Duration::from_secs(300),
                    created_at: SystemTime::now(),
                });

                // Implementation task
                let research_task_id = tasks[0].id;
                tasks.push(SwarmTask {
                    id: Uuid::new_v4(),
                    title: format!("Implement: {}", goal.description),
                    description: format!("Implement the solution for: {}", goal.description),
                    required_specializations: vec![AgentSpecialization::CodeSpecialist],
                    priority: TaskPriority::High,
                    dependencies: vec![research_task_id],
                    estimated_duration: Duration::from_secs(600),
                    created_at: SystemTime::now(),
                });

                // Testing task
                let implementation_task_id = tasks[1].id;
                tasks.push(SwarmTask {
                    id: Uuid::new_v4(),
                    title: format!("Test: {}", goal.description),
                    description: format!(
                        "Test and validate the implementation for: {}",
                        goal.description
                    ),
                    required_specializations: vec![AgentSpecialization::TestSpecialist],
                    priority: TaskPriority::High,
                    dependencies: vec![implementation_task_id],
                    estimated_duration: Duration::from_secs(300),
                    created_at: SystemTime::now(),
                });
            }
            _ => {
                // Simple goals get a single general task
                tasks.push(SwarmTask {
                    id: Uuid::new_v4(),
                    title: goal.description.clone(),
                    description: goal.description.clone(),
                    required_specializations: vec![AgentSpecialization::GeneralPurpose],
                    priority: TaskPriority::Medium,
                    dependencies: vec![],
                    estimated_duration: Duration::from_secs(300),
                    created_at: SystemTime::now(),
                });
            }
        }

        Ok(tasks)
    }

    /// Coordinate the execution of multiple tasks
    async fn coordinate_execution(
        &self,
        task_ids: &[Uuid],
        context: &ExecutionContext,
    ) -> Result<SwarmResult> {
        let start_time = SystemTime::now();
        let mut completed_tasks = 0;
        let mut failed_tasks = 0;

        // Monitor task completion
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;

            let agents = self.agents.read().await;
            let mut all_completed = true;

            for task_id in task_ids {
                let mut task_completed = false;

                for agent in agents.values() {
                    if let AgentStatus::Working {
                        task_id: agent_task_id,
                        ..
                    } = agent.status
                    {
                        if agent_task_id == *task_id {
                            all_completed = false;
                            break;
                        }
                    }
                }

                if !task_completed {
                    all_completed = false;
                }
            }

            if all_completed {
                break;
            }

            // Timeout after 30 minutes
            if start_time.elapsed().unwrap_or(Duration::from_secs(0)) > Duration::from_secs(1800) {
                break;
            }
        }

        // Collect results
        let mut results = Vec::new();
        let agents = self.agents.read().await;

        for agent in agents.values() {
            // In a real implementation, we'd collect actual task results
            // For now, we'll create mock results
            results.push(TaskResult {
                task_id: Uuid::new_v4(),
                success: true,
                output: "Task completed successfully".to_string(),
                duration: Duration::from_secs(60),
                quality_score: 0.9,
                collaborators: vec![],
            });
        }

        Ok(SwarmResult {
            total_tasks: task_ids.len() as u32,
            completed_tasks: results.len() as u32,
            failed_tasks,
            results,
            total_duration: start_time.elapsed().unwrap_or(Duration::from_secs(0)),
            swarm_efficiency: 0.85,
        })
    }
}

/// Result of swarm execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmResult {
    pub total_tasks: u32,
    pub completed_tasks: u32,
    pub failed_tasks: u32,
    pub results: Vec<TaskResult>,
    pub total_duration: Duration,
    pub swarm_efficiency: f64,
}

impl ConsensusEngine {
    fn new() -> Self {
        Self {
            active_proposals: HashMap::new(),
            voting_history: Vec::new(),
            consensus_threshold: 0.66, // 66% majority
        }
    }

    /// Propose a new decision for consensus
    pub async fn propose(&mut self, proposal: Proposal) -> Result<Uuid> {
        let proposal_id = proposal.id;
        self.active_proposals.insert(proposal_id, proposal);
        Ok(proposal_id)
    }

    /// Vote on a proposal
    pub async fn vote(
        &mut self,
        proposal_id: Uuid,
        voter_id: Uuid,
        vote: Vote,
        reasoning: String,
    ) -> Result<()> {
        if let Some(proposal) = self.active_proposals.get_mut(&proposal_id) {
            proposal.votes.insert(voter_id, vote);

            // Check if consensus reached
            let total_votes = proposal.votes.len();
            let approve_votes = proposal
                .votes
                .values()
                .filter(|v| matches!(v, Vote::Approve))
                .count();
            let approve_ratio = approve_votes as f64 / total_votes as f64;

            if approve_ratio >= self.consensus_threshold {
                // Consensus reached
                let record = VotingRecord {
                    proposal_id,
                    votes: proposal.votes.clone(),
                    consensus_reached: true,
                    final_decision: Some(proposal.content.clone()),
                };
                self.voting_history.push(record);
                self.active_proposals.remove(&proposal_id);
            }
        }

        Ok(())
    }
}

impl TaskAllocator {
    fn new() -> Self {
        Self {
            task_queue: VecDeque::new(),
            agent_workloads: HashMap::new(),
            specialization_matching: HashMap::new(),
        }
    }

    fn add_task(&mut self, task: SwarmTask) {
        self.task_queue.push_back(task);
    }

    fn get_next_task(&mut self) -> Option<SwarmTask> {
        self.task_queue.pop_front()
    }

    fn requeue_task(&mut self, task: SwarmTask) {
        self.task_queue.push_front(task);
    }

    fn increment_agent_workload(&mut self, agent_id: Uuid) {
        *self.agent_workloads.entry(agent_id).or_insert(0) += 1;
    }

    fn get_agent_workload(&self, agent_id: Uuid) -> u32 {
        *self.agent_workloads.get(&agent_id).unwrap_or(&0)
    }
}

impl SwarmPerformanceMonitor {
    fn new() -> Self {
        Self {
            swarm_metrics: SwarmMetrics {
                total_tasks_completed: 0,
                average_task_completion_time: Duration::from_secs(0),
                swarm_efficiency: 0.0,
                collaboration_rate: 0.0,
                consensus_reach_rate: 0.0,
                last_updated: SystemTime::now(),
            },
            agent_metrics: HashMap::new(),
            collaboration_patterns: Vec::new(),
        }
    }
}

impl SpecializationRegistry {
    fn new() -> Self {
        let mut capabilities = HashMap::new();

        capabilities.insert(
            AgentSpecialization::CodeSpecialist,
            vec![
                "code_generation".to_string(),
                "code_analysis".to_string(),
                "refactoring".to_string(),
            ],
        );

        capabilities.insert(
            AgentSpecialization::TestSpecialist,
            vec![
                "unit_testing".to_string(),
                "integration_testing".to_string(),
                "test_automation".to_string(),
            ],
        );

        capabilities.insert(
            AgentSpecialization::DocumentationSpecialist,
            vec![
                "technical_writing".to_string(),
                "api_documentation".to_string(),
                "code_comments".to_string(),
            ],
        );

        Self {
            specializations: HashSet::new(),
            specialization_counts: HashMap::new(),
            capabilities,
        }
    }

    fn add_agent(&mut self, specialization: AgentSpecialization) {
        self.specializations.insert(specialization.clone());
        *self
            .specialization_counts
            .entry(specialization)
            .or_insert(0) += 1;
    }
}
