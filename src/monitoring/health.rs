use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub status: HealthStatus,
    pub message: String,
    pub timestamp: u64,
    pub checks: Vec<ComponentHealth>,
}

/// Component health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub name: String,
    pub status: HealthStatus,
    pub message: String,
}

/// Health checker for monitoring system health
#[allow(dead_code)]
pub struct HealthChecker {
    last_check: Arc<RwLock<Option<Instant>>>,
    check_interval: Duration,
    components: Arc<RwLock<Vec<ComponentHealth>>>,
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new(check_interval: Duration) -> Self {
        Self {
            last_check: Arc::new(RwLock::new(None)),
            check_interval,
            components: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a component health check
    pub async fn register_component(&self, name: String, status: HealthStatus, message: String) {
        let mut components = self.components.write().await;
        if let Some(component) = components.iter_mut().find(|c| c.name == name) {
            component.status = status;
            component.message = message;
        } else {
            components.push(ComponentHealth {
                name,
                status,
                message,
            });
        }
    }

    /// Perform health check
    pub async fn check(&self) -> HealthCheckResult {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let components = self.components.read().await;
        let mut overall_status = HealthStatus::Healthy;
        let mut messages = Vec::new();

        for component in components.iter() {
            match component.status {
                HealthStatus::Unhealthy => {
                    overall_status = HealthStatus::Unhealthy;
                    messages.push(format!("{}: {}", component.name, component.message));
                }
                HealthStatus::Degraded => {
                    if overall_status == HealthStatus::Healthy {
                        overall_status = HealthStatus::Degraded;
                    }
                    messages.push(format!("{}: {}", component.name, component.message));
                }
                HealthStatus::Healthy => {}
            }
        }

        let message = if messages.is_empty() {
            "All systems operational".to_string()
        } else {
            messages.join("; ")
        };

        *self.last_check.write().await = Some(Instant::now());

        HealthCheckResult {
            status: overall_status,
            message,
            timestamp,
            checks: components.clone(),
        }
    }

    /// Get last check time
    pub async fn last_check_time(&self) -> Option<Instant> {
        *self.last_check.read().await
    }
}
