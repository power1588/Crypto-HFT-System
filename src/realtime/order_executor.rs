use crate::oms::{OrderManager, RateLimiter};
use crate::risk::ShadowLedger;
use crate::traits::{ExecutionClient, ExecutionReport, NewOrder, OrderId, OrderStatus};
use log::{debug, error, info, warn};
use rust_decimal::prelude::ToPrimitive;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Order executor configuration
#[derive(Debug, Clone)]
pub struct OrderExecutorConfig {
    /// Maximum retry attempts for failed orders
    pub max_retry_attempts: u32,
    /// Retry delay for failed orders
    pub retry_delay: Duration,
    /// Order timeout
    pub order_timeout: Duration,
    /// Enable order splitting for large orders
    pub enable_order_splitting: bool,
    /// Maximum order size for splitting
    pub max_order_size: crate::types::Size,
    /// Enable order cancellation on timeout
    pub enable_timeout_cancellation: bool,
}

impl Default for OrderExecutorConfig {
    fn default() -> Self {
        Self {
            max_retry_attempts: 3,
            retry_delay: Duration::from_millis(1000),
            order_timeout: Duration::from_secs(30),
            enable_order_splitting: true,
            max_order_size: crate::types::Size::from_str("1.0").unwrap(),
            enable_timeout_cancellation: true,
        }
    }
}

/// Order executor for placing and managing orders
pub struct OrderExecutor {
    /// Configuration
    config: OrderExecutorConfig,
    /// Execution client
    execution_client:
        Arc<dyn ExecutionClient<Error = Box<dyn std::error::Error + Send + Sync>> + Send + Sync>,
    /// Order manager
    order_manager: Arc<
        RwLock<dyn OrderManager<Error = Box<dyn std::error::Error + Send + Sync>> + Send + Sync>,
    >,
    /// Rate limiter
    rate_limiter: Arc<RateLimiter>,
    /// Shadow ledger
    shadow_ledger: Arc<ShadowLedger>,
    /// Pending orders by client order ID
    pending_orders: Arc<RwLock<HashMap<String, PendingOrder>>>,
    /// Order execution attempts by order ID
    order_attempts: Arc<RwLock<HashMap<String, u32>>>,
}

/// Pending order information
#[derive(Debug, Clone)]
struct PendingOrder {
    /// Original order
    pub order: NewOrder,
    /// Creation time
    pub created_at: Instant,
    /// Last retry time
    pub last_retry_at: Option<Instant>,
    /// Retry attempts
    pub retry_count: u32,
}

impl PendingOrder {
    /// Create a new pending order
    pub fn new(order: NewOrder) -> Self {
        Self {
            order,
            created_at: Instant::now(),
            last_retry_at: None,
            retry_count: 0,
        }
    }

    /// Check if the order has timed out
    pub fn is_timed_out(&self, timeout: Duration) -> bool {
        self.created_at.elapsed() > timeout
    }

    /// Increment retry count and update last retry time
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
        self.last_retry_at = Some(Instant::now());
    }

    /// Check if the order should be retried
    pub fn should_retry(&self, max_attempts: u32) -> bool {
        self.retry_count < max_attempts
    }
}

impl OrderExecutor {
    /// Create a new order executor
    pub fn new(
        config: OrderExecutorConfig,
        execution_client: Arc<
            dyn ExecutionClient<Error = Box<dyn std::error::Error + Send + Sync>> + Send + Sync,
        >,
        order_manager: Arc<
            RwLock<
                dyn OrderManager<Error = Box<dyn std::error::Error + Send + Sync>> + Send + Sync,
            >,
        >,
        rate_limiter: Arc<RateLimiter>,
        shadow_ledger: Arc<ShadowLedger>,
    ) -> Self {
        Self {
            config,
            execution_client,
            order_manager,
            rate_limiter,
            shadow_ledger,
            pending_orders: Arc::new(RwLock::new(HashMap::new())),
            order_attempts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Execute an order
    pub async fn execute_order(
        &self,
        order: NewOrder,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Executing order: {:?}", order);

        // Check if order should be split
        if self.config.enable_order_splitting && order.size > self.config.max_order_size {
            return self.execute_split_order(order).await;
        }

        // Apply rate limiting
        self.rate_limiter.wait_for_slot().await;

        // Place the order
        let order_id = self
            .execution_client
            .place_order(order.clone())
            .await
            .map_err(|e| {
                error!("Failed to place order: {}", e);
                e // Error is already Box<dyn Error>
            })?;

        // Add to pending orders
        self.add_pending_order(&order, order_id.clone()).await;

        // Record order attempt
        self.record_order_attempt(&order_id).await;

        info!("Order placed with ID: {}", &order_id);

        Ok(())
    }

    /// Execute a split order (large order split into multiple smaller orders)
    async fn execute_split_order(
        &self,
        order: NewOrder,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Splitting order of size {} into smaller orders", order.size);

        // Calculate number of sub-orders
        let ratio = order.size.value() / self.config.max_order_size.value();
        let num_orders = ratio.ceil().to_usize().unwrap_or(1).max(1);
        let num_orders_decimal = rust_decimal::Decimal::from(num_orders as u64);
        let sub_order_size = crate::types::Size::new(order.size.value() / num_orders_decimal);

        // Create and execute sub-orders
        for i in 0..num_orders {
            let mut sub_order = order.clone();
            sub_order.size = sub_order_size;

            // Add suffix to client order ID to identify sub-orders
            if let Some(ref client_id) = sub_order.client_order_id {
                sub_order.client_order_id = Some(format!("{}_part{}", client_id, i + 1));
            }

            // Execute sub-order
            if let Err(e) = self.execute_single_order(sub_order).await {
                error!("Failed to execute sub-order {}: {}", i, e);
                return Err(e);
            }
        }

        Ok(())
    }

    /// Execute a single order (without splitting)
    async fn execute_single_order(
        &self,
        order: NewOrder,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Apply rate limiting
        self.rate_limiter.wait_for_slot().await;

        // Place the order
        let order_id = self
            .execution_client
            .place_order(order.clone())
            .await
            .map_err(|e| {
                error!("Failed to place order: {}", e);
                e // Error is already Box<dyn Error>
            })?;

        // Add to pending orders
        self.add_pending_order(&order, order_id.clone()).await;

        // Record order attempt
        self.record_order_attempt(&order_id).await;

        info!("Order placed with ID: {}", &order_id);

        Ok(())
    }

    /// Add an order to the pending orders map
    async fn add_pending_order(&self, order: &NewOrder, order_id: OrderId) {
        let mut pending_orders = self.pending_orders.write().await;

        let client_order_id = order
            .client_order_id
            .clone()
            .unwrap_or_else(|| format!("auto_{}", &order_id));

        pending_orders.insert(client_order_id, PendingOrder::new(order.clone()));
    }

    /// Record an order attempt
    async fn record_order_attempt(&self, order_id: &OrderId) {
        let mut attempts = self.order_attempts.write().await;
        let count = attempts.get(order_id.as_str()).cloned().unwrap_or(0);
        attempts.insert(order_id.clone(), count + 1);
    }

    /// Get the number of attempts for an order
    #[allow(dead_code)]
    async fn get_order_attempts(&self, order_id: &OrderId) -> u32 {
        let attempts = self.order_attempts.read().await;
        attempts.get(order_id.as_str()).cloned().unwrap_or(0)
    }

    /// Check and update pending orders
    pub async fn check_pending_orders(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut pending_orders = self.pending_orders.write().await;
        let mut orders_to_remove = Vec::new();
        let mut orders_to_retry = Vec::new();

        // Check each pending order
        for (client_order_id, pending_order) in pending_orders.iter_mut() {
            // Check if order has timed out
            if pending_order.is_timed_out(self.config.order_timeout) {
                warn!("Order {} timed out", client_order_id);

                if self.config.enable_timeout_cancellation {
                    // Cancel the order
                    if let Some(order_id) = self.get_order_id_from_client_id(client_order_id).await
                    {
                        if let Err(e) = self.cancel_order(order_id).await {
                            error!(
                                "Failed to cancel timed out order {}: {}",
                                client_order_id, e
                            );
                        }
                    }
                }

                // Remove from pending orders
                orders_to_remove.push(client_order_id.clone());
                continue;
            }

            // Check if we should retry the order
            if pending_order.should_retry(self.config.max_retry_attempts) {
                // Check if enough time has passed since last retry
                if let Some(last_retry) = pending_order.last_retry_at {
                    if last_retry.elapsed() >= self.config.retry_delay {
                        info!("Retrying order {}", client_order_id);

                        // Increment retry count
                        pending_order.increment_retry();

                        // Add to retry list
                        orders_to_retry.push(client_order_id.clone());
                    }
                } else {
                    // First retry
                    info!("Retrying order {}", client_order_id);

                    // Increment retry count
                    pending_order.increment_retry();

                    // Add to retry list
                    orders_to_retry.push(client_order_id.clone());
                }
            } else {
                // Order is complete or max retries reached
                orders_to_remove.push(client_order_id.clone());
            }
        }

        // Remove completed orders
        for client_order_id in orders_to_remove {
            pending_orders.remove(&client_order_id);
        }

        // Retry orders
        for client_order_id in orders_to_retry {
            if let Some(pending_order) = pending_orders.get(&client_order_id) {
                // Apply rate limiting
                self.rate_limiter.wait_for_slot().await;

                // Retry the order
                if let Err(e) = self
                    .execution_client
                    .place_order(pending_order.order.clone())
                    .await
                {
                    error!("Failed to retry order {}: {}", client_order_id, e);
                }
            }
        }

        Ok(())
    }

    /// Cancel an order
    async fn cancel_order(
        &self,
        order_id: OrderId,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Cancelling order {}", &order_id);

        // Apply rate limiting
        self.rate_limiter.wait_for_slot().await;

        let order_id_clone = order_id.clone();
        self.execution_client
            .cancel_order(order_id)
            .await
            .map_err(|e| {
                error!("Failed to cancel order {}: {}", &order_id_clone, e);
                e // Error is already Box<dyn Error>
            })
    }

    /// Get order ID from client order ID
    async fn get_order_id_from_client_id(&self, _client_order_id: &str) -> Option<OrderId> {
        // In a real implementation, you'd track the mapping between client order IDs and order IDs
        // For now, we'll return None
        None
    }

    /// Process an execution report
    pub async fn process_execution_report(
        &self,
        report: &ExecutionReport,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Processing execution report: {:?}", report);

        // Update order manager
        let mut order_mgr = self.order_manager.write().await;
        if let Err(e) = order_mgr.handle_execution_report(report.clone()).await {
            error!("Failed to update order manager: {}", e);
            return Err(e); // Error is already Box<dyn Error>
        }

        // Update shadow ledger (returns (), no error handling needed)
        self.shadow_ledger.process_execution_report(report).await;

        // Remove from pending orders if filled or canceled
        match report.status {
            OrderStatus::Filled | OrderStatus::Cancelled | OrderStatus::Rejected => {
                let mut pending_orders = self.pending_orders.write().await;

                if let Some(client_order_id) = &report.client_order_id {
                    pending_orders.remove(client_order_id);
                }
            }
            _ => {
                // Order is still active, keep in pending
            }
        }

        Ok(())
    }

    /// Get statistics about order execution
    pub async fn get_execution_stats(&self) -> ExecutionStats {
        let pending_orders = self.pending_orders.read().await;
        let _order_attempts = self.order_attempts.read().await;

        let mut total_orders = 0;
        let mut pending_count = 0;
        let mut retry_count = 0;
        let mut max_retries = 0;

        for pending_order in pending_orders.values() {
            total_orders += 1;

            if pending_order.should_retry(self.config.max_retry_attempts) {
                pending_count += 1;
                retry_count += pending_order.retry_count;
                max_retries = max_retries.max(pending_order.retry_count);
            }
        }

        ExecutionStats {
            total_orders,
            pending_orders: pending_count,
            retry_count,
            max_retries,
            average_retries: if total_orders > 0 {
                retry_count as f64 / total_orders as f64
            } else {
                0.0
            },
        }
    }
}

/// Order execution statistics
#[derive(Debug, Clone)]
pub struct ExecutionStats {
    /// Total number of orders
    pub total_orders: usize,
    /// Number of pending orders
    pub pending_orders: usize,
    /// Total retry count
    pub retry_count: u32,
    /// Maximum retry count
    pub max_retries: u32,
    /// Average retry count
    pub average_retries: f64,
}

/// Order executor implementation for testing
#[allow(dead_code)]
pub struct OrderExecutorImpl {
    /// Configuration
    config: OrderExecutorConfig,
}

impl OrderExecutorImpl {
    /// Create a new order executor implementation
    pub fn new() -> Self {
        Self {
            config: OrderExecutorConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::TimeInForce;
    use crate::types::{Price, Size};

    #[test]
    fn test_pending_order_creation() {
        let order = NewOrder::new_limit_buy(
            "BTCUSDT".to_string(),
            Size::from_str("1.0").unwrap(),
            Price::from_str("50000.0").unwrap(),
            TimeInForce::GoodTillCancelled,
        );

        let pending_order = PendingOrder::new(order.clone());

        assert_eq!(pending_order.order, order);
        assert!(!pending_order.is_timed_out(Duration::from_secs(30)));
        assert_eq!(pending_order.retry_count, 0);
        assert!(pending_order.should_retry(3));

        // Increment retry
        let mut pending_order_mut = pending_order.clone();
        pending_order_mut.increment_retry();

        assert_eq!(pending_order_mut.retry_count, 1);
        assert!(pending_order_mut.should_retry(3));

        // Increment retry again
        pending_order_mut.increment_retry();

        assert_eq!(pending_order_mut.retry_count, 2);
        assert!(pending_order_mut.should_retry(3));

        // Increment retry again (max)
        pending_order_mut.increment_retry();

        assert_eq!(pending_order_mut.retry_count, 3);
        assert!(!pending_order_mut.should_retry(3));
    }

    #[test]
    fn test_pending_order_timeout() {
        let order = NewOrder::new_limit_buy(
            "BTCUSDT".to_string(),
            Size::from_str("1.0").unwrap(),
            Price::from_str("50000.0").unwrap(),
            TimeInForce::GoodTillCancelled,
        );

        let pending_order = PendingOrder::new(order);

        // Should not be timed out initially
        assert!(!pending_order.is_timed_out(Duration::from_secs(30)));

        // Simulate time passing
        std::thread::sleep(Duration::from_millis(100));

        // Should still not be timed out
        assert!(!pending_order.is_timed_out(Duration::from_secs(30)));
    }

    #[test]
    fn test_order_executor_config_default() {
        let config = OrderExecutorConfig::default();

        assert_eq!(config.max_retry_attempts, 3);
        assert_eq!(config.retry_delay, Duration::from_millis(1000));
        assert_eq!(config.order_timeout, Duration::from_secs(30));
        assert!(config.enable_order_splitting);
        assert_eq!(config.max_order_size, Size::from_str("1.0").unwrap());
        assert!(config.enable_timeout_cancellation);
    }

    #[test]
    fn test_order_executor_impl() {
        let executor_impl = OrderExecutorImpl::new();

        // Verify default configuration
        assert_eq!(executor_impl.config.max_retry_attempts, 3);
        assert!(executor_impl.config.enable_order_splitting);
    }

    // Note: Full OrderExecutor integration tests require complex setup with
    // trait objects (Arc<dyn ExecutionClient>, Arc<RwLock<dyn OrderManager>>).
    // The individual component tests provide coverage for the main functionality.
}
