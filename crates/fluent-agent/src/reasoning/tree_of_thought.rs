//! Tree-of-Thought Reasoning Engine
//!
//! This module implements the Tree-of-Thought (ToT) reasoning pattern, which enables
//! the agent to explore multiple solution paths simultaneously and choose the most
//! promising approach for complex problem solving.
//!
//! The ToT approach maintains a tree of possible reasoning paths, evaluates each
//! path's promise, and can backtrack when a path proves unfruitful. This is
//! particularly useful for complex, multi-step problems where the optimal
//! approach isn't immediately obvious.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::reasoning::{ReasoningEngine, ReasoningCapability};
use crate::context::ExecutionContext;
use fluent_core::traits::Engine;

/// Tree-of-Thought reasoning engine that explores multiple solution paths
pub struct TreeOfThoughtEngine {
    base_engine: Arc<dyn Engine>,
    config: ToTConfig,
    thought_tree: Arc<RwLock<ThoughtTree>>,
    evaluation_cache: Arc<RwLock<HashMap<String, f64>>>,
}

/// Configuration for Tree-of-Thought reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToTConfig {
    /// Maximum depth of the thought tree
    pub max_depth: u32,
    /// Maximum number of branches at each level
    pub max_branches: u32,
    /// Minimum confidence threshold for exploring a branch
    pub confidence_threshold: f64,
    /// Number of thoughts to consider at each step
    pub thoughts_per_step: u32,
    /// Enable pruning of low-quality branches
    pub enable_pruning: bool,
    /// Pruning threshold - branches below this score get pruned
    pub pruning_threshold: f64,
    /// Maximum time to spend on tree exploration
    pub max_exploration_time: Duration,
    /// Enable parallel branch exploration
    pub enable_parallel_exploration: bool,
}

impl Default for ToTConfig {
    fn default() -> Self {
        Self {
            max_depth: 8,
            max_branches: 4,
            confidence_threshold: 0.3,
            thoughts_per_step: 3,
            enable_pruning: true,
            pruning_threshold: 0.2,
            max_exploration_time: Duration::from_secs(300), // 5 minutes
            enable_parallel_exploration: true,
        }
    }
}

/// A node in the thought tree representing a reasoning step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub depth: u32,
    pub thought_content: String,
    pub confidence_score: f64,
    pub evaluation_score: f64,
    pub reasoning_type: ThoughtType,
    pub children: Vec<String>,
    pub created_at: SystemTime,
    pub is_terminal: bool,
    pub path_context: String,
    pub accumulated_confidence: f64,
}

/// Types of thoughts in the reasoning tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThoughtType {
    /// Initial problem analysis
    ProblemAnalysis,
    /// Approach consideration
    ApproachExploration,
    /// Sub-problem decomposition
    SubProblemBreakdown,
    /// Solution attempt
    SolutionAttempt,
    /// Evaluation and validation
    EvaluationCheck,
    /// Alternative exploration
    AlternativeExploration,
    /// Synthesis and integration
    SynthesisAttempt,
    /// Final solution
    FinalSolution,
}

/// The complete thought tree structure
#[derive(Debug, Default)]
pub struct ThoughtTree {
    nodes: HashMap<String, ThoughtNode>,
    root_id: Option<String>,
    active_paths: Vec<String>, // Currently active leaf nodes
    completed_paths: Vec<ReasoningPath>,
    best_path: Option<ReasoningPath>,
    tree_metrics: TreeMetrics,
}

/// A complete reasoning path from root to leaf
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningPath {
    pub path_id: String,
    pub node_sequence: Vec<String>,
    pub total_confidence: f64,
    pub path_quality: f64,
    pub reasoning_chain: Vec<String>,
    pub final_conclusion: String,
    pub path_evaluation: String,
}

/// Metrics for monitoring tree exploration
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TreeMetrics {
    pub total_nodes: usize,
    pub max_depth_reached: u32,
    pub average_branch_factor: f64,
    pub exploration_time: Duration,
    pub paths_explored: usize,
    pub paths_pruned: usize,
    pub best_path_confidence: f64,
}

/// Result of tree-of-thought reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToTReasoningResult {
    pub best_path: ReasoningPath,
    pub alternative_paths: Vec<ReasoningPath>,
    pub exploration_summary: String,
    pub tree_metrics: TreeMetrics,
    pub reasoning_confidence: f64,
    pub exploration_completeness: f64,
}

impl TreeOfThoughtEngine {
    /// Create a new Tree-of-Thought reasoning engine
    pub fn new(base_engine: Arc<dyn Engine>, config: ToTConfig) -> Self {
        Self {
            base_engine,
            config,
            thought_tree: Arc::new(RwLock::new(ThoughtTree::default())),
            evaluation_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Perform tree-of-thought reasoning on a problem
    pub async fn reason_with_tree(&self, problem: &str, context: &ExecutionContext) -> Result<ToTReasoningResult> {
        let start_time = SystemTime::now();
        
        // Initialize the tree with root problem analysis
        let root_id = self.initialize_tree(problem, context).await?;
        
        // Explore the tree breadth-first with depth limits
        self.explore_tree(root_id.clone(), start_time).await?;
        
        // Select the best reasoning path
        let result = self.select_best_path().await?;
        
        Ok(result)
    }

    /// Initialize the thought tree with the root problem
    async fn initialize_tree(&self, problem: &str, context: &ExecutionContext) -> Result<String> {
        let root_id = Uuid::new_v4().to_string();
        
        // Generate initial problem analysis thoughts
        let initial_thoughts = self.generate_initial_thoughts(problem, context).await?;
        
        let mut tree = self.thought_tree.write().await;
        
        // Create root node
        let root_node = ThoughtNode {
            id: root_id.clone(),
            parent_id: None,
            depth: 0,
            thought_content: format!("Problem Analysis: {}", problem),
            confidence_score: 1.0,
            evaluation_score: 0.5, // Neutral start
            reasoning_type: ThoughtType::ProblemAnalysis,
            children: Vec::new(),
            created_at: SystemTime::now(),
            is_terminal: false,
            path_context: problem.to_string(),
            accumulated_confidence: 1.0,
        };
        
        tree.nodes.insert(root_id.clone(), root_node);
        tree.root_id = Some(root_id.clone());
        
        // Add initial thought branches
        for (i, thought) in initial_thoughts.iter().enumerate() {
            let child_id = self.add_thought_branch(
                &root_id,
                thought,
                ThoughtType::ApproachExploration,
                &mut tree,
            ).await?;
            
            tree.active_paths.push(child_id);
        }
        
        Ok(root_id)
    }

    /// Generate initial thoughts for the problem
    async fn generate_initial_thoughts(&self, problem: &str, context: &ExecutionContext) -> Result<Vec<String>> {
        let prompt = format!(
            r#"Given this problem: "{}"

Context: {}

Generate {} distinct initial approaches for solving this problem. Each approach should:
1. Be a clear, different strategy
2. Consider the problem from a unique angle  
3. Be feasible given the context
4. Provide a specific starting direction

Format your response as numbered approaches:
1. [First approach]
2. [Second approach]  
3. [Third approach]"#,
            problem,
            self.format_context_summary(context),
            self.config.thoughts_per_step
        );

        let request = fluent_core::types::Request {
            flowname: "tree_of_thought_initial".to_string(),
            payload: prompt,
        };

        let response = std::pin::Pin::from(self.base_engine.execute(&request)).await?;
        let thoughts = self.parse_numbered_thoughts(&response.content)?;
        
        Ok(thoughts)
    }

    /// Explore the thought tree using breadth-first search with evaluation
    async fn explore_tree(&self, root_id: String, start_time: SystemTime) -> Result<()> {
        let mut exploration_queue = VecDeque::new();
        exploration_queue.push_back(root_id);

        while !exploration_queue.is_empty() {
            // Check time limit
            if SystemTime::now().duration_since(start_time).unwrap_or_default() > self.config.max_exploration_time {
                break;
            }

            let current_node_id = exploration_queue.pop_front().unwrap();
            
            // Get current node
            let current_node = {
                let tree = self.thought_tree.read().await;
                tree.nodes.get(&current_node_id).cloned()
            };

            if let Some(node) = current_node {
                // Skip if we've reached max depth or node is terminal
                if node.depth >= self.config.max_depth || node.is_terminal {
                    continue;
                }

                // Skip if confidence is too low
                if node.accumulated_confidence < self.config.confidence_threshold {
                    continue;
                }

                // Generate next level thoughts
                let next_thoughts = self.generate_next_thoughts(&node).await?;
                
                // Evaluate and add promising thoughts
                for thought in next_thoughts {
                    let confidence = self.evaluate_thought_quality(&thought, &node).await?;
                    
                    if confidence >= self.config.confidence_threshold {
                        let child_id = {
                            let mut tree = self.thought_tree.write().await;
                            self.add_evaluated_thought_branch(&node.id, &thought, confidence, &mut tree).await?
                        };
                        
                        exploration_queue.push_back(child_id);
                    }
                }

                // Prune low-quality branches if enabled
                if self.config.enable_pruning {
                    self.prune_low_quality_branches(&current_node_id).await?;
                }
            }
        }

        Ok(())
    }

    /// Generate next-level thoughts based on current node
    async fn generate_next_thoughts(&self, node: &ThoughtNode) -> Result<Vec<String>> {
        let next_type = self.determine_next_thought_type(&node.reasoning_type);
        
        let prompt = format!(
            r#"Current reasoning path: {}

Current thought: "{}"
Depth: {}
Confidence so far: {:.2}

Generate {} distinct next thoughts of type {:?}. Each should:
1. Build logically on the current thought
2. Explore a different aspect or direction
3. Be specific and actionable
4. Maintain reasoning coherence

Format as numbered thoughts:
1. [First thought]
2. [Second thought]
3. [Third thought]"#,
            node.path_context,
            node.thought_content,
            node.depth,
            node.accumulated_confidence,
            self.config.thoughts_per_step,
            next_type
        );

        let request = fluent_core::types::Request {
            flowname: "tree_of_thought_expand".to_string(),
            payload: prompt,
        };

        let response = std::pin::Pin::from(self.base_engine.execute(&request)).await?;
        let thoughts = self.parse_numbered_thoughts(&response.content)?;
        
        Ok(thoughts)
    }

    /// Evaluate the quality of a thought
    async fn evaluate_thought_quality(&self, thought: &str, parent_node: &ThoughtNode) -> Result<f64> {
        // Check cache first
        let cache_key = format!("{}:{}", parent_node.id, thought);
        {
            let cache = self.evaluation_cache.read().await;
            if let Some(score) = cache.get(&cache_key) {
                return Ok(*score);
            }
        }

        let prompt = format!(
            r#"Evaluate this reasoning thought:

Current path: {}
Parent thought: "{}"
New thought: "{}"

Rate this thought on a scale of 0.0 to 1.0 considering:
1. Logical consistency with the path so far (0.3 weight)
2. Likelihood to lead to a good solution (0.3 weight)  
3. Clarity and specificity (0.2 weight)
4. Novelty and creativity (0.2 weight)

Respond with just the numerical score (e.g., 0.75)"#,
            parent_node.path_context,
            parent_node.thought_content,
            thought
        );

        let request = fluent_core::types::Request {
            flowname: "tree_of_thought_evaluate".to_string(),
            payload: prompt,
        };

        let response = std::pin::Pin::from(self.base_engine.execute(&request)).await?;
        let score = response.content.trim().parse::<f64>().unwrap_or(0.5);
        let clamped_score = score.clamp(0.0, 1.0);

        // Cache the result
        {
            let mut cache = self.evaluation_cache.write().await;
            cache.insert(cache_key, clamped_score);
        }

        Ok(clamped_score)
    }

    /// Add a thought branch with evaluation to the tree
    async fn add_evaluated_thought_branch(
        &self,
        parent_id: &str,
        thought: &str,
        confidence: f64,
        tree: &mut ThoughtTree,
    ) -> Result<String> {
        let child_id = Uuid::new_v4().to_string();
        
        // Get parent data first before any mutable borrows
        let (parent_depth, parent_path_context, parent_confidence) = {
            let parent = tree.nodes.get(parent_id).unwrap();
            (parent.depth, parent.path_context.clone(), parent.accumulated_confidence)
        };
        
        let accumulated_confidence = parent_confidence * confidence;
        
        let child_node = ThoughtNode {
            id: child_id.clone(),
            parent_id: Some(parent_id.to_string()),
            depth: parent_depth + 1,
            thought_content: thought.to_string(),
            confidence_score: confidence,
            evaluation_score: confidence,
            reasoning_type: self.determine_next_thought_type(&{
                tree.nodes.get(parent_id).unwrap().reasoning_type.clone()
            }),
            children: Vec::new(),
            created_at: SystemTime::now(),
            is_terminal: self.is_terminal_thought(thought),
            path_context: format!("{} -> {}", parent_path_context, thought),
            accumulated_confidence,
        };

        // Update parent's children list
        if let Some(parent_node) = tree.nodes.get_mut(parent_id) {
            parent_node.children.push(child_id.clone());
        }

        tree.nodes.insert(child_id.clone(), child_node);
        tree.tree_metrics.total_nodes += 1;
        tree.tree_metrics.max_depth_reached = tree.tree_metrics.max_depth_reached.max(parent_depth + 1);

        Ok(child_id)
    }

    /// Add a simple thought branch to the tree  
    async fn add_thought_branch(
        &self,
        parent_id: &str,
        thought: &str,
        thought_type: ThoughtType,
        tree: &mut ThoughtTree,
    ) -> Result<String> {
        let child_id = Uuid::new_v4().to_string();
        
        let parent = tree.nodes.get(parent_id).unwrap();
        
        let child_node = ThoughtNode {
            id: child_id.clone(),
            parent_id: Some(parent_id.to_string()),
            depth: parent.depth + 1,
            thought_content: thought.to_string(),
            confidence_score: 0.7, // Default confidence
            evaluation_score: 0.5,
            reasoning_type: thought_type,
            children: Vec::new(),
            created_at: SystemTime::now(),
            is_terminal: false,
            path_context: format!("{} -> {}", parent.path_context, thought),
            accumulated_confidence: parent.accumulated_confidence * 0.7,
        };

        // Update parent's children list
        if let Some(parent_node) = tree.nodes.get_mut(parent_id) {
            parent_node.children.push(child_id.clone());
        }

        tree.nodes.insert(child_id.clone(), child_node);
        
        Ok(child_id)
    }

    /// Select the best reasoning path from the tree
    async fn select_best_path(&self) -> Result<ToTReasoningResult> {
        let tree = self.thought_tree.read().await;
        
        // Find all complete paths (leaf nodes)
        let leaf_nodes: Vec<&ThoughtNode> = tree.nodes.values()
            .filter(|node| node.children.is_empty() || node.is_terminal)
            .collect();

        let mut paths = Vec::new();
        
        for leaf in leaf_nodes {
            let path = self.build_reasoning_path(leaf, &tree)?;
            paths.push(path);
        }

        // Sort paths by quality (combination of confidence and completeness)
        paths.sort_by(|a, b| {
            let score_a = a.path_quality;
            let score_b = b.path_quality;
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        let best_path = paths.first().cloned().unwrap_or_else(|| {
            // Fallback empty path
            ReasoningPath {
                path_id: Uuid::new_v4().to_string(),
                node_sequence: Vec::new(),
                total_confidence: 0.0,
                path_quality: 0.0,
                reasoning_chain: vec!["No valid paths found".to_string()],
                final_conclusion: "Unable to reach a conclusion".to_string(),
                path_evaluation: "Exploration incomplete".to_string(),
            }
        });

        let alternative_paths = paths.into_iter().skip(1).take(3).collect();

        Ok(ToTReasoningResult {
            best_path,
            alternative_paths,
            exploration_summary: self.generate_exploration_summary(&tree).await?,
            tree_metrics: tree.tree_metrics.clone(),
            reasoning_confidence: tree.tree_metrics.best_path_confidence,
            exploration_completeness: self.calculate_exploration_completeness(&tree),
        })
    }

    /// Build a reasoning path from leaf to root
    fn build_reasoning_path(&self, leaf_node: &ThoughtNode, tree: &ThoughtTree) -> Result<ReasoningPath> {
        let mut path_nodes = Vec::new();
        let mut current_node = leaf_node;
        
        // Traverse from leaf to root
        loop {
            path_nodes.push(current_node.clone());
            
            if let Some(parent_id) = &current_node.parent_id {
                if let Some(parent) = tree.nodes.get(parent_id) {
                    current_node = parent;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        
        // Reverse to get root-to-leaf order
        path_nodes.reverse();
        
        let reasoning_chain: Vec<String> = path_nodes.iter()
            .map(|node| node.thought_content.clone())
            .collect();
            
        let node_sequence: Vec<String> = path_nodes.iter()
            .map(|node| node.id.clone())
            .collect();

        // Calculate path quality based on confidence and depth
        let path_quality = leaf_node.accumulated_confidence * 
            (1.0 + (leaf_node.depth as f64 * 0.1)); // Bonus for deeper reasoning

        Ok(ReasoningPath {
            path_id: Uuid::new_v4().to_string(),
            node_sequence,
            total_confidence: leaf_node.accumulated_confidence,
            path_quality,
            reasoning_chain,
            final_conclusion: leaf_node.thought_content.clone(),
            path_evaluation: format!(
                "Path confidence: {:.2}, Depth: {}, Quality: {:.2}",
                leaf_node.accumulated_confidence,
                leaf_node.depth,
                path_quality
            ),
        })
    }

    // Helper methods

    fn determine_next_thought_type(&self, current_type: &ThoughtType) -> ThoughtType {
        match current_type {
            ThoughtType::ProblemAnalysis => ThoughtType::ApproachExploration,
            ThoughtType::ApproachExploration => ThoughtType::SubProblemBreakdown,
            ThoughtType::SubProblemBreakdown => ThoughtType::SolutionAttempt,
            ThoughtType::SolutionAttempt => ThoughtType::EvaluationCheck,
            ThoughtType::EvaluationCheck => ThoughtType::AlternativeExploration,
            ThoughtType::AlternativeExploration => ThoughtType::SynthesisAttempt,
            ThoughtType::SynthesisAttempt => ThoughtType::FinalSolution,
            ThoughtType::FinalSolution => ThoughtType::FinalSolution,
        }
    }

    fn is_terminal_thought(&self, thought: &str) -> bool {
        thought.to_lowercase().contains("final") || 
        thought.to_lowercase().contains("conclusion") ||
        thought.to_lowercase().contains("complete")
    }

    fn parse_numbered_thoughts(&self, text: &str) -> Result<Vec<String>> {
        let mut thoughts = Vec::new();
        
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with(char::is_numeric) {
                // Extract content after the number and dot/parenthesis
                if let Some(content_start) = trimmed.find('.').or_else(|| trimmed.find(')')).map(|i| i + 1) {
                    let content = trimmed[content_start..].trim();
                    if !content.is_empty() {
                        thoughts.push(content.to_string());
                    }
                }
            }
        }
        
        if thoughts.is_empty() {
            // Fallback: split by lines and take non-empty ones
            thoughts = text.lines()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .take(self.config.thoughts_per_step as usize)
                .collect();
        }
        
        Ok(thoughts)
    }

    fn format_context_summary(&self, context: &ExecutionContext) -> String {
        format!(
            "Goal: {}, Context items: {}, Iteration: {}",
            context.current_goal.as_ref()
                .map(|g| g.description.clone())
                .unwrap_or_else(|| "No goal set".to_string()),
            context.context_data.len(),
            context.iteration_count
        )
    }

    async fn prune_low_quality_branches(&self, _parent_id: &str) -> Result<()> {
        // TODO: Implement branch pruning based on quality thresholds
        // This would remove branches that consistently produce low-quality thoughts
        Ok(())
    }

    async fn generate_exploration_summary(&self, tree: &ThoughtTree) -> Result<String> {
        Ok(format!(
            "Explored {} nodes across {} levels. Found {} complete reasoning paths. Best path confidence: {:.2}",
            tree.tree_metrics.total_nodes,
            tree.tree_metrics.max_depth_reached,
            tree.completed_paths.len(),
            tree.tree_metrics.best_path_confidence
        ))
    }

    fn calculate_exploration_completeness(&self, tree: &ThoughtTree) -> f64 {
        // Simple completeness metric based on tree size and depth
        let expected_nodes = (self.config.max_branches as f64).powf(tree.tree_metrics.max_depth_reached as f64);
        let actual_nodes = tree.tree_metrics.total_nodes as f64;
        (actual_nodes / expected_nodes).min(1.0)
    }
}

#[async_trait::async_trait]
impl ReasoningEngine for TreeOfThoughtEngine {
    async fn reason(&self, prompt: &str, context: &ExecutionContext) -> Result<String> {
        let result = self.reason_with_tree(prompt, context).await?;
        
        let summary = format!(
            "Tree-of-Thought Reasoning Result:\n\nBest Path:\n{}\n\nConfidence: {:.2}\nExploration: {}",
            result.best_path.reasoning_chain.join("\n -> "),
            result.reasoning_confidence,
            result.exploration_summary
        );
        
        Ok(summary)
    }

    async fn get_capabilities(&self) -> Vec<ReasoningCapability> {
        vec![
            ReasoningCapability::MultiPathExploration,
            ReasoningCapability::QualityEvaluation,
            ReasoningCapability::BacktrackingSearch,
            ReasoningCapability::ConfidenceScoring,
        ]
    }

    async fn get_confidence(&self) -> f64 {
        let tree = self.thought_tree.read().await;
        tree.tree_metrics.best_path_confidence
    }
}