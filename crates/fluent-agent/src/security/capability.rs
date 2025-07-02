use super::{
    SecurityPolicy, SecuritySession, SecurityError, Capability, ResourceType, Permission,
    Constraint, Condition, ResourceUsage
};
use anyhow::Result;
use chrono::Datelike;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Resource request for capability checking
#[derive(Debug, Clone)]
pub struct ResourceRequest {
    pub resource_type: ResourceType,
    pub operation: Permission,
    pub target: String,
    pub size: Option<u64>,
    pub metadata: HashMap<String, String>,
}

/// Permission check result
#[derive(Debug, Clone)]
pub enum PermissionResult {
    Granted,
    Denied { reason: String },
    Conditional { conditions: Vec<String> },
}

/// Capability manager for enforcing security policies
pub struct CapabilityManager {
    policies: Arc<RwLock<HashMap<String, SecurityPolicy>>>,
    active_sessions: Arc<RwLock<HashMap<String, SecuritySession>>>,
    rate_limiters: Arc<RwLock<HashMap<String, RateLimiter>>>,
}

impl CapabilityManager {
    pub fn new() -> Self {
        Self {
            policies: Arc::new(RwLock::new(HashMap::new())),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Load a security policy
    pub async fn load_policy(&self, policy: SecurityPolicy) -> Result<()> {
        let mut policies = self.policies.write().await;
        policies.insert(policy.name.clone(), policy);
        Ok(())
    }
    
    /// Create a new security session
    pub async fn create_session(
        &self,
        policy_name: &str,
        user_id: Option<String>,
        metadata: HashMap<String, String>,
    ) -> Result<String> {
        let policies = self.policies.read().await;
        let policy = policies.get(policy_name)
            .ok_or_else(|| SecurityError::SessionNotFound(policy_name.to_string()))?;
        
        let session_id = Uuid::new_v4().to_string();
        let session = SecuritySession {
            session_id: session_id.clone(),
            policy_name: policy_name.to_string(),
            user_id,
            granted_capabilities: policy.capabilities.clone(),
            resource_usage: ResourceUsage::default(),
            created_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            metadata,
        };
        
        let mut sessions = self.active_sessions.write().await;
        sessions.insert(session_id.clone(), session);
        
        Ok(session_id)
    }
    
    /// Check permission for a resource request
    pub async fn check_permission(
        &self,
        session_id: &str,
        resource: &ResourceRequest,
    ) -> Result<PermissionResult> {
        let mut sessions = self.active_sessions.write().await;
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SecurityError::SessionNotFound(session_id.to_string()))?;
        
        // Update last activity
        session.last_activity = chrono::Utc::now();
        
        // Find matching capability
        let capability = session.granted_capabilities.iter()
            .find(|cap| self.matches_resource(&cap.resource_type, resource))
            .ok_or_else(|| SecurityError::CapabilityNotGranted(format!("No capability for {:?}", resource.resource_type)))?;
        
        // Check if operation is permitted
        if !capability.permissions.contains(&resource.operation) {
            return Ok(PermissionResult::Denied {
                reason: format!("Operation {:?} not permitted", resource.operation),
            });
        }
        
        // Check constraints
        self.validate_constraints(capability, resource, session).await?;
        
        // Check conditions
        if let Some(ref conditions) = capability.conditions {
            let unmet_conditions = self.check_conditions(conditions, session).await?;
            if !unmet_conditions.is_empty() {
                return Ok(PermissionResult::Conditional {
                    conditions: unmet_conditions,
                });
            }
        }
        
        // Update resource usage
        self.update_resource_usage(session, resource);
        
        Ok(PermissionResult::Granted)
    }
    
    /// Check if a capability matches a resource request
    fn matches_resource(&self, capability_resource: &ResourceType, request: &ResourceRequest) -> bool {
        match (capability_resource, &request.resource_type) {
            (ResourceType::FileSystem { paths, .. }, ResourceType::FileSystem { .. }) => {
                paths.iter().any(|path| request.target.starts_with(path))
            }
            (ResourceType::Network { hosts, ports, .. }, ResourceType::Network { .. }) => {
                // Check if request matches allowed hosts and ports
                hosts.iter().any(|host| request.target.contains(host)) ||
                ports.iter().any(|port| request.target.contains(&port.to_string()))
            }
            (ResourceType::Process { commands, .. }, ResourceType::Process { .. }) => {
                commands.iter().any(|cmd| request.target.starts_with(cmd))
            }
            _ => std::mem::discriminant(capability_resource) == std::mem::discriminant(&request.resource_type),
        }
    }
    
    /// Validate constraints for a capability
    async fn validate_constraints(
        &self,
        capability: &Capability,
        resource: &ResourceRequest,
        session: &SecuritySession,
    ) -> Result<()> {
        for constraint in &capability.constraints {
            match constraint {
                Constraint::MaxFileSize(max_size) => {
                    if let Some(size) = resource.size {
                        if size > *max_size {
                            return Err(SecurityError::ConstraintViolation(
                                format!("File size {} exceeds maximum {}", size, max_size)
                            ).into());
                        }
                    }
                }
                Constraint::MaxMemoryUsage(max_memory) => {
                    if session.resource_usage.memory_used > *max_memory {
                        return Err(SecurityError::ConstraintViolation(
                            format!("Memory usage {} exceeds maximum {}", session.resource_usage.memory_used, max_memory)
                        ).into());
                    }
                }
                Constraint::MaxExecutionTime(max_time) => {
                    let elapsed = chrono::Utc::now().signed_duration_since(session.created_at);
                    if elapsed.to_std().unwrap_or_default() > *max_time {
                        return Err(SecurityError::ConstraintViolation(
                            "Execution time limit exceeded".to_string()
                        ).into());
                    }
                }
                Constraint::RateLimit { max_requests, window } => {
                    self.check_rate_limit(session, max_requests, window).await?;
                }
                Constraint::TimeWindow { start, end } => {
                    let now = chrono::Utc::now().time();
                    if now < *start || now > *end {
                        return Err(SecurityError::ConstraintViolation(
                            "Outside allowed time window".to_string()
                        ).into());
                    }
                }
                Constraint::IpWhitelist(allowed_ips) => {
                    if let Some(client_ip) = resource.metadata.get("client_ip") {
                        if !allowed_ips.contains(client_ip) {
                            return Err(SecurityError::ConstraintViolation(
                                format!("IP {} not in whitelist", client_ip)
                            ).into());
                        }
                    }
                }
                _ => {
                    // Other constraints can be implemented as needed
                }
            }
        }
        Ok(())
    }
    
    /// Check conditions for capability activation
    async fn check_conditions(
        &self,
        conditions: &[Condition],
        session: &SecuritySession,
    ) -> Result<Vec<String>> {
        let mut unmet_conditions = Vec::new();
        
        for condition in conditions {
            match condition {
                Condition::UserRole(required_role) => {
                    if let Some(user_role) = session.metadata.get("role") {
                        if user_role != required_role {
                            unmet_conditions.push(format!("User role {} required", required_role));
                        }
                    } else {
                        unmet_conditions.push(format!("User role {} required", required_role));
                    }
                }
                Condition::TimeOfDay { start, end } => {
                    let now = chrono::Utc::now().time();
                    if now < *start || now > *end {
                        unmet_conditions.push(format!("Time must be between {} and {}", start, end));
                    }
                }
                Condition::DayOfWeek(allowed_days) => {
                    let today = chrono::Utc::now().weekday().num_days_from_sunday() as u8;
                    if !allowed_days.contains(&today) {
                        unmet_conditions.push("Not allowed on this day of week".to_string());
                    }
                }
                Condition::IpAddress(required_ip) => {
                    if let Some(client_ip) = session.metadata.get("client_ip") {
                        if client_ip != required_ip {
                            unmet_conditions.push(format!("IP address {} required", required_ip));
                        }
                    } else {
                        unmet_conditions.push(format!("IP address {} required", required_ip));
                    }
                }
                Condition::Environment(required_env) => {
                    if let Some(env) = session.metadata.get("environment") {
                        if env != required_env {
                            unmet_conditions.push(format!("Environment {} required", required_env));
                        }
                    } else {
                        unmet_conditions.push(format!("Environment {} required", required_env));
                    }
                }
                Condition::Custom { key, value } => {
                    if let Some(actual_value) = session.metadata.get(key) {
                        if actual_value != value {
                            unmet_conditions.push(format!("Custom condition {}={} required", key, value));
                        }
                    } else {
                        unmet_conditions.push(format!("Custom condition {}={} required", key, value));
                    }
                }
            }
        }
        
        Ok(unmet_conditions)
    }
    
    /// Check rate limiting
    async fn check_rate_limit(
        &self,
        session: &SecuritySession,
        max_requests: &u32,
        window: &Duration,
    ) -> Result<()> {
        let mut rate_limiters = self.rate_limiters.write().await;
        let limiter = rate_limiters
            .entry(session.session_id.clone())
            .or_insert_with(|| RateLimiter::new(*max_requests, *window));
        
        if !limiter.allow_request() {
            return Err(SecurityError::ConstraintViolation(
                "Rate limit exceeded".to_string()
            ).into());
        }
        
        Ok(())
    }
    
    /// Update resource usage for a session
    fn update_resource_usage(&self, session: &mut SecuritySession, resource: &ResourceRequest) {
        match &resource.resource_type {
            ResourceType::FileSystem { .. } => {
                session.resource_usage.files_accessed += 1;
                if let Some(size) = resource.size {
                    session.resource_usage.disk_space_used += size;
                }
            }
            ResourceType::Memory { max_bytes, .. } => {
                if let Some(size) = resource.size {
                    session.resource_usage.memory_used += size.min(*max_bytes);
                }
            }
            ResourceType::Process { .. } => {
                session.resource_usage.processes_spawned += 1;
            }
            ResourceType::Network { .. } => {
                if let Some(size) = resource.size {
                    session.resource_usage.network_bytes_sent += size;
                }
            }
            _ => {}
        }
    }
    
    /// Get session information
    pub async fn get_session(&self, session_id: &str) -> Option<SecuritySession> {
        let sessions = self.active_sessions.read().await;
        sessions.get(session_id).cloned()
    }
    
    /// Remove a session
    pub async fn remove_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.active_sessions.write().await;
        sessions.remove(session_id);
        
        let mut rate_limiters = self.rate_limiters.write().await;
        rate_limiters.remove(session_id);
        
        Ok(())
    }
    
    /// Get all active sessions
    pub async fn get_active_sessions(&self) -> Vec<SecuritySession> {
        let sessions = self.active_sessions.read().await;
        sessions.values().cloned().collect()
    }
}

impl Default for CapabilityManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Rate limiter for enforcing request rate limits
struct RateLimiter {
    max_requests: u32,
    window: Duration,
    requests: Vec<Instant>,
}

impl RateLimiter {
    fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            requests: Vec::new(),
        }
    }
    
    fn allow_request(&mut self) -> bool {
        let now = Instant::now();
        
        // Remove old requests outside the window
        self.requests.retain(|&time| now.duration_since(time) < self.window);
        
        // Check if we can allow this request
        if self.requests.len() < self.max_requests as usize {
            self.requests.push(now);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_capability_manager_creation() {
        let manager = CapabilityManager::new();
        let sessions = manager.get_active_sessions().await;
        assert!(sessions.is_empty());
    }
    
    #[tokio::test]
    async fn test_session_creation() {
        let manager = CapabilityManager::new();
        let policy = SecurityPolicy::default();
        
        manager.load_policy(policy).await.unwrap();
        
        let session_id = manager.create_session(
            "default",
            Some("test_user".to_string()),
            HashMap::new(),
        ).await.unwrap();
        
        assert!(!session_id.is_empty());
        
        let session = manager.get_session(&session_id).await;
        assert!(session.is_some());
        assert_eq!(session.unwrap().user_id, Some("test_user".to_string()));
    }
    
    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(2, Duration::from_secs(1));
        
        assert!(limiter.allow_request());
        assert!(limiter.allow_request());
        assert!(!limiter.allow_request()); // Should be rate limited
    }
}
