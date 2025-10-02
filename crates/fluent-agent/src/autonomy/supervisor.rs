use crate::action::{ActionPlan, ActionResult, RiskLevel as ActionRiskLevel};
use crate::monitoring::PerformanceMetrics;
use crate::orchestrator::Observation;
use crate::security::capability::{CapabilityManager, ResourceRequest};
use crate::security::{AuditEvent, AuditEventType, AuditOutcome, AuditSeverity, SecurityFramework};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

/// Supervisor stages for lifecycle governance
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SupervisorStage {
    PreIteration,
    PreAction,
    PostAction,
    PostIteration,
    Incident,
}

/// Risk assessment returned by the supervisor guardrails
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub stage: SupervisorStage,
    pub risk_score: f64,
    pub risk_level: RiskLevel,
    pub confidence: f64,
    pub triggers: Vec<String>,
    pub recommended_action: GuardrailDecision,
}

/// Supervisor decision when guardrails trigger
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GuardrailDecision {
    Allow,
    Review,
    Mitigate,
    Block,
    Escalate,
}

/// Risk level enumeration (lightweight alias to avoid coupling)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Minimal,
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn from_score(score: f64) -> Self {
        match score {
            s if s < 0.2 => RiskLevel::Minimal,
            s if s < 0.4 => RiskLevel::Low,
            s if s < 0.6 => RiskLevel::Medium,
            s if s < 0.8 => RiskLevel::High,
            _ => RiskLevel::Critical,
        }
    }
}

/// Incident generated when supervisor blocks or escalates behaviour
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorIncident {
    pub incident_id: String,
    pub created_at: SystemTime,
    pub stage: SupervisorStage,
    pub reason: String,
    pub risk_assessment: RiskAssessment,
    pub metadata: HashMap<String, String>,
}

/// Configuration for the autonomy supervisor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomySupervisorConfig {
    pub enabled: bool,
    pub max_risk_score: f64,
    pub pre_iteration_threshold: f64,
    pub pre_action_threshold: f64,
    pub post_action_threshold: f64,
    pub incident_escalation_threshold: f64,
    pub allow_tool_whitelist: Vec<String>,
    pub blocked_tool_list: Vec<String>,
    pub risk_decay: f64,
    pub confidence_floor: f64,
    pub policy_name: String,
}

impl Default for AutonomySupervisorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_risk_score: 1.0,
            pre_iteration_threshold: 0.55,
            pre_action_threshold: 0.45,
            post_action_threshold: 0.7,
            incident_escalation_threshold: 0.85,
            allow_tool_whitelist: vec!["filesystem".to_string(), "string_replace".to_string()],
            blocked_tool_list: vec![],
            risk_decay: 0.1,
            confidence_floor: 0.3,
            policy_name: "default".to_string(),
        }
    }
}

/// Autonomy supervisor orchestrating global guardrails
pub struct AutonomySupervisor {
    config: AutonomySupervisorConfig,
    security_framework: Arc<SecurityFramework>,
    capability_manager: Arc<CapabilityManager>,
    active_session: RwLock<Option<String>>,
    rolling_risk: RwLock<f64>,
}

impl AutonomySupervisor {
    pub async fn new(
        config: AutonomySupervisorConfig,
        security_framework: Arc<SecurityFramework>,
        capability_manager: Arc<CapabilityManager>,
    ) -> Result<Self> {
        let supervisor = Self {
            config,
            security_framework,
            capability_manager,
            active_session: RwLock::new(None),
            rolling_risk: RwLock::new(0.0),
        };

        supervisor.initialize_session().await?;
        Ok(supervisor)
    }

    async fn initialize_session(&self) -> Result<()> {
        let mut guard = self.active_session.write().await;
        if guard.is_some() {
            return Ok(());
        }

        let session_id = self
            .capability_manager
            .create_session(
                &self.config.policy_name,
                None,
                HashMap::from([("environment".to_string(), "agentic".to_string())]),
            )
            .await?;
        *guard = Some(session_id);
        Ok(())
    }

    pub fn config(&self) -> &AutonomySupervisorConfig {
        &self.config
    }

    pub async fn assess_iteration(
        &self,
        iteration: u32,
        metrics: &PerformanceMetrics,
    ) -> Result<RiskAssessment> {
        let mut score = 0.0;
        let mut triggers = Vec::new();

        if metrics.execution_metrics.queue_length > 10 {
            score += 0.15;
            triggers.push("queue_length>10".to_string());
        }
        if metrics.execution_metrics.tasks_failed > 0 {
            let fail_factor = (metrics.execution_metrics.tasks_failed as f64).min(5.0) * 0.05;
            score += fail_factor;
            triggers.push(format!(
                "failed_tasks:{}",
                metrics.execution_metrics.tasks_failed
            ));
        }
        if iteration > 0 {
            score += (iteration as f64 * 0.005).min(0.1);
        }

        self.apply_decay(&mut score).await;
        self.clamp_score(&mut score);

        let risk_level = RiskLevel::from_score(score);
        let decision = self.select_decision(score, self.config.pre_iteration_threshold);

        Ok(RiskAssessment {
            stage: SupervisorStage::PreIteration,
            risk_score: score,
            risk_level,
            confidence: 1.0,
            triggers,
            recommended_action: decision,
        })
    }

    pub async fn assess_action(
        &self,
        confidence_score: f64,
        action_plan: &ActionPlan,
        context_summary: &str,
    ) -> Result<RiskAssessment> {
        let mut score = match action_plan.risk_level {
            ActionRiskLevel::Low => 0.1,
            ActionRiskLevel::Medium => 0.4,
            ActionRiskLevel::High => 0.7,
            ActionRiskLevel::Critical => 0.85,
        };

        let mut triggers = vec![format!("plan_risk:{:?}", action_plan.risk_level)];

        if !self
            .config
            .allow_tool_whitelist
            .contains(&action_plan.action_type.to_string())
        {
            score += 0.2;
            triggers.push("non_whitelisted_tool".to_string());
        }

        if self
            .config
            .blocked_tool_list
            .iter()
            .any(|blocked| action_plan.description.contains(blocked))
        {
            score += 0.3;
            triggers.push("blocked_tool_detected".to_string());
        }

        if context_summary.contains("critical") {
            score += 0.15;
            triggers.push("context_critical".to_string());
        }

        self.apply_decay(&mut score).await;
        self.clamp_score(&mut score);

        let risk_level = RiskLevel::from_score(score);
        let decision = self.select_decision(score, self.config.pre_action_threshold);

        Ok(RiskAssessment {
            stage: SupervisorStage::PreAction,
            risk_score: score,
            risk_level,
            confidence: confidence_score,
            triggers,
            recommended_action: decision,
        })
    }

    pub async fn assess_post_action(
        &self,
        action_result: &ActionResult,
        observation: &Observation,
    ) -> Result<RiskAssessment> {
        let mut score = 0.0;
        let mut triggers = Vec::new();

        if !action_result.success {
            score += 0.35;
            triggers.push("action_failed".to_string());
        }

        if observation.content.to_lowercase().contains("error") {
            score += 0.25;
            triggers.push("observation_error".to_string());
        }

        if let Some(stderr) = action_result.metadata.get("stderr") {
            if stderr.as_str().unwrap_or_default().len() > 0 {
                score += 0.1;
                triggers.push("stderr_present".to_string());
            }
        }

        self.apply_decay(&mut score).await;
        self.clamp_score(&mut score);

        let risk_level = RiskLevel::from_score(score);
        let decision = self.select_decision(score, self.config.post_action_threshold);

        Ok(RiskAssessment {
            stage: SupervisorStage::PostAction,
            risk_score: score,
            risk_level,
            confidence: action_result.success.then(|| 0.8).unwrap_or(0.3),
            triggers,
            recommended_action: decision,
        })
    }

    pub async fn evaluate_incident(
        &self,
        stage: SupervisorStage,
        reason: &str,
        assessment: &RiskAssessment,
    ) -> Result<Option<SupervisorIncident>> {
        if assessment.risk_score >= self.config.incident_escalation_threshold
            || matches!(assessment.recommended_action, GuardrailDecision::Block)
        {
            let incident = SupervisorIncident {
                incident_id: uuid::Uuid::new_v4().to_string(),
                created_at: SystemTime::now(),
                stage,
                reason: reason.to_string(),
                risk_assessment: assessment.clone(),
                metadata: HashMap::new(),
            };
            self.emit_audit_event(&incident).await?;
            return Ok(Some(incident));
        }
        Ok(None)
    }

    pub async fn register_resource_usage(&self, resource: ResourceRequest) -> Result<()> {
        if let Some(session_id) = self.active_session.read().await.as_ref() {
            match self
                .capability_manager
                .check_permission(session_id, &resource)
                .await?
            {
                crate::security::capability::PermissionResult::Granted => Ok(()),
                crate::security::capability::PermissionResult::Conditional { conditions } => Err(
                    anyhow!(format!("Capability conditions unmet: {:?}", conditions)),
                ),
                crate::security::capability::PermissionResult::Denied { reason } => {
                    Err(anyhow!(format!("Resource request denied: {}", reason)))
                }
            }
        } else {
            Err(anyhow!("Supervisor has no active security session"))
        }
    }

    async fn apply_decay(&self, score: &mut f64) {
        let mut rolling = self.rolling_risk.write().await;
        let decayed = (*rolling * (1.0 - self.config.risk_decay)) + *score;
        *score = decayed;
        *rolling = decayed;
    }

    fn clamp_score(&self, score: &mut f64) {
        if *score > self.config.max_risk_score {
            *score = self.config.max_risk_score;
        }
        if *score < 0.0 {
            *score = 0.0;
        }
    }

    fn select_decision(&self, score: f64, threshold: f64) -> GuardrailDecision {
        if score >= self.config.incident_escalation_threshold {
            GuardrailDecision::Escalate
        } else if score >= threshold + 0.15 {
            GuardrailDecision::Block
        } else if score >= threshold {
            GuardrailDecision::Mitigate
        } else if score >= threshold * 0.75 {
            GuardrailDecision::Review
        } else {
            GuardrailDecision::Allow
        }
    }

    async fn emit_audit_event(&self, incident: &SupervisorIncident) -> Result<()> {
        let event = AuditEvent {
            event_id: incident.incident_id.clone(),
            timestamp: incident.created_at,
            event_type: AuditEventType::SecurityViolation,
            severity: match incident.risk_assessment.risk_level {
                RiskLevel::Minimal | RiskLevel::Low => AuditSeverity::Info,
                RiskLevel::Medium => AuditSeverity::Warning,
                RiskLevel::High => AuditSeverity::Error,
                RiskLevel::Critical => AuditSeverity::Critical,
            },
            user_id: Some("autonomy_supervisor".to_string()),
            resource: Some(format!("stage::{:?}", incident.stage)),
            action: format!("incident:{}", incident.reason),
            outcome: AuditOutcome::Blocked,
            details: incident.metadata.clone(),
            source_ip: None,
            user_agent: None,
        };

        self.security_framework.log_security_event(event).await
    }
}
