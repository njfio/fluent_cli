//! Hierarchical Task Networks (HTN) Planner for goal decomposition

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::goal::Goal;
use crate::context::ExecutionContext;
use fluent_core::traits::Engine;

/// HTN Planner for sophisticated goal decomposition
pub struct HTNPlanner {
    base_engine: Arc<dyn Engine>,
    config: HTNConfig,
    task_network: Arc<RwLock<TaskNetwork>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTNConfig {
    pub max_depth: u32,
    pub max_parallel: u32,
    pub timeout_secs: u64,
}

impl Default for HTNConfig {
    fn default() -> Self {
        Self { max_depth: 6, max_parallel: 8, timeout_secs: 300 }
    }
}

#[derive(Debug, Default)]
pub struct TaskNetwork {
    tasks: HashMap<String, NetworkTask>,
    root_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTask {
    pub id: String,
    pub description: String,
    pub task_type: TaskType,
    pub parent_id: Option<String>,
    pub children: Vec<String>,
    pub depth: u32,
    pub status: TaskStatus,
    pub effort: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType { Compound, Primitive }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus { Pending, Ready, InProgress, Complete, Failed }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub plan_id: String,
    pub phases: Vec<ExecutionPhase>,
    pub total_time: Duration,
    pub parallel_groups: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPhase {
    pub phase_id: String,
    pub tasks: Vec<String>,
    pub duration: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTNResult {
    pub plan: ExecutionPlan,
    pub tasks: Vec<NetworkTask>,
    pub metrics: PlanMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanMetrics {
    pub total_tasks: u32,
    pub max_depth: u32,
    pub planning_time: Duration,
    pub feasibility: f64,
}

impl HTNPlanner {
    pub fn new(engine: Arc<dyn Engine>, config: HTNConfig) -> Self {
        Self {
            base_engine: engine,
            config,
            task_network: Arc::new(RwLock::new(TaskNetwork::default())),
        }
    }

    /// Plan goal decomposition using HTN
    pub async fn plan_decomposition(&self, goal: &Goal, context: &ExecutionContext) -> Result<HTNResult> {
        let start = SystemTime::now();
        
        // Create root task
        let root = self.create_root_task(goal).await?;
        self.init_network(root).await?;
        
        // Decompose recursively
        self.decompose_tasks(context).await?;
        
        // Generate execution plan
        let plan = self.create_plan().await?;
        
        let network = self.task_network.read().await;
        let tasks: Vec<NetworkTask> = network.tasks.values().cloned().collect();
        
        Ok(HTNResult {
            plan,
            tasks: tasks.clone(),
            metrics: PlanMetrics {
                total_tasks: tasks.len() as u32,
                max_depth: tasks.iter().map(|t| t.depth).max().unwrap_or(0),
                planning_time: SystemTime::now().duration_since(start).unwrap_or_default(),
                feasibility: 0.85,
            },
        })
    }

    async fn create_root_task(&self, goal: &Goal) -> Result<NetworkTask> {
        Ok(NetworkTask {
            id: Uuid::new_v4().to_string(),
            description: goal.description.clone(),
            task_type: TaskType::Compound,
            parent_id: None,
            children: Vec::new(),
            depth: 0,
            status: TaskStatus::Pending,
            effort: 1.0,
        })
    }

    async fn init_network(&self, root: NetworkTask) -> Result<()> {
        let mut network = self.task_network.write().await;
        let id = root.id.clone();
        network.tasks.insert(id.clone(), root);
        network.root_id = Some(id);
        Ok(())
    }

    async fn decompose_tasks(&self, context: &ExecutionContext) -> Result<()> {
        let mut to_process = Vec::new();
        
        if let Some(root_id) = &self.task_network.read().await.root_id {
            to_process.push(root_id.clone());
        }

        while let Some(task_id) = to_process.pop() {
            let task = self.task_network.read().await.tasks.get(&task_id).cloned();
            
            if let Some(task) = task {
                if matches!(task.task_type, TaskType::Compound) && task.depth < self.config.max_depth {
                    let subtasks = self.decompose_task(&task, context).await?;
                    
                    let mut network = self.task_network.write().await;
                    for subtask in subtasks {
                        let subtask_id = subtask.id.clone();
                        
                        // Update parent
                        if let Some(parent) = network.tasks.get_mut(&task_id) {
                            parent.children.push(subtask_id.clone());
                        }
                        
                        // Queue compound tasks for further decomposition
                        if matches!(subtask.task_type, TaskType::Compound) {
                            to_process.push(subtask_id.clone());
                        }
                        
                        network.tasks.insert(subtask_id, subtask);
                    }
                }
            }
        }
        Ok(())
    }

    async fn decompose_task(&self, task: &NetworkTask, _context: &ExecutionContext) -> Result<Vec<NetworkTask>> {
        let prompt = format!(
            "Break down this task into 3-5 concrete subtasks:\n\nTask: {}\nDepth: {}\n\nFormat each as:\nSUBTASK: [description]\nTYPE: [primitive/compound]",
            task.description, task.depth
        );

        let request = fluent_core::types::Request {
            flowname: "htn_decompose".to_string(),
            payload: prompt,
        };

        let response = std::pin::Pin::from(self.base_engine.execute(&request)).await?;
        self.parse_subtasks(&response.content, task)
    }

    fn parse_subtasks(&self, response: &str, parent: &NetworkTask) -> Result<Vec<NetworkTask>> {
        let mut subtasks = Vec::new();
        let mut current: Option<NetworkTask> = None;
        
        for line in response.lines() {
            let line = line.trim();
            
            if line.starts_with("SUBTASK:") {
                if let Some(task) = current.take() {
                    subtasks.push(task);
                }
                
                if let Some(desc) = line.strip_prefix("SUBTASK:") {
                    current = Some(NetworkTask {
                        id: Uuid::new_v4().to_string(),
                        description: desc.trim().to_string(),
                        task_type: TaskType::Primitive,
                        parent_id: Some(parent.id.clone()),
                        children: Vec::new(),
                        depth: parent.depth + 1,
                        status: TaskStatus::Pending,
                        effort: 1.0,
                    });
                }
            } else if line.starts_with("TYPE:") && line.contains("compound") {
                if let Some(ref mut task) = current {
                    task.task_type = TaskType::Compound;
                }
            }
        }
        
        if let Some(task) = current {
            subtasks.push(task);
        }
        
        Ok(subtasks)
    }

    async fn create_plan(&self) -> Result<ExecutionPlan> {
        let network = self.task_network.read().await;
        
        // Find primitive (executable) tasks
        let primitives: Vec<_> = network.tasks.values()
            .filter(|t| matches!(t.task_type, TaskType::Primitive) && t.children.is_empty())
            .cloned()
            .collect();

        // Create simple sequential phases
        let mut phases = Vec::new();
        for (i, task) in primitives.iter().enumerate() {
            phases.push(ExecutionPhase {
                phase_id: Uuid::new_v4().to_string(),
                tasks: vec![task.id.clone()],
                duration: Duration::from_secs((task.effort * 300.0) as u64), // 5 min base
            });
        }

        // Identify parallel opportunities (independent tasks)
        let independent: Vec<String> = primitives.iter()
            .filter(|t| t.parent_id != network.root_id) // Not direct children of root
            .map(|t| t.id.clone())
            .collect();

        let parallel_groups = if independent.len() > 1 {
            vec![independent]
        } else {
            Vec::new()
        };

        Ok(ExecutionPlan {
            plan_id: Uuid::new_v4().to_string(),
            phases,
            total_time: Duration::from_secs(primitives.len() as u64 * 300), // Estimated
            parallel_groups,
        })
    }
}