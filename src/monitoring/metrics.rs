use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Metric value types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(Vec<f64>),
}

/// A single metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: MetricValue,
    pub timestamp: u64,
    pub tags: HashMap<String, String>,
}

/// Metrics collector for tracking system metrics
#[allow(dead_code)]
pub struct MetricsCollector {
    metrics: Arc<RwLock<HashMap<String, Metric>>>,
    counters: Arc<RwLock<HashMap<String, u64>>>,
    gauges: Arc<RwLock<HashMap<String, f64>>>,
    histograms: Arc<RwLock<HashMap<String, Vec<f64>>>>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
            histograms: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Increment a counter
    pub async fn increment_counter(&self, name: &str, value: u64) {
        let mut counters = self.counters.write().await;
        *counters.entry(name.to_string()).or_insert(0) += value;
    }

    /// Set a gauge value
    pub async fn set_gauge(&self, name: &str, value: f64) {
        let mut gauges = self.gauges.write().await;
        gauges.insert(name.to_string(), value);
    }

    /// Record a histogram value
    pub async fn record_histogram(&self, name: &str, value: f64) {
        let mut histograms = self.histograms.write().await;
        histograms
            .entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(value);

        // Keep only last 1000 values
        if let Some(values) = histograms.get_mut(name) {
            if values.len() > 1000 {
                values.remove(0);
            }
        }
    }

    /// Get all metrics
    pub async fn get_metrics(&self) -> Vec<Metric> {
        let mut metrics = Vec::new();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Collect counters
        let counters = self.counters.read().await;
        for (name, value) in counters.iter() {
            metrics.push(Metric {
                name: format!("counter.{}", name),
                value: MetricValue::Counter(*value),
                timestamp,
                tags: HashMap::new(),
            });
        }

        // Collect gauges
        let gauges = self.gauges.read().await;
        for (name, value) in gauges.iter() {
            metrics.push(Metric {
                name: format!("gauge.{}", name),
                value: MetricValue::Gauge(*value),
                timestamp,
                tags: HashMap::new(),
            });
        }

        // Collect histogram summaries
        let histograms = self.histograms.read().await;
        for (name, values) in histograms.iter() {
            if !values.is_empty() {
                let sum: f64 = values.iter().sum();
                let count = values.len() as f64;
                let avg = sum / count;
                let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

                metrics.push(Metric {
                    name: format!("histogram.{}.avg", name),
                    value: MetricValue::Gauge(avg),
                    timestamp,
                    tags: HashMap::new(),
                });
                metrics.push(Metric {
                    name: format!("histogram.{}.min", name),
                    value: MetricValue::Gauge(min),
                    timestamp,
                    tags: HashMap::new(),
                });
                metrics.push(Metric {
                    name: format!("histogram.{}.max", name),
                    value: MetricValue::Gauge(max),
                    timestamp,
                    tags: HashMap::new(),
                });
            }
        }

        metrics
    }

    /// Reset all metrics
    pub async fn reset(&self) {
        *self.counters.write().await = HashMap::new();
        *self.gauges.write().await = HashMap::new();
        *self.histograms.write().await = HashMap::new();
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
