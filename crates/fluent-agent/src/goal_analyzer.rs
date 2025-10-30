//! Goal analysis and decomposition module
//!
//! This module provides intelligent goal analysis, decomposition,
//! and planning capabilities for agent goals.

use crate::goal::{Goal, GoalComplexity, GoalPriority, GoalType};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of goal analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalAnalysis {
    /// The original goal
    pub goal: Goal,
    /// Decomposed sub-goals with dependencies
    pub sub_goals: Vec<SubGoal>,
    /// Estimated complexity
    pub estimated_complexity: GoalComplexity,
    /// Estimated number of iterations needed
    pub estimated_iterations: u32,
    /// Required tools and capabilities
    pub required_tools: Vec<String>,
    /// Required capabilities
    pub required_capabilities: Vec<String>,
    /// Execution plan with milestones
    pub execution_plan: ExecutionPlan,
}

/// Sub-goal with dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubGoal {
    /// Sub-goal description
    pub description: String,
    /// Sub-goal type
    pub goal_type: GoalType,
    /// Dependencies (indices of other sub-goals)
    pub dependencies: Vec<usize>,
    /// Estimated iterations
    pub estimated_iterations: u32,
    /// Required tools for this sub-goal
    pub required_tools: Vec<String>,
}

/// Execution plan with milestones
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// Milestones in the plan
    pub milestones: Vec<Milestone>,
    /// Estimated total duration
    pub estimated_duration_secs: u64,
}

/// Milestone in execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    /// Milestone name
    pub name: String,
    /// Description
    pub description: String,
    /// Sub-goal indices that contribute to this milestone
    pub sub_goal_indices: Vec<usize>,
    /// Estimated completion time (relative to start)
    pub estimated_time_secs: u64,
}

/// Goal analyzer
pub struct GoalAnalyzer;

impl GoalAnalyzer {
    /// Analyze a goal and decompose it into sub-goals
    pub fn analyze_goal(goal: &Goal) -> Result<GoalAnalysis> {
        // Estimate complexity
        let estimated_complexity = goal.get_complexity();

        // Decompose into sub-goals
        let sub_goals = Self::decompose_goal(goal)?;

        // Estimate iterations
        let estimated_iterations = Self::estimate_iterations(goal, &sub_goals);

        // Identify required tools
        let required_tools = Self::identify_required_tools(goal, &sub_goals);

        // Identify required capabilities
        let required_capabilities = Self::identify_required_capabilities(goal);

        // Create execution plan
        let execution_plan = Self::create_execution_plan(&sub_goals)?;

        Ok(GoalAnalysis {
            goal: goal.clone(),
            sub_goals,
            estimated_complexity,
            estimated_iterations,
            required_tools,
            required_capabilities,
            execution_plan,
        })
    }

    /// Decompose a goal into sub-goals
    fn decompose_goal(goal: &Goal) -> Result<Vec<SubGoal>> {
        let mut sub_goals = Vec::new();

        match goal.goal_type {
            GoalType::CodeGeneration => {
                // Decompose code generation into: analysis, design, implementation, testing
                sub_goals.push(SubGoal {
                    description: format!("Analyze requirements for: {}", goal.description),
                    goal_type: GoalType::Analysis,
                    dependencies: vec![],
                    estimated_iterations: 2,
                    required_tools: vec!["read_file".to_string(), "analyze_code".to_string()],
                });

                sub_goals.push(SubGoal {
                    description: format!("Design structure for: {}", goal.description),
                    goal_type: GoalType::Planning,
                    dependencies: vec![0],
                    estimated_iterations: 2,
                    required_tools: vec![],
                });

                sub_goals.push(SubGoal {
                    description: format!("Implement: {}", goal.description),
                    goal_type: GoalType::CodeGeneration,
                    dependencies: vec![0, 1],
                    estimated_iterations: 5,
                    required_tools: vec!["write_file".to_string(), "run_command".to_string()],
                });

                sub_goals.push(SubGoal {
                    description: format!("Test and validate: {}", goal.description),
                    goal_type: GoalType::Testing,
                    dependencies: vec![2],
                    estimated_iterations: 3,
                    required_tools: vec!["run_command".to_string(), "read_file".to_string()],
                });
            }
            GoalType::Research => {
                sub_goals.push(SubGoal {
                    description: format!("Research topic: {}", goal.description),
                    goal_type: GoalType::Research,
                    dependencies: vec![],
                    estimated_iterations: 5,
                    required_tools: vec!["read_file".to_string(), "search".to_string()],
                });

                sub_goals.push(SubGoal {
                    description: format!("Analyze findings for: {}", goal.description),
                    goal_type: GoalType::Analysis,
                    dependencies: vec![0],
                    estimated_iterations: 3,
                    required_tools: vec!["read_file".to_string()],
                });

                sub_goals.push(SubGoal {
                    description: format!("Document research: {}", goal.description),
                    goal_type: GoalType::Documentation,
                    dependencies: vec![0, 1],
                    estimated_iterations: 2,
                    required_tools: vec!["write_file".to_string()],
                });
            }
            GoalType::Debugging => {
                sub_goals.push(SubGoal {
                    description: format!("Investigate issue: {}", goal.description),
                    goal_type: GoalType::Analysis,
                    dependencies: vec![],
                    estimated_iterations: 3,
                    required_tools: vec!["read_file".to_string(), "run_command".to_string()],
                });

                sub_goals.push(SubGoal {
                    description: format!("Identify root cause: {}", goal.description),
                    goal_type: GoalType::Analysis,
                    dependencies: vec![0],
                    estimated_iterations: 4,
                    required_tools: vec!["read_file".to_string(), "analyze_code".to_string()],
                });

                sub_goals.push(SubGoal {
                    description: format!("Fix issue: {}", goal.description),
                    goal_type: GoalType::CodeGeneration,
                    dependencies: vec![0, 1],
                    estimated_iterations: 4,
                    required_tools: vec!["write_file".to_string(), "run_command".to_string()],
                });

                sub_goals.push(SubGoal {
                    description: format!("Verify fix: {}", goal.description),
                    goal_type: GoalType::Testing,
                    dependencies: vec![2],
                    estimated_iterations: 2,
                    required_tools: vec!["run_command".to_string()],
                });
            }
            _ => {
                // For other goal types, create a single sub-goal
                sub_goals.push(SubGoal {
                    description: goal.description.clone(),
                    goal_type: goal.goal_type.clone(),
                    dependencies: vec![],
                    estimated_iterations: goal.max_iterations.unwrap_or(10),
                    required_tools: Self::get_default_tools_for_type(&goal.goal_type),
                });
            }
        }

        Ok(sub_goals)
    }

    /// Estimate total iterations needed
    fn estimate_iterations(goal: &Goal, sub_goals: &[SubGoal]) -> u32 {
        // Base estimate from goal complexity
        let base_estimate = match goal.get_complexity() {
            GoalComplexity::Low => 3,
            GoalComplexity::Medium => 8,
            GoalComplexity::High => 15,
        };

        // Add estimates from sub-goals (accounting for dependencies)
        let sub_goal_estimate: u32 = sub_goals.iter().map(|sg| sg.estimated_iterations).sum();

        // Use the higher of the two estimates
        base_estimate.max(sub_goal_estimate).max(goal.max_iterations.unwrap_or(10))
    }

    /// Identify required tools
    fn identify_required_tools(goal: &Goal, sub_goals: &[SubGoal]) -> Vec<String> {
        let mut tools = std::collections::HashSet::new();

        // Add tools from sub-goals
        for sub_goal in sub_goals {
            for tool in &sub_goal.required_tools {
                tools.insert(tool.clone());
            }
        }

        // Add default tools based on goal type
        tools.extend(Self::get_default_tools_for_type(&goal.goal_type));

        tools.into_iter().collect()
    }

    /// Get default tools for a goal type
    fn get_default_tools_for_type(goal_type: &GoalType) -> Vec<String> {
        match goal_type {
            GoalType::CodeGeneration => vec![
                "read_file".to_string(),
                "write_file".to_string(),
                "run_command".to_string(),
                "analyze_code".to_string(),
            ],
            GoalType::CodeReview => vec!["read_file".to_string(), "analyze_code".to_string()],
            GoalType::Testing => vec!["run_command".to_string(), "read_file".to_string()],
            GoalType::Debugging => vec![
                "read_file".to_string(),
                "write_file".to_string(),
                "run_command".to_string(),
                "analyze_code".to_string(),
            ],
            GoalType::Documentation => vec!["read_file".to_string(), "write_file".to_string()],
            GoalType::Analysis => vec!["read_file".to_string(), "analyze_code".to_string()],
            GoalType::FileOperation => vec!["read_file".to_string(), "write_file".to_string()],
            GoalType::Research => vec!["read_file".to_string(), "search".to_string()],
            _ => vec![],
        }
    }

    /// Identify required capabilities
    fn identify_required_capabilities(goal: &Goal) -> Vec<String> {
        let mut capabilities = Vec::new();

        match goal.goal_type {
            GoalType::CodeGeneration => {
                capabilities.push("code_generation".to_string());
                capabilities.push("file_operations".to_string());
            }
            GoalType::Testing => {
                capabilities.push("test_execution".to_string());
                capabilities.push("result_analysis".to_string());
            }
            GoalType::Debugging => {
                capabilities.push("error_analysis".to_string());
                capabilities.push("code_modification".to_string());
            }
            GoalType::Research => {
                capabilities.push("information_gathering".to_string());
                capabilities.push("analysis".to_string());
            }
            _ => {
                capabilities.push("general_reasoning".to_string());
            }
        }

        capabilities
    }

    /// Create execution plan with milestones
    fn create_execution_plan(sub_goals: &[SubGoal]) -> Result<ExecutionPlan> {
        let mut milestones = Vec::new();
        let mut completed_sub_goals = std::collections::HashSet::new();
        let mut time_elapsed = 0u64;

        // Group sub-goals by their dependency level
        let mut levels = Vec::new();
        let mut remaining = sub_goals.iter().enumerate().collect::<Vec<_>>();

        while !remaining.is_empty() {
            let mut current_level = Vec::new();
            remaining.retain(|(idx, sg)| {
                if sg.dependencies.iter().all(|dep| completed_sub_goals.contains(dep)) {
                    current_level.push(*idx);
                    false
                } else {
                    true
                }
            });

            if current_level.is_empty() {
                // Circular dependency or broken dependency chain
                break;
            }

            levels.push(current_level);
            for idx in &levels[levels.len() - 1] {
                completed_sub_goals.insert(*idx);
            }
        }

        // Create milestones for each level
        for (level_idx, level) in levels.iter().enumerate() {
            let level_sub_goals: Vec<_> = level.iter().map(|idx| sub_goals[*idx].clone()).collect();
            let level_name = if level_idx == 0 {
                "Initialization"
            } else if level_idx == levels.len() - 1 {
                "Finalization"
            } else {
                "Progress"
            };

            let estimated_time: u64 = level_sub_goals
                .iter()
                .map(|sg| sg.estimated_iterations as u64 * 30) // 30 seconds per iteration
                .sum();

            milestones.push(Milestone {
                name: format!("{} Phase", level_name),
                description: format!(
                    "Complete {} sub-goal(s)",
                    level_sub_goals.len()
                ),
                sub_goal_indices: level.clone(),
                estimated_time_secs: time_elapsed + estimated_time,
            });

            time_elapsed += estimated_time;
        }

        // Create final milestone
        milestones.push(Milestone {
            name: "Completion".to_string(),
            description: "All sub-goals completed".to_string(),
            sub_goal_indices: (0..sub_goals.len()).collect(),
            estimated_time_secs: time_elapsed,
        });

        Ok(ExecutionPlan {
            milestones,
            estimated_duration_secs: time_elapsed,
        })
    }

    /// Format analysis for display
    pub fn format_analysis(analysis: &GoalAnalysis) -> String {
        let mut output = String::new();

        output.push_str(&format!("🎯 Goal Analysis\n"));
        output.push_str(&format!("════════════════════════════════════════\n\n"));
        output.push_str(&format!("Goal: {}\n", analysis.goal.description));
        output.push_str(&format!("Type: {:?}\n", analysis.goal.goal_type));
        output.push_str(&format!("Complexity: {:?}\n", analysis.estimated_complexity));
        output.push_str(&format!("Estimated Iterations: {}\n", analysis.estimated_iterations));
        output.push_str(&format!("Estimated Duration: {} seconds\n\n", analysis.execution_plan.estimated_duration_secs));

        output.push_str(&format!("📋 Sub-Goals ({}):\n", analysis.sub_goals.len()));
        for (idx, sub_goal) in analysis.sub_goals.iter().enumerate() {
            output.push_str(&format!("  {}. {}\n", idx + 1, sub_goal.description));
            output.push_str(&format!("     Type: {:?}\n", sub_goal.goal_type));
            output.push_str(&format!("     Est. Iterations: {}\n", sub_goal.estimated_iterations));
            if !sub_goal.dependencies.is_empty() {
                output.push_str(&format!("     Dependencies: {:?}\n", sub_goal.dependencies));
            }
            if !sub_goal.required_tools.is_empty() {
                output.push_str(&format!("     Tools: {}\n", sub_goal.required_tools.join(", ")));
            }
            output.push_str("\n");
        }

        output.push_str(&format!("🔧 Required Tools:\n"));
        for tool in &analysis.required_tools {
            output.push_str(&format!("  - {}\n", tool));
        }
        output.push_str("\n");

        output.push_str(&format!("📊 Execution Plan:\n"));
        for (idx, milestone) in analysis.execution_plan.milestones.iter().enumerate() {
            output.push_str(&format!("  Milestone {}: {}\n", idx + 1, milestone.name));
            output.push_str(&format!("    Description: {}\n", milestone.description));
            output.push_str(&format!("    Est. Time: {} seconds\n", milestone.estimated_time_secs));
            output.push_str(&format!("    Sub-goals: {:?}\n", milestone.sub_goal_indices));
            output.push_str("\n");
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_goal_analysis() {
        let goal = Goal::builder(
            "Create a REST API in Rust".to_string(),
            GoalType::CodeGeneration,
        )
        .success_criterion("API is functional".to_string())
        .max_iterations(20)
        .build()
        .unwrap();

        let analysis = GoalAnalyzer::analyze_goal(&goal).unwrap();

        assert_eq!(analysis.goal.description, goal.description);
        assert!(!analysis.sub_goals.is_empty());
        assert!(!analysis.required_tools.is_empty());
        assert!(!analysis.execution_plan.milestones.is_empty());
    }

    #[test]
    fn test_goal_decomposition() {
        let goal = Goal::builder(
            "Research AI trends".to_string(),
            GoalType::Research,
        )
        .success_criterion("Research complete".to_string())
        .build()
        .unwrap();

        let sub_goals = GoalAnalyzer::decompose_goal(&goal).unwrap();

        assert!(!sub_goals.is_empty());
        assert!(sub_goals.len() >= 2); // Research should have multiple phases
    }
}

