use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Alert level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

/// An alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub level: AlertLevel,
    pub message: String,
    pub component: String,
    pub timestamp: u64,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Alert manager for managing alerts
pub struct AlertManager {
    alerts: Arc<RwLock<VecDeque<Alert>>>,
    max_alerts: usize,
    alert_callbacks: Arc<RwLock<Vec<Box<dyn Fn(&Alert) + Send + Sync>>>>,
}

impl AlertManager {
    /// Create a new alert manager
    pub fn new(max_alerts: usize) -> Self {
        Self {
            alerts: Arc::new(RwLock::new(VecDeque::new())),
            max_alerts,
            alert_callbacks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Emit an alert
    pub async fn emit(&self, level: AlertLevel, component: &str, message: String) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let alert = Alert {
            level,
            message,
            component: component.to_string(),
            timestamp,
            metadata: std::collections::HashMap::new(),
        };

        // Add to alerts queue
        let mut alerts = self.alerts.write().await;
        alerts.push_back(alert.clone());

        // Keep only last N alerts
        while alerts.len() > self.max_alerts {
            alerts.pop_front();
        }

        // Call callbacks
        let callbacks = self.alert_callbacks.read().await;
        for callback in callbacks.iter() {
            callback(&alert);
        }

        // Log based on level
        match level {
            AlertLevel::Info => log::info!("[{}] {}", component, alert.message),
            AlertLevel::Warning => log::warn!("[{}] {}", component, alert.message),
            AlertLevel::Error => log::error!("[{}] {}", component, alert.message),
            AlertLevel::Critical => {
                log::error!("[CRITICAL] [{}] {}", component, alert.message);
                // In production, you might want to send critical alerts immediately
            }
        }
    }

    /// Register an alert callback
    pub async fn register_callback<F>(&self, callback: F)
    where
        F: Fn(&Alert) + Send + Sync + 'static,
    {
        let mut callbacks = self.alert_callbacks.write().await;
        callbacks.push(Box::new(callback));
    }

    /// Get recent alerts
    pub async fn get_recent_alerts(&self, count: usize) -> Vec<Alert> {
        let alerts = self.alerts.read().await;
        alerts.iter().rev().take(count).cloned().collect()
    }

    /// Get alerts by level
    pub async fn get_alerts_by_level(&self, level: AlertLevel) -> Vec<Alert> {
        let alerts = self.alerts.read().await;
        alerts
            .iter()
            .filter(|a| a.level == level)
            .cloned()
            .collect()
    }

    /// Clear all alerts
    pub async fn clear(&self) {
        *self.alerts.write().await = VecDeque::new();
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new(1000)
    }
}
