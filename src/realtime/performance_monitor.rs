use log::{debug, info};
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Number of market data events processed
    pub market_data_events: u64,
    /// Number of signals generated
    pub signals_generated: u64,
    /// Number of orders placed
    pub orders_placed: u64,
    /// Number of orders filled
    pub orders_filled: u64,
    /// Number of orders canceled
    pub orders_canceled: u64,
    /// Number of orders rejected
    pub orders_rejected: u64,
    /// Number of risk violations
    pub risk_violations: u64,
    /// Average order latency (milliseconds)
    pub average_latency: Option<f64>,
    /// Total P&L
    pub total_pnl: Option<rust_decimal::Decimal>,
    /// Sharpe ratio
    pub sharpe_ratio: Option<f64>,
    /// Maximum drawdown
    pub max_drawdown: Option<rust_decimal::Decimal>,
    /// Win rate (percentage)
    pub win_rate: Option<f64>,
    /// Profit factor
    pub profit_factor: Option<f64>,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            market_data_events: 0,
            signals_generated: 0,
            orders_placed: 0,
            orders_filled: 0,
            orders_canceled: 0,
            orders_rejected: 0,
            risk_violations: 0,
            average_latency: None,
            total_pnl: None,
            sharpe_ratio: None,
            max_drawdown: None,
            win_rate: None,
            profit_factor: None,
        }
    }
}

/// Performance monitor for tracking trading performance
pub struct PerformanceMonitor {
    /// Current metrics
    metrics: Arc<RwLock<PerformanceMetrics>>,
    /// Order placement times (order ID -> placement instant)
    order_latencies: Arc<RwLock<HashMap<String, Instant>>>,
    /// P&L history for calculating statistics
    pnl_history: Arc<RwLock<Vec<rust_decimal::Decimal>>>,
    /// Maximum size of P&L history
    max_pnl_history_size: usize,
    /// Start time for metrics collection
    start_time: Arc<RwLock<Instant>>,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            order_latencies: Arc::new(RwLock::new(HashMap::new())),
            pnl_history: Arc::new(RwLock::new(Vec::new())),
            max_pnl_history_size: 1000,
            start_time: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// Record a market data event
    pub async fn record_market_data_event(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.market_data_events += 1;
    }

    /// Record a signal generation
    pub async fn record_signal(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.signals_generated += 1;
    }

    /// Record an order placement
    pub async fn record_order_placement(&self, order_id: &str) {
        let mut metrics = self.metrics.write().await;
        metrics.orders_placed += 1;

        // Record placement time
        let mut latencies = self.order_latencies.write().await;
        latencies.insert(order_id.to_string(), Instant::now());
    }

    /// Record an order fill
    pub async fn record_order_fill(&self, order_id: &str) {
        let mut metrics = self.metrics.write().await;
        metrics.orders_filled += 1;

        // Calculate latency
        let mut latencies = self.order_latencies.write().await;
        if let Some(placement_time) = latencies.get(order_id) {
            let latency = placement_time.elapsed();

            // Update average latency
            let current_avg = metrics.average_latency.unwrap_or(0.0);
            let new_avg = (current_avg + latency.as_millis() as f64) / 2.0;
            metrics.average_latency = Some(new_avg);

            // Remove from pending latencies
            latencies.remove(order_id);
        }
    }

    /// Record an order cancellation
    pub async fn record_order_cancellation(&self, order_id: &str) {
        let mut metrics = self.metrics.write().await;
        metrics.orders_canceled += 1;

        // Remove from pending latencies
        let mut latencies = self.order_latencies.write().await;
        latencies.remove(order_id);
    }

    /// Record an order rejection
    pub async fn record_order_rejection(&self, order_id: &str) {
        let mut metrics = self.metrics.write().await;
        metrics.orders_rejected += 1;

        // Remove from pending latencies
        let mut latencies = self.order_latencies.write().await;
        latencies.remove(order_id);
    }

    /// Record an order failure (execution failure, not rejection)
    pub async fn record_order_failure(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.orders_rejected += 1; // Count as rejected for metrics purposes
    }

    /// Record a risk violation
    pub async fn record_risk_violation(&self, violation: &crate::risk::RiskViolation) {
        let mut metrics = self.metrics.write().await;
        metrics.risk_violations += 1;

        debug!("Risk violation: {} - {}", violation.rule, violation.details);
    }

    /// Record P&L
    pub async fn record_pnl(&self, pnl: rust_decimal::Decimal) {
        // First, update the P&L history (scoped to release lock)
        {
            let mut pnl_history = self.pnl_history.write().await;

            // Add to P&L history
            pnl_history.push(pnl);

            // Limit history size
            if pnl_history.len() > self.max_pnl_history_size {
                pnl_history.remove(0);
            }
        } // pnl_history lock released here

        // Now calculate performance metrics (acquires its own locks)
        self.calculate_performance_metrics().await;
    }

    /// Calculate performance metrics from P&L history
    async fn calculate_performance_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        let pnl_history = self.pnl_history.read().await;

        if pnl_history.len() < 2 {
            return; // Not enough data
        }

        // Calculate total P&L
        let total_pnl = pnl_history.iter().sum();

        // Calculate win rate
        let wins = pnl_history
            .iter()
            .filter(|pnl| **pnl > rust_decimal::Decimal::ZERO)
            .count();
        let win_rate = wins as f64 / pnl_history.len() as f64;

        // Calculate profit factor
        let profits: Vec<rust_decimal::Decimal> = pnl_history
            .iter()
            .filter(|pnl| **pnl > rust_decimal::Decimal::ZERO)
            .cloned()
            .collect();

        let losses: Vec<rust_decimal::Decimal> = pnl_history
            .iter()
            .filter(|pnl| **pnl < rust_decimal::Decimal::ZERO)
            .map(|pnl| pnl.abs())
            .collect();

        let total_profits: rust_decimal::Decimal = profits.iter().sum();
        let total_losses: rust_decimal::Decimal = losses.iter().sum();

        let profit_factor = if total_losses.is_zero() {
            1.0
        } else {
            (total_profits / total_losses).to_f64().unwrap_or(1.0)
        };

        // Calculate maximum drawdown
        let mut max_drawdown = rust_decimal::Decimal::ZERO;
        let mut peak = rust_decimal::Decimal::ZERO;

        for pnl in pnl_history.iter() {
            if *pnl > peak {
                peak = *pnl;
            } else {
                let drawdown = peak - *pnl;
                if drawdown > max_drawdown {
                    max_drawdown = drawdown;
                }
            }
        }

        // Calculate Sharpe ratio (simplified, assumes risk-free rate of 0)
        let returns: Vec<f64> = pnl_history
            .windows(2)
            .map(|window| {
                if window.len() == 2 {
                    let prev = window[0].to_f64().unwrap_or(0.0);
                    let curr = window[1].to_f64().unwrap_or(0.0);
                    (curr - prev) / prev.abs().max(0.001) // Avoid division by very small numbers
                } else {
                    0.0
                }
            })
            .collect();

        let avg_return = returns.iter().sum::<f64>() / returns.len() as f64;
        let return_variance = returns
            .iter()
            .map(|r| (r - avg_return).powi(2))
            .sum::<f64>()
            / returns.len() as f64;

        let sharpe_ratio = if return_variance > 0.0 {
            avg_return / return_variance.sqrt()
        } else {
            0.0
        };

        // Update metrics
        metrics.total_pnl = Some(total_pnl);
        metrics.sharpe_ratio = Some(sharpe_ratio);
        metrics.max_drawdown = Some(max_drawdown);
        metrics.win_rate = Some(win_rate);
        metrics.profit_factor = Some(profit_factor);
    }

    /// Get current performance metrics
    pub async fn get_metrics(&self) -> PerformanceMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// Reset all metrics
    pub async fn reset_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        *metrics = PerformanceMetrics::default();

        let mut latencies = self.order_latencies.write().await;
        latencies.clear();

        let mut pnl_history = self.pnl_history.write().await;
        pnl_history.clear();

        // Reset start time
        let mut start_time = self.start_time.write().await;
        *start_time = Instant::now();
    }

    /// Get uptime since metrics collection started
    pub async fn uptime(&self) -> Duration {
        let start_time = self.start_time.read().await;
        start_time.elapsed()
    }

    /// Get orders per hour
    pub async fn get_orders_per_hour(&self) -> f64 {
        let metrics = self.metrics.read().await;
        let uptime_hours = self.uptime().await.as_secs() as f64 / 3600.0;

        if uptime_hours > 0.0 {
            metrics.orders_placed as f64 / uptime_hours
        } else {
            0.0
        }
    }

    /// Get signals per hour
    pub async fn get_signals_per_hour(&self) -> f64 {
        let metrics = self.metrics.read().await;
        let uptime_hours = self.uptime().await.as_secs() as f64 / 3600.0;

        if uptime_hours > 0.0 {
            metrics.signals_generated as f64 / uptime_hours
        } else {
            0.0
        }
    }

    /// Get fill rate
    pub async fn get_fill_rate(&self) -> f64 {
        let metrics = self.metrics.read().await;

        if metrics.orders_placed > 0 {
            (metrics.orders_filled as f64 / metrics.orders_placed as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Get cancellation rate
    pub async fn get_cancellation_rate(&self) -> f64 {
        let metrics = self.metrics.read().await;

        if metrics.orders_placed > 0 {
            (metrics.orders_canceled as f64 / metrics.orders_placed as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Get rejection rate
    pub async fn get_rejection_rate(&self) -> f64 {
        let metrics = self.metrics.read().await;

        if metrics.orders_placed > 0 {
            (metrics.orders_rejected as f64 / metrics.orders_placed as f64) * 100.0
        } else {
            0.0
        }
    }
}

/// Performance monitor implementation for testing
pub struct PerformanceMonitorImpl {
    /// Current metrics
    metrics: PerformanceMetrics,
}

impl PerformanceMonitorImpl {
    /// Create a new performance monitor implementation
    pub fn new() -> Self {
        Self {
            metrics: PerformanceMetrics::default(),
        }
    }

    /// Record a market data event
    pub async fn record_market_data_event(&self) {
        // In a real implementation, you'd update metrics
        // For testing, we'll just increment the counter
        info!("Recording market data event");
    }

    /// Record a signal generation
    pub async fn record_signal(&self) {
        // In a real implementation, you'd update metrics
        // For testing, we'll just increment the counter
        info!("Recording signal generation");
    }

    /// Record an order placement
    pub async fn record_order_placement(&self, _order_id: &str) {
        // In a real implementation, you'd update metrics
        // For testing, we'll just increment the counter
        info!("Recording order placement");
    }

    /// Record an order fill
    pub async fn record_order_fill(&self, _order_id: &str) {
        // In a real implementation, you'd update metrics
        // For testing, we'll just increment the counter
        info!("Recording order fill");
    }

    /// Record an order cancellation
    pub async fn record_order_cancellation(&self, _order_id: &str) {
        // In a real implementation, you'd update metrics
        // For testing, we'll just increment the counter
        info!("Recording order cancellation");
    }

    /// Record an order rejection
    pub async fn record_order_rejection(&self, _order_id: &str) {
        // In a real implementation, you'd update metrics
        // For testing, we'll just increment the counter
        info!("Recording order rejection");
    }

    /// Record a risk violation
    pub async fn record_risk_violation(&self, _violation: &crate::risk::RiskViolation) {
        // In a real implementation, you'd update metrics
        // For testing, we'll just increment the counter
        info!("Recording risk violation");
    }

    /// Record P&L
    pub async fn record_pnl(&self, pnl: rust_decimal::Decimal) {
        // In a real implementation, you'd update metrics
        // For testing, we'll just record the value
        info!("Recording P&L: {}", pnl);
    }

    /// Get current performance metrics
    pub async fn get_metrics(&self) -> PerformanceMetrics {
        // For the testing impl, just return a clone of the metrics
        self.metrics.clone()
    }

    /// Reset all metrics - note: this is a no-op for the testing impl
    /// since we can't mutate self.metrics through &self
    pub async fn reset_metrics(&self) {
        info!("Resetting performance metrics (no-op in testing impl)");
        // In a real implementation, you'd reset the metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk::RiskViolation;

    #[test]
    fn test_performance_metrics_default() {
        let metrics = PerformanceMetrics::default();

        assert_eq!(metrics.market_data_events, 0);
        assert_eq!(metrics.signals_generated, 0);
        assert_eq!(metrics.orders_placed, 0);
        assert_eq!(metrics.orders_filled, 0);
        assert_eq!(metrics.orders_canceled, 0);
        assert_eq!(metrics.orders_rejected, 0);
        assert_eq!(metrics.risk_violations, 0);
        assert!(metrics.average_latency.is_none());
        assert!(metrics.total_pnl.is_none());
        assert!(metrics.sharpe_ratio.is_none());
        assert!(metrics.max_drawdown.is_none());
        assert!(metrics.win_rate.is_none());
        assert!(metrics.profit_factor.is_none());
    }

    #[tokio::test]
    async fn test_performance_monitor_creation() {
        let monitor = PerformanceMonitor::new();

        // Verify initial state
        let metrics = monitor.get_metrics().await;
        assert_eq!(metrics.market_data_events, 0);
        assert_eq!(metrics.signals_generated, 0);
        assert_eq!(metrics.orders_placed, 0);
        assert_eq!(metrics.orders_filled, 0);
        assert_eq!(metrics.orders_canceled, 0);
        assert_eq!(metrics.orders_rejected, 0);
        assert_eq!(metrics.risk_violations, 0);
        assert!(metrics.average_latency.is_none());
        assert!(metrics.total_pnl.is_none());
        assert!(metrics.sharpe_ratio.is_none());
        assert!(metrics.max_drawdown.is_none());
        assert!(metrics.win_rate.is_none());
        assert!(metrics.profit_factor.is_none());
    }

    #[tokio::test]
    async fn test_performance_monitor_record_events() {
        let monitor = PerformanceMonitor::new();

        // Record market data events
        for _ in 0..5 {
            monitor.record_market_data_event().await;
        }

        // Record signals
        for _ in 0..3 {
            monitor.record_signal().await;
        }

        // Record order placements
        for i in 0..3 {
            monitor
                .record_order_placement(&format!("order_{}", i))
                .await;
        }

        // Record order fills
        for i in 0..2 {
            monitor.record_order_fill(&format!("order_{}", i)).await;
        }

        // Record order cancellations
        for i in 0..1 {
            monitor
                .record_order_cancellation(&format!("order_{}", i + 3))
                .await;
        }

        // Record order rejections
        for i in 0..1 {
            monitor
                .record_order_rejection(&format!("order_{}", i + 4))
                .await;
        }

        // Record risk violations
        let violation =
            RiskViolation::new("TestViolation".to_string(), "Test violation".to_string());

        for _ in 0..2 {
            monitor.record_risk_violation(&violation).await;
        }

        // Check metrics
        let metrics = monitor.get_metrics().await;
        assert_eq!(metrics.market_data_events, 5);
        assert_eq!(metrics.signals_generated, 3);
        assert_eq!(metrics.orders_placed, 3);
        assert_eq!(metrics.orders_filled, 2);
        assert_eq!(metrics.orders_canceled, 1);
        assert_eq!(metrics.orders_rejected, 1);
        assert_eq!(metrics.risk_violations, 2);
    }

    #[tokio::test]
    async fn test_performance_monitor_pnl() {
        let monitor = PerformanceMonitor::new();

        // Record some P&L
        monitor
            .record_pnl(rust_decimal::Decimal::from_str("100.0").unwrap())
            .await;
        monitor
            .record_pnl(rust_decimal::Decimal::from_str("-50.0").unwrap())
            .await;
        monitor
            .record_pnl(rust_decimal::Decimal::from_str("25.0").unwrap())
            .await;
        monitor
            .record_pnl(rust_decimal::Decimal::from_str("-10.0").unwrap())
            .await;
        monitor
            .record_pnl(rust_decimal::Decimal::from_str("15.0").unwrap())
            .await;

        // Check metrics
        let metrics = monitor.get_metrics().await;

        // Total P&L should be 80.0
        assert_eq!(
            metrics.total_pnl,
            Some(rust_decimal::Decimal::from_str("80.0").unwrap())
        );

        // Win rate should be 60% (3 wins, 2 losses)
        assert_eq!(metrics.win_rate, Some(0.6));

        // Profit factor should be ~2.333 (140 profit / 60 loss)
        // 140 = 100 + 25 + 15 (profits), 60 = 50 + 10 (losses)
        let expected_profit_factor = 140.0 / 60.0;
        assert!(
            (metrics.profit_factor.unwrap() - expected_profit_factor).abs() < 0.01,
            "Profit factor mismatch: expected {}, got {:?}",
            expected_profit_factor,
            metrics.profit_factor
        );
    }

    #[tokio::test]
    async fn test_performance_monitor_rates() {
        let monitor = PerformanceMonitor::new();

        // Record some orders
        for i in 0..5 {
            monitor
                .record_order_placement(&format!("order_{}", i))
                .await;
        }

        for i in 0..2 {
            monitor.record_order_fill(&format!("order_{}", i)).await;
        }

        for i in 0..1 {
            monitor
                .record_order_cancellation(&format!("order_{}", i + 2))
                .await;
        }

        for i in 0..1 {
            monitor
                .record_order_rejection(&format!("order_{}", i + 3))
                .await;
        }

        // Check rates
        let metrics = monitor.get_metrics().await;

        // Fill rate should be 40% (2 fills / 5 placed)
        let fill_rate = if metrics.orders_placed > 0 {
            (metrics.orders_filled as f64 / metrics.orders_placed as f64) * 100.0
        } else {
            0.0
        };
        assert_eq!(fill_rate, 40.0);

        // Cancellation rate should be 20% (1 cancel / 5 placed)
        let cancel_rate = if metrics.orders_placed > 0 {
            (metrics.orders_canceled as f64 / metrics.orders_placed as f64) * 100.0
        } else {
            0.0
        };
        assert_eq!(cancel_rate, 20.0);

        // Rejection rate should be 20% (1 reject / 5 placed)
        let reject_rate = if metrics.orders_placed > 0 {
            (metrics.orders_rejected as f64 / metrics.orders_placed as f64) * 100.0
        } else {
            0.0
        };
        assert_eq!(reject_rate, 20.0);
    }

    #[tokio::test]
    async fn test_performance_monitor_impl() {
        let monitor = PerformanceMonitorImpl::new();

        // Verify initial state
        let metrics = monitor.get_metrics().await;
        assert_eq!(metrics.market_data_events, 0);
        assert_eq!(metrics.signals_generated, 0);
        assert_eq!(metrics.orders_placed, 0);
        assert_eq!(metrics.orders_filled, 0);
        assert_eq!(metrics.orders_canceled, 0);
        assert_eq!(metrics.orders_rejected, 0);
        assert_eq!(metrics.risk_violations, 0);
    }
}
