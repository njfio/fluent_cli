use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use std::env;

pub mod capability;

/// Security policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub capabilities: Vec<Capability>,
    pub restrictions: SecurityRestrictions,
    pub audit_config: AuditConfig,
    pub sandbox_config: SandboxConfig,
}

/// Capability definition for fine-grained access control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub name: String,
    pub resource_type: ResourceType,
    pub permissions: Vec<Permission>,
    pub constraints: Vec<Constraint>,
    pub conditions: Option<Vec<Condition>>,
}

/// Resource types that can be accessed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceType {
    FileSystem {
        paths: Vec<String>,
        allowed_extensions: Option<Vec<String>>,
    },
    Network {
        hosts: Vec<String>,
        ports: Vec<u16>,
        protocols: Vec<String>,
    },
    Process {
        commands: Vec<String>,
        allowed_args: Option<Vec<String>>,
    },
    Environment {
        variables: Vec<String>,
        read_only: bool,
    },
    Memory {
        max_bytes: u64,
        shared_access: bool,
    },
    Time {
        max_duration_seconds: u64,
        time_windows: Option<Vec<TimeWindow>>,
    },
    Database {
        connections: Vec<String>,
        operations: Vec<String>,
    },
    Api {
        endpoints: Vec<String>,
        methods: Vec<String>,
    },
}

/// Time window for time-based access control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWindow {
    pub start_hour: u8,
    pub end_hour: u8,
    pub days_of_week: Vec<u8>, // 0 = Sunday, 1 = Monday, etc.
}

/// Permissions that can be granted
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Permission {
    Read,
    Write,
    Execute,
    Create,
    Delete,
    Modify,
    List,
    Connect,
    Bind,
    Listen,
}

/// Constraints on capability usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Constraint {
    MaxFileSize(u64),
    MaxMemoryUsage(u64),
    MaxExecutionTime(Duration),
    RateLimit {
        max_requests: u32,
        window: Duration,
    },
    TimeWindow {
        start: chrono::NaiveTime,
        end: chrono::NaiveTime,
    },
    IpWhitelist(Vec<String>),
    UserAgent(String),
    Referrer(String),
}

/// Conditions for capability activation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    UserRole(String),
    TimeOfDay {
        start: chrono::NaiveTime,
        end: chrono::NaiveTime,
    },
    DayOfWeek(Vec<u8>),
    IpAddress(String),
    Environment(String),
    Custom {
        key: String,
        value: String,
    },
}

/// Security restrictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRestrictions {
    pub max_file_size: u64,
    pub max_memory_usage: u64,
    pub max_execution_time: Duration,
    pub allowed_file_extensions: HashSet<String>,
    pub blocked_commands: HashSet<String>,
    pub network_restrictions: NetworkRestrictions,
    pub process_restrictions: ProcessRestrictions,
}

/// Network access restrictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRestrictions {
    pub allow_outbound: bool,
    pub allow_inbound: bool,
    pub allowed_domains: Vec<String>,
    pub blocked_ips: Vec<String>,
    pub allowed_ports: Option<Vec<u16>>,
    pub blocked_ports: Vec<u16>,
    pub require_tls: bool,
}

/// Process execution restrictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRestrictions {
    pub allowed_commands: Option<Vec<String>>,
    pub blocked_commands: Vec<String>,
    pub max_processes: u32,
    pub max_memory_per_process: u64,
    pub max_cpu_percent: f64,
    pub allow_shell_access: bool,
}

/// Audit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    pub enabled: bool,
    pub log_level: AuditLogLevel,
    pub log_destinations: Vec<AuditDestination>,
    pub retention_days: u32,
    pub encryption_enabled: bool,
    pub real_time_alerts: bool,
    pub alert_thresholds: AlertThresholds,
}

/// Audit log levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditLogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Audit log destinations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditDestination {
    File {
        path: String,
    },
    Database {
        connection_string: String,
    },
    Syslog {
        server: String,
        port: u16,
    },
    Http {
        endpoint: String,
        headers: HashMap<String, String>,
    },
}

/// Alert thresholds for security monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub failed_attempts_per_minute: u32,
    pub suspicious_activity_score: u32,
    pub resource_usage_percent: f64,
    pub error_rate_percent: f64,
}

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub enabled: bool,
    pub sandbox_type: SandboxType,
    pub resource_limits: ResourceLimits,
    pub isolation_level: IsolationLevel,
    pub allowed_syscalls: Option<Vec<String>>,
    pub blocked_syscalls: Vec<String>,
    pub mount_points: Vec<MountPoint>,
}

/// Types of sandboxing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SandboxType {
    Process,
    Container,
    Vm,
    Wasm,
}

/// Resource limits for sandboxed execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_memory: u64,
    pub max_cpu_percent: f64,
    pub max_disk_space: u64,
    pub max_network_bandwidth: u64,
    pub max_file_descriptors: u32,
    pub max_processes: u32,
}

/// Isolation levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IsolationLevel {
    None,
    Process,
    User,
    Network,
    Full,
}

/// Mount point configuration for sandboxes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountPoint {
    pub source: String,
    pub target: String,
    pub read_only: bool,
    pub mount_type: String,
}

/// Security session for tracking active security contexts
#[derive(Debug, Clone)]
pub struct SecuritySession {
    pub session_id: String,
    pub policy_name: String,
    pub user_id: Option<String>,
    pub granted_capabilities: Vec<Capability>,
    pub resource_usage: ResourceUsage,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, String>,
}

/// Resource usage tracking
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub memory_used: u64,
    pub cpu_time_used: Duration,
    pub disk_space_used: u64,
    pub network_bytes_sent: u64,
    pub network_bytes_received: u64,
    pub files_accessed: u32,
    pub processes_spawned: u32,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            memory_used: 0,
            cpu_time_used: Duration::ZERO,
            disk_space_used: 0,
            network_bytes_sent: 0,
            network_bytes_received: 0,
            files_accessed: 0,
            processes_spawned: 0,
        }
    }
}

/// Security error types
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Capability not granted: {0}")]
    CapabilityNotGranted(String),

    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),

    #[error("Access denied: {0}")]
    AccessDenied(String),

    #[error("Sandbox violation: {0}")]
    SandboxViolation(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Audit error: {0}")]
    AuditError(String),
}

/// Default security policy for development
impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            version: "1.0".to_string(),
            description: Some("Default security policy for development".to_string()),
            capabilities: vec![Capability {
                name: "file_read".to_string(),
                resource_type: ResourceType::FileSystem {
                    paths: vec!["/tmp".to_string(), "./".to_string()],
                    allowed_extensions: Some(vec!["txt".to_string(), "json".to_string()]),
                },
                permissions: vec![Permission::Read, Permission::List],
                constraints: vec![Constraint::MaxFileSize(10 * 1024 * 1024)], // 10MB
                conditions: None,
            }],
            restrictions: SecurityRestrictions {
                max_file_size: 100 * 1024 * 1024,             // 100MB
                max_memory_usage: 1024 * 1024 * 1024,         // 1GB
                max_execution_time: Duration::from_secs(
                    env::var("FLUENT_SECURITY_MAX_EXECUTION_TIME_SECONDS")
                        .ok()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(300)
                ), // Default: 5 minutes, configurable via FLUENT_SECURITY_MAX_EXECUTION_TIME_SECONDS
                allowed_file_extensions: ["txt", "json", "yaml", "md"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                blocked_commands: ["rm", "sudo", "chmod", "chown"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                network_restrictions: NetworkRestrictions {
                    allow_outbound: true,
                    allow_inbound: false,
                    allowed_domains: vec![
                        "api.openai.com".to_string(),
                        "api.anthropic.com".to_string(),
                    ],
                    blocked_ips: vec!["127.0.0.1".to_string()],
                    allowed_ports: Some(vec![80, 443]),
                    blocked_ports: vec![22, 23, 3389],
                    require_tls: true,
                },
                process_restrictions: ProcessRestrictions {
                    allowed_commands: None,
                    blocked_commands: vec!["rm".to_string(), "sudo".to_string()],
                    max_processes: 10,
                    max_memory_per_process: 512 * 1024 * 1024, // 512MB
                    max_cpu_percent: 50.0,
                    allow_shell_access: false,
                },
            },
            audit_config: AuditConfig {
                enabled: true,
                log_level: AuditLogLevel::Info,
                log_destinations: vec![AuditDestination::File {
                    path: "audit.log".to_string(),
                }],
                retention_days: 30,
                encryption_enabled: false,
                real_time_alerts: false,
                alert_thresholds: AlertThresholds {
                    failed_attempts_per_minute: 10,
                    suspicious_activity_score: 80,
                    resource_usage_percent: 90.0,
                    error_rate_percent: 10.0,
                },
            },
            sandbox_config: SandboxConfig {
                enabled: true,
                sandbox_type: SandboxType::Process,
                resource_limits: ResourceLimits {
                    max_memory: 512 * 1024 * 1024, // 512MB
                    max_cpu_percent: 25.0,
                    max_disk_space: 100 * 1024 * 1024, // 100MB
                    max_network_bandwidth: 10 * 1024 * 1024, // 10MB/s
                    max_file_descriptors: 100,
                    max_processes: 5,
                },
                isolation_level: IsolationLevel::Process,
                allowed_syscalls: None,
                blocked_syscalls: vec!["execve".to_string(), "fork".to_string()],
                mount_points: vec![],
            },
        }
    }
}

impl SecurityPolicy {
    /// Load security configuration from environment variables
    /// This allows runtime configuration of security settings for production deployments
    pub fn from_environment() -> Self {
        let mut config = Self::default();

        // Sandbox configuration from environment
        if let Ok(enabled) = env::var("FLUENT_SECURITY_SANDBOX_ENABLED") {
            config.sandbox_config.enabled = enabled.parse().unwrap_or(true);
        }

        if let Ok(max_memory) = env::var("FLUENT_SECURITY_MAX_MEMORY") {
            if let Ok(memory_bytes) = max_memory.parse::<u64>() {
                config.sandbox_config.resource_limits.max_memory = memory_bytes;
            }
        }

        if let Ok(max_cpu) = env::var("FLUENT_SECURITY_MAX_CPU_PERCENT") {
            if let Ok(cpu_percent) = max_cpu.parse::<f64>() {
                config.sandbox_config.resource_limits.max_cpu_percent = cpu_percent;
            }
        }

        if let Ok(max_disk) = env::var("FLUENT_SECURITY_MAX_DISK_SPACE") {
            if let Ok(disk_bytes) = max_disk.parse::<u64>() {
                config.sandbox_config.resource_limits.max_disk_space = disk_bytes;
            }
        }

        if let Ok(max_processes) = env::var("FLUENT_SECURITY_MAX_PROCESSES") {
            if let Ok(processes) = max_processes.parse::<u32>() {
                config.sandbox_config.resource_limits.max_processes = processes;
            }
        }

        // Audit configuration from environment
        if let Ok(audit_enabled) = env::var("FLUENT_SECURITY_AUDIT_ENABLED") {
            config.audit_config.enabled = audit_enabled.parse().unwrap_or(true);
        }

        if let Ok(retention_days) = env::var("FLUENT_SECURITY_AUDIT_RETENTION_DAYS") {
            if let Ok(days) = retention_days.parse::<u32>() {
                config.audit_config.retention_days = days;
            }
        }

        if let Ok(encryption_enabled) = env::var("FLUENT_SECURITY_AUDIT_ENCRYPTION") {
            config.audit_config.encryption_enabled = encryption_enabled.parse().unwrap_or(false);
        }

        if let Ok(log_file) = env::var("FLUENT_SECURITY_AUDIT_LOG_FILE") {
            config.audit_config.log_destinations = vec![AuditDestination::File { path: log_file }];
        }

        // Alert thresholds from environment
        if let Ok(failed_attempts) = env::var("FLUENT_SECURITY_ALERT_FAILED_ATTEMPTS") {
            if let Ok(attempts) = failed_attempts.parse::<u32>() {
                config.audit_config.alert_thresholds.failed_attempts_per_minute = attempts;
            }
        }

        if let Ok(suspicious_score) = env::var("FLUENT_SECURITY_ALERT_SUSPICIOUS_SCORE") {
            if let Ok(score) = suspicious_score.parse::<u32>() {
                config.audit_config.alert_thresholds.suspicious_activity_score = score;
            }
        }

        // Blocked syscalls from environment (comma-separated)
        if let Ok(blocked_syscalls) = env::var("FLUENT_SECURITY_BLOCKED_SYSCALLS") {
            config.sandbox_config.blocked_syscalls = blocked_syscalls
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_policy_default() {
        let policy = SecurityPolicy::default();
        assert_eq!(policy.name, "default");
        assert_eq!(policy.version, "1.0");
        assert!(!policy.capabilities.is_empty());
        assert!(policy.audit_config.enabled);
        assert!(policy.sandbox_config.enabled);
    }

    #[test]
    fn test_resource_usage_default() {
        let usage = ResourceUsage::default();
        assert_eq!(usage.memory_used, 0);
        assert_eq!(usage.cpu_time_used, Duration::ZERO);
        assert_eq!(usage.files_accessed, 0);
    }

    #[test]
    fn test_security_session_creation() {
        let session = SecuritySession {
            session_id: "test_session".to_string(),
            policy_name: "test_policy".to_string(),
            user_id: Some("test_user".to_string()),
            granted_capabilities: vec![],
            resource_usage: ResourceUsage::default(),
            created_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        assert_eq!(session.session_id, "test_session");
        assert_eq!(session.policy_name, "test_policy");
        assert_eq!(session.user_id, Some("test_user".to_string()));
    }
}
