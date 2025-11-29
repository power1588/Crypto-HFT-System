use crate::oms::{OrderManager, RateLimiter};
use crate::realtime::{OrderExecutor, PerformanceMonitor, RiskManager, SignalGenerator};
use crate::risk::RiskEngine;
use crate::strategy::{Signal, Strategy, StrategyEngine};
use crate::traits::{ExecutionClient, MarketDataStream};
use log::{debug, error, info, warn};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration, Instant};

/// Event loop configuration
#[derive(Debug, Clone)]
pub struct EventLoopConfig {
    /// Symbols to trade
    pub symbols: Vec<String>,
    /// Strategy update interval
    pub strategy_update_interval: Duration,
    /// Order check interval
    pub order_check_interval: Duration,
    /// Performance report interval
    pub performance_report_interval: Duration,
    /// Maximum consecutive errors before stopping
    pub max_consecutive_errors: u32,
    /// Error recovery delay
    pub error_recovery_delay: Duration,
}

impl Default for EventLoopConfig {
    fn default() -> Self {
        Self {
            symbols: vec!["BTCUSDT".to_string()],
            strategy_update_interval: Duration::from_millis(100),
            order_check_interval: Duration::from_millis(500),
            performance_report_interval: Duration::from_secs(60),
            max_consecutive_errors: 5,
            error_recovery_delay: Duration::from_secs(5),
        }
    }
}

/// Event loop for processing market data and trading signals
#[allow(dead_code)]
pub struct EventLoop<S>
where
    S: Strategy + Send + Sync + 'static,
{
    /// Configuration
    config: EventLoopConfig,
    /// Market data stream
    market_stream: Arc<
        RwLock<
            dyn MarketDataStream<Error = Box<dyn std::error::Error + Send + Sync>> + Send + Sync,
        >,
    >,
    /// Execution client
    execution_client:
        Arc<dyn ExecutionClient<Error = Box<dyn std::error::Error + Send + Sync>> + Send + Sync>,
    /// Strategy engine
    strategy_engine: Arc<RwLock<StrategyEngine<S>>>,
    /// Order manager
    order_manager: Arc<
        RwLock<dyn OrderManager<Error = Box<dyn std::error::Error + Send + Sync>> + Send + Sync>,
    >,
    /// Rate limiter
    rate_limiter: Arc<RateLimiter>,
    /// Risk engine
    risk_engine: Arc<RwLock<RiskEngine>>,
    /// Signal generator
    signal_generator: Arc<SignalGenerator<S>>,
    /// Order executor
    order_executor: Arc<OrderExecutor>,
    /// Risk manager
    risk_manager: Arc<RiskManager>,
    /// Performance monitor
    performance_monitor: Arc<PerformanceMonitor>,
    /// Running state
    running: Arc<RwLock<bool>>,
    /// Consecutive error count
    consecutive_errors: Arc<RwLock<u32>>,
    /// Last performance report time
    last_performance_report: Arc<RwLock<Instant>>,
}

impl<S> EventLoop<S>
where
    S: Strategy + Send + Sync + 'static,
{
    /// Create a new event loop
    pub fn new(
        config: EventLoopConfig,
        market_stream: Arc<
            RwLock<
                dyn MarketDataStream<Error = Box<dyn std::error::Error + Send + Sync>>
                    + Send
                    + Sync,
            >,
        >,
        execution_client: Arc<
            dyn ExecutionClient<Error = Box<dyn std::error::Error + Send + Sync>> + Send + Sync,
        >,
        strategy: S,
        order_manager: Arc<
            RwLock<
                dyn OrderManager<Error = Box<dyn std::error::Error + Send + Sync>> + Send + Sync,
            >,
        >,
        rate_limiter: Arc<RateLimiter>,
        risk_engine: Arc<RwLock<RiskEngine>>,
        signal_generator: Arc<SignalGenerator<S>>,
        order_executor: Arc<OrderExecutor>,
        risk_manager: Arc<RiskManager>,
        performance_monitor: Arc<PerformanceMonitor>,
    ) -> Self {
        // Create strategy engine with the provided strategy
        let strategy_engine = Arc::new(RwLock::new(StrategyEngine::new(
            strategy,
            config.strategy_update_interval,
        )));

        Self {
            config,
            market_stream,
            execution_client,
            strategy_engine,
            order_manager,
            rate_limiter,
            risk_engine,
            signal_generator,
            order_executor,
            risk_manager,
            performance_monitor,
            running: Arc::new(RwLock::new(false)),
            consecutive_errors: Arc::new(RwLock::new(0)),
            last_performance_report: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// Start the event loop
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting event loop for symbols: {:?}", self.config.symbols);

        // Set running state
        {
            let mut running = self.running.write().await;
            *running = true;
        }

        // Subscribe to market data
        self.subscribe_to_market_data().await?;

        // Main event loop
        let mut last_strategy_update = Instant::now();
        let mut last_order_check = Instant::now();

        while self.is_running().await {
            // Process market data events
            if let Err(e) = self.process_market_data().await {
                error!("Error processing market data: {}", e);
                self.increment_error_count().await;

                // Wait before retrying
                sleep(self.config.error_recovery_delay).await;
                continue;
            }

            // Generate and process signals
            let now = Instant::now();

            if now.duration_since(last_strategy_update) >= self.config.strategy_update_interval {
                if let Err(e) = self.process_signals().await {
                    error!("Error processing signals: {}", e);
                    self.increment_error_count().await;
                }
                last_strategy_update = now;
            }

            if now.duration_since(last_order_check) >= self.config.order_check_interval {
                if let Err(e) = self.check_orders().await {
                    error!("Error checking orders: {}", e);
                    self.increment_error_count().await;
                }
                last_order_check = now;
            }

            // Report performance metrics
            if now.duration_since(*self.last_performance_report.read().await)
                >= self.config.performance_report_interval
            {
                if let Err(e) = self.report_performance().await {
                    error!("Error reporting performance: {}", e);
                    self.increment_error_count().await;
                }

                let mut last_report = self.last_performance_report.write().await;
                *last_report = now;
            }

            // Small delay to prevent busy waiting
            sleep(Duration::from_millis(10)).await;
        }

        info!("Event loop stopped");
        Ok(())
    }

    /// Stop the event loop
    pub async fn stop(&self) {
        info!("Stopping event loop");

        // Set running state to false
        {
            let mut running = self.running.write().await;
            *running = false;
        }

        // Unsubscribe from market data
        if let Err(e) = self.unsubscribe_from_market_data().await {
            error!("Error unsubscribing from market data: {}", e);
        }
    }

    /// Check if the event loop is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Subscribe to market data for all configured symbols
    pub async fn subscribe_to_market_data(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let symbol_refs: Vec<&str> = self.config.symbols.iter().map(|s| s.as_str()).collect();

        info!("Subscribing to market data for symbols: {:?}", symbol_refs);

        // Apply rate limiting
        self.rate_limiter.wait_for_slot().await;

        let mut stream = self.market_stream.write().await;
        stream.subscribe(&symbol_refs).await.map_err(|e| {
            error!("Failed to subscribe to market data: {}", e);
            e // Error is already Box<dyn Error>, don't re-box
        })
    }

    /// Unsubscribe from market data
    pub async fn unsubscribe_from_market_data(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let symbol_refs: Vec<&str> = self.config.symbols.iter().map(|s| s.as_str()).collect();

        info!(
            "Unsubscribing from market data for symbols: {:?}",
            symbol_refs
        );

        let mut stream = self.market_stream.write().await;
        stream.unsubscribe(&symbol_refs).await.map_err(|e| {
            error!("Failed to unsubscribe from market data: {}", e);
            e // Error is already Box<dyn Error>, don't re-box
        })
    }

    /// Process market data events
    pub async fn process_market_data(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Process all available market data events
        let mut stream = self.market_stream.write().await;
        while let Some(event_result) = stream.next().await {
            match event_result {
                Ok(event) => {
                    // Record market data event
                    self.performance_monitor.record_market_data_event().await;

                    // Update strategy with market data
                    {
                        let mut strategy_engine = self.strategy_engine.write().await;
                        let signal = strategy_engine.process_event(event);

                        // Process signal if generated
                        if let Some(signal) = signal {
                            if let Err(e) = self.process_signal(signal).await {
                                error!("Error processing signal: {}", e);
                                return Err(e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Market data stream error: {}", e);
                    return Err(e); // Error is already Box<dyn Error>
                }
            }
        }

        Ok(())
    }

    /// Process a trading signal
    pub async fn process_signal(
        &self,
        signal: Signal,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Processing signal: {:?}", signal);

        // Record signal
        self.performance_monitor.record_signal().await;

        // Check signal against risk rules
        {
            let risk_engine = self.risk_engine.read().await;

            // Convert signal to order for risk checking
            if let Some(order) = self.signal_generator.signal_to_order(&signal) {
                // Check order against risk rules
                match risk_engine.check_order(&order).await {
                    Err(violation) => {
                        warn!(
                            "Signal rejected by risk rules: {} - {}",
                            violation.rule, violation.details
                        );
                        self.performance_monitor
                            .record_risk_violation(&violation)
                            .await;
                        return Ok(()); // Reject signal but don't treat as error
                    }
                    Ok(()) => {
                        // Risk check passed, proceed with execution
                        debug!("Risk check passed for order: {:?}", order);
                    }
                }

                // Execute order
                if let Err(e) = self.order_executor.execute_order(order).await {
                    error!("Failed to execute order: {}", e);
                    self.performance_monitor.record_order_failure().await;
                    return Err(e);
                }
            }
        }

        // Risk manager performs periodic checks asynchronously
        // and doesn't block order execution

        Ok(())
    }

    /// Generate and process signals from strategy
    pub async fn process_signals(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Generate signals from strategy
        let signals = {
            let mut strategy_engine = self.strategy_engine.write().await;
            strategy_engine.generate_signals()
        };

        // Process each signal
        for signal in signals {
            if let Err(e) = self.process_signal(signal).await {
                error!("Error processing signal: {}", e);
                return Err(e);
            }
        }

        Ok(())
    }

    /// Check order status and update order manager
    pub async fn check_orders(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get open orders from order manager
        let order_mgr = self.order_manager.read().await;
        let open_orders = order_mgr.get_open_orders().await.map_err(|e| {
            error!("Error getting open orders: {}", e);
            e // Error is already Box<dyn Error>
        })?;
        drop(order_mgr); // Release read lock before potentially writing

        // Check status of each open order
        for order_report in open_orders {
            // Apply rate limiting
            self.rate_limiter.wait_for_slot().await;

            // Get current order status
            let current_status = self
                .execution_client
                .get_order_status(order_report.order_id.clone())
                .await
                .map_err(|e| {
                    error!("Error getting order status: {}", e);
                    e // Error is already Box<dyn Error>
                })?;

            // If status changed, update order manager
            if current_status.status != order_report.status {
                debug!(
                    "Order {} status changed: {:?} -> {:?}",
                    &order_report.order_id, order_report.status, current_status.status
                );

                // Update order manager
                let mut order_mgr = self.order_manager.write().await;
                if let Err(e) = order_mgr
                    .handle_execution_report(current_status.clone())
                    .await
                {
                    error!("Failed to update order manager: {}", e);
                    return Err(e); // Error is already Box<dyn Error>
                }
            }
        }

        Ok(())
    }

    /// Report performance metrics
    pub async fn report_performance(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get performance metrics
        let metrics = self.performance_monitor.get_metrics().await;

        info!("Performance Report:");
        info!("  Market Data Events: {}", metrics.market_data_events);
        info!("  Signals Generated: {}", metrics.signals_generated);
        info!("  Orders Placed: {}", metrics.orders_placed);
        info!("  Orders Filled: {}", metrics.orders_filled);
        info!("  Orders Canceled: {}", metrics.orders_canceled);
        info!("  Orders Rejected: {}", metrics.orders_rejected);
        info!("  Risk Violations: {}", metrics.risk_violations);
        info!("  Average Latency: {:?}", metrics.average_latency);
        info!("  P&L: {:?}", metrics.total_pnl);

        // Reset performance metrics
        self.performance_monitor.reset_metrics().await;

        Ok(())
    }

    /// Increment consecutive error count
    pub async fn increment_error_count(&self) {
        let mut count = self.consecutive_errors.write().await;
        *count += 1;
    }

    /// Reset consecutive error count
    pub async fn reset_error_count(&self) {
        let mut count = self.consecutive_errors.write().await;
        *count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: The EventLoop tests require complex setup with trait objects.
    // These tests verify basic configuration and state management.

    #[test]
    fn test_event_loop_config_default() {
        let config = EventLoopConfig::default();

        // Verify default configuration
        assert!(config.max_consecutive_errors > 0);
        assert!(config.strategy_update_interval > Duration::ZERO);
        assert!(config.performance_report_interval > Duration::ZERO);
    }

    #[test]
    fn test_event_loop_config_custom() {
        let mut config = EventLoopConfig::default();
        config.max_consecutive_errors = 5;
        config.strategy_update_interval = Duration::from_millis(500);
        config.performance_report_interval = Duration::from_secs(120);

        assert_eq!(config.max_consecutive_errors, 5);
        assert_eq!(config.strategy_update_interval, Duration::from_millis(500));
        assert_eq!(config.performance_report_interval, Duration::from_secs(120));
    }

    // Full EventLoop integration tests require proper setup with trait objects
    // which is complex. The individual component tests (SignalGenerator, 
    // OrderExecutor, RiskManager, PerformanceMonitor) provide coverage
    // for the main functionality.
}
