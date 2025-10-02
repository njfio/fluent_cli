//! Collaborative Agent Demo with Working TUI
//!
//! This example demonstrates a fully functional human-in-the-loop agent
//! with real-time TUI updates, approvals, and interactive controls.

use anyhow::Result;
use fluent_agent::{
    agent_control::{
        AgentControlChannel, AgentStatus, ControlMessageType, LogLevel, StateUpdate,
        StateUpdateType,
    },
    ApprovalConfig, CollaborativeOrchestrator,
};
use fluent_cli::tui::SimpleTui;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    println!("🚀 Starting Collaborative Agent Demo");
    println!("This demo shows a working human-in-the-loop agent with TUI");
    println!();

    // Create control channel
    let channel = Arc::new(AgentControlChannel::new());

    // Spawn mock agent that sends realistic updates
    let agent_channel = channel.clone();
    let agent_handle = tokio::spawn(async move {
        run_mock_agent(agent_channel).await
    });

    // Small delay to let agent start
    sleep(Duration::from_millis(100)).await;

    // Create and run TUI
    let tui_channel = channel.clone();
    match SimpleTui::new(Some(tui_channel)) {
        Ok(mut tui) => {
            println!("✅ TUI initialized successfully");
            println!("Press 'P' to pause/resume, 'Q' to quit");

            // Run TUI (this blocks until user quits)
            if let Err(e) = tui.run().await {
                eprintln!("TUI error: {}", e);
            }

            println!("\n👋 TUI closed, shutting down agent...");
        }
        Err(e) => {
            eprintln!("Failed to initialize TUI: {}", e);
            eprintln!("Make sure you're running in a terminal that supports TUI");
        }
    }

    // Clean shutdown
    agent_handle.abort();

    Ok(())
}

/// Mock agent that sends realistic state updates
async fn run_mock_agent(channel: Arc<AgentControlChannel>) -> Result<()> {
    println!("🤖 Mock agent started");

    // Send initialization
    channel
        .send_state(StateUpdate::status_change(AgentStatus::Initializing))
        .await?;

    channel
        .send_state(StateUpdate::log(
            LogLevel::Info,
            "Agent initializing...".to_string(),
        ))
        .await?;

    sleep(Duration::from_secs(2)).await;

    // Agent is now running
    channel
        .send_state(StateUpdate::status_change(AgentStatus::Running))
        .await?;

    channel
        .send_state(StateUpdate::log(
            LogLevel::Info,
            "Agent started successfully!".to_string(),
        ))
        .await?;

    // Simulate agent iterations
    for iteration in 1..=20 {
        // Check for control messages
        if let Ok(Some(msg)) = channel.control_receiver().try_recv().await {
            match msg.message_type {
                ControlMessageType::Pause => {
                    channel
                        .send_state(StateUpdate::status_change(AgentStatus::Paused))
                        .await?;

                    channel
                        .send_state(StateUpdate::log(
                            LogLevel::Info,
                            "Agent paused by human".to_string(),
                        ))
                        .await?;

                    // Wait for resume
                    loop {
                        sleep(Duration::from_millis(100)).await;
                        if let Ok(Some(resume_msg)) = channel.control_receiver().try_recv().await {
                            if matches!(resume_msg.message_type, ControlMessageType::Resume) {
                                channel
                                    .send_state(StateUpdate::status_change(AgentStatus::Running))
                                    .await?;

                                channel
                                    .send_state(StateUpdate::log(
                                        LogLevel::Info,
                                        "Agent resumed".to_string(),
                                    ))
                                    .await?;
                                break;
                            }
                        }
                    }
                }
                ControlMessageType::Input { guidance, .. } => {
                    channel
                        .send_state(StateUpdate::log(
                            LogLevel::Info,
                            format!("Received human guidance: {}", guidance),
                        ))
                        .await?;
                }
                ControlMessageType::ModifyGoal { new_goal, .. } => {
                    channel
                        .send_state(StateUpdate::log(
                            LogLevel::Info,
                            format!("Goal modified to: {}", new_goal),
                        ))
                        .await?;
                }
                ControlMessageType::EmergencyStop { reason } => {
                    channel
                        .send_state(StateUpdate::status_change(
                            AgentStatus::Failed(format!("Emergency stop: {}", reason)),
                        ))
                        .await?;
                    return Ok(());
                }
                _ => {}
            }
        }

        // Send iteration update
        let progress = ((iteration as f32 / 20.0) * 100.0) as u32;
        channel
            .send_state(StateUpdate::iteration_update(iteration, 20, progress))
            .await?;

        // Simulate different types of actions
        let actions = [
            "Analyzing requirements",
            "Planning approach",
            "Generating code",
            "Running tests",
            "Refining implementation",
            "Documenting changes",
        ];

        let action = actions[iteration as usize % actions.len()];
        channel
            .send_state(StateUpdate::action_update(
                action.to_string(),
                "analysis".to_string(),
            ))
            .await?;

        // Send reasoning step
        let confidence = 0.7 + (iteration as f64 * 0.01);
        channel
            .send_state(StateUpdate::new(StateUpdateType::ReasoningStep {
                step_description: format!("Iteration {} reasoning", iteration),
                confidence,
                thought_process: format!(
                    "Analyzing current state and determining next action. Current confidence: {:.1}%",
                    confidence * 100.0
                ),
            }))
            .await?;

        // Simulate occasional warnings
        if iteration % 5 == 0 {
            channel
                .send_state(StateUpdate::log(
                    LogLevel::Warning,
                    format!("Checkpoint {} reached", iteration / 5),
                ))
                .await?;
        }

        // Send goal progress
        channel
            .send_state(StateUpdate::new(StateUpdateType::GoalProgress {
                goal_description: "Complete the demonstration task".to_string(),
                completion_percentage: progress as f64,
                achieved_criteria: vec![
                    "Initialized agent".to_string(),
                    "Started processing".to_string(),
                ],
                remaining_criteria: vec![
                    "Complete all iterations".to_string(),
                    "Generate final report".to_string(),
                ],
            }))
            .await?;

        // Wait between iterations
        sleep(Duration::from_millis(1500)).await;
    }

    // Agent completed
    channel
        .send_state(StateUpdate::status_change(AgentStatus::Completed))
        .await?;

    channel
        .send_state(StateUpdate::log(
            LogLevel::Info,
            "✅ Agent completed successfully!".to_string(),
        ))
        .await?;

    // Keep agent alive
    loop {
        sleep(Duration::from_secs(1)).await;
    }
}
