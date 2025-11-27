use crate::traits::{OrderManager, OrderId, ExecutionReport, OrderStatus};
use crate::types::{Price, Size};
use std::collections::HashMap;
use std::time::Instant;

/// Order manager that tracks order lifecycle
pub struct OrderManagerImpl {
    /// All tracked orders
    orders: HashMap<OrderId, ExecutionReport>,
    /// Last update time
    last_update: Instant,
}

impl OrderManagerImpl {
    /// Create a new order manager
    pub fn new() -> Self {
        Self {
            orders: HashMap::new(),
            last_update: Instant::now(),
        }
    }

    /// Handle an execution report
    pub async fn handle_execution_report(&mut self, report: ExecutionReport) -> Result<(), crate::traits::OrderManager::Error> {
        // Update or insert the order
        self.orders.insert(report.order_id.clone(), report.clone());
        self.last_update = Instant::now();
        
        // In a real implementation, you might trigger other actions
        // based on the order status change
        
        Ok(())
    }

    /// Get order by ID
    pub async fn get_order(&self, order_id: OrderId) -> Result<ExecutionReport, crate::traits::OrderManager::Error> {
        self.orders
            .get(&order_id)
            .cloned()
            .ok_or(crate::traits::OrderManager::Error::OrderNotFound(order_id))
    }

    /// Get all orders
    pub async fn get_all_orders(&self) -> Result<Vec<ExecutionReport>, crate::traits::OrderManager::Error> {
        Ok(self.orders.values().cloned().collect())
    }

    /// Get orders by symbol
    pub async fn get_orders_by_symbol(&self, symbol: &str) -> Result<Vec<ExecutionReport>, crate::traits::OrderManager::Error> {
        Ok(self
            .orders
            .values()
            .filter(|order| order.symbol == symbol)
            .cloned()
            .collect())
    }

    /// Get open orders
    pub async fn get_open_orders(&self) -> Result<Vec<ExecutionReport>, crate::traits::OrderManager::Error> {
        Ok(self
            .orders
            .values()
            .filter(|order| matches!(order.status, OrderStatus::New | OrderStatus::PartiallyFilled { .. }))
            .cloned()
            .collect())
    }

    /// Get order history
    pub async fn get_order_history(
        &self,
        symbol: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<ExecutionReport>, crate::traits::OrderManager::Error> {
        let mut orders: Vec<&ExecutionReport> = self.orders.values().collect();
        
        // Filter by symbol if provided
        if let Some(s) = symbol {
            orders.retain(|order| order.symbol == s);
        }
        
        // Sort by timestamp descending
        orders.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        // Apply limit if provided
        if let Some(l) = limit {
            orders.truncate(l);
        }
        
        Ok(orders.into_iter().cloned().collect())
    }
}

/// Error type for order manager
#[derive(Debug, Clone)]
pub enum OrderManagerError {
    /// Order not found
    OrderNotFound(OrderId),
    /// Other error
    Other(String),
}

impl std::fmt::Display for OrderManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderManagerError::OrderNotFound(id) => write!(f, "Order not found: {}", id.as_str()),
            OrderManagerError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for OrderManagerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Price, Size};

    #[tokio::test]
    async fn test_order_lifecycle() {
        let mut manager = OrderManagerImpl::new();
        
        // Create a new order
        let order_id = OrderId::new("test_order_1".to_string());
        let order = ExecutionReport {
            order_id: order_id.clone(),
            client_order_id: Some("client_1".to_string()),
            symbol: "BTCUSDT".to_string(),
            status: OrderStatus::New,
            side: crate::traits::OrderSide::Buy,
            order_type: crate::traits::OrderType::Limit,
            time_in_force: crate::traits::TimeInForce::GTC,
            quantity: Size::from_str("1.0").unwrap(),
            price: Some(Price::from_str("50000.0").unwrap()),
            timestamp: 123456789,
        };
        
        // Handle the order
        manager.handle_execution_report(order).await.unwrap();
        
        // Check order exists
        let retrieved_order = manager.get_order(order_id.clone()).await.unwrap();
        assert_eq!(retrieved_order.status, OrderStatus::New);
        
        // Simulate partial fill
        let partial_fill = ExecutionReport {
            order_id: order_id.clone(),
            client_order_id: Some("client_1".to_string()),
            symbol: "BTCUSDT".to_string(),
            status: OrderStatus::PartiallyFilled {
                filled_size: Size::from_str("0.5").unwrap(),
                remaining_size: Size::from_str("0.5").unwrap(),
            },
            side: crate::traits::OrderSide::Buy,
            order_type: crate::traits::OrderType::Limit,
            time_in_force: crate::traits::TimeInForce::GTC,
            quantity: Size::from_str("1.0").unwrap(),
            price: Some(Price::from_str("50000.0").unwrap()),
            timestamp: 123456790,
        };
        
        // Handle the partial fill
        manager.handle_execution_report(partial_fill).await.unwrap();
        
        // Check order was updated
        let updated_order = manager.get_order(order_id.clone()).await.unwrap();
        assert_eq!(updated_order.status, OrderStatus::PartiallyFilled);
        
        // Simulate full fill
        let full_fill = ExecutionReport {
            order_id: order_id.clone(),
            client_order_id: Some("client_1".to_string()),
            symbol: "BTCUSDT".to_string(),
            status: OrderStatus::Filled {
                filled_size: Size::from_str("1.0").unwrap(),
            },
            side: crate::traits::OrderSide::Buy,
            order_type: crate::traits::OrderType::Limit,
            time_in_force: crate::traits::TimeInForce::GTC,
            quantity: Size::from_str("1.0").unwrap(),
            price: Some(Price::from_str("50000.0").unwrap()),
            timestamp: 123456791,
        };
        
        // Handle the full fill
        manager.handle_execution_report(full_fill).await.unwrap();
        
        // Check order was updated
        let final_order = manager.get_order(order_id.clone()).await.unwrap();
        assert_eq!(final_order.status, OrderStatus::Filled);
        
        // Check all orders
        let all_orders = manager.get_all_orders().await.unwrap();
        assert_eq!(all_orders.len(), 1);
        
        // Check open orders
        let open_orders = manager.get_open_orders().await.unwrap();
        assert_eq!(open_orders.len(), 0);
    }

    #[tokio::test]
    async fn test_order_not_found() {
        let mut manager = OrderManagerImpl::new();
        
        // Try to get a non-existent order
        let order_id = OrderId::new("non_existent".to_string());
        let result = manager.get_order(order_id).await;
        
        // Should return an error
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OrderManagerError::OrderNotFound(_)));
    }

    #[tokio::test]
    async fn test_get_orders_by_symbol() {
        let mut manager = OrderManagerImpl::new();
        
        // Create orders for different symbols
        let btc_order_id = OrderId::new("btc_order".to_string());
        let btc_order = ExecutionReport {
            order_id: btc_order_id.clone(),
            symbol: "BTCUSDT".to_string(),
            status: OrderStatus::New,
            side: crate::traits::OrderSide::Buy,
            order_type: crate::traits::OrderType::Limit,
            time_in_force: crate::traits::TimeInForce::GTC,
            quantity: Size::from_str("1.0").unwrap(),
            price: Some(Price::from_str("50000.0").unwrap()),
            timestamp: 123456789,
        };
        
        let eth_order_id = OrderId::new("eth_order".to_string());
        let eth_order = ExecutionReport {
            order_id: eth_order_id.clone(),
            symbol: "ETHUSDT".to_string(),
            status: OrderStatus::New,
            side: crate::traits::OrderSide::Buy,
            order_type: crate::traits::OrderType::Limit,
            time_in_force: crate::traits::TimeInForce::GTC,
            quantity: Size::from_str("2.0").unwrap(),
            price: Some(Price::from_str("3000.0").unwrap()),
            timestamp: 123456789,
        };
        
        // Handle both orders
        manager.handle_execution_report(btc_order).await.unwrap();
        manager.handle_execution_report(eth_order).await.unwrap();
        
        // Get BTC orders
        let btc_orders = manager.get_orders_by_symbol(Some("BTCUSDT")).await.unwrap();
        assert_eq!(btc_orders.len(), 1);
        assert_eq!(btc_orders[0].order_id, btc_order_id);
        
        // Get ETH orders
        let eth_orders = manager.get_orders_by_symbol(Some("ETHUSDT")).await.unwrap();
        assert_eq!(eth_orders.len(), 1);
        assert_eq!(eth_orders[0].order_id, eth_order_id);
        
        // Get all orders
        let all_orders = manager.get_all_orders().await.unwrap();
        assert_eq!(all_orders.len(), 2);
    }

    #[tokio::test]
    async fn test_order_history() {
        let mut manager = OrderManagerImpl::new();
        
        // Create orders with different timestamps
        let order1_id = OrderId::new("order1".to_string());
        let order1 = ExecutionReport {
            order_id: order1_id.clone(),
            symbol: "BTCUSDT".to_string(),
            status: OrderStatus::Filled,
            side: crate::traits::OrderSide::Buy,
            order_type: crate::traits::OrderType::Limit,
            time_in_force: crate::traits::TimeInForce::GTC,
            quantity: Size::from_str("1.0").unwrap(),
            price: Some(Price::from_str("50000.0").unwrap()),
            timestamp: 1000,
        };
        
        let order2_id = OrderId::new("order2".to_string());
        let order2 = ExecutionReport {
            order_id: order2_id.clone(),
            symbol: "BTCUSDT".to_string(),
            status: OrderStatus::Filled,
            side: crate::traits::OrderSide::Sell,
            order_type: crate::traits::OrderType::Limit,
            time_in_force: crate::traits::TimeInForce::GTC,
            quantity: Size::from_str("2.0").unwrap(),
            price: Some(Price::from_str("51000.0").unwrap()),
            timestamp: 2000,
        };
        
        let order3_id = OrderId::new("order3".to_string());
        let order3 = ExecutionReport {
            order_id: order3_id.clone(),
            symbol: "BTCUSDT".to_string(),
            status: OrderStatus::Filled,
            side: crate::traits::OrderSide::Buy,
            order_type: crate::traits::OrderType::Limit,
            time_in_force: crate::traits::TimeInForce::GTC,
            quantity: Size::from_str("1.5").unwrap(),
            price: Some(Price::from_str("50000.0").unwrap()),
            timestamp: 3000,
        };
        
        // Handle all orders
        manager.handle_execution_report(order1).await.unwrap();
        manager.handle_execution_report(order2).await.unwrap();
        manager.handle_execution_report(order3).await.unwrap();
        
        // Get order history with limit
        let history = manager.get_order_history(Some("BTCUSDT"), Some(2)).await.unwrap();
        assert_eq!(history.len(), 2);
        
        // Check orders are sorted by timestamp (newest first)
        assert_eq!(history[0].order_id, order3_id);
        assert_eq!(history[1].order_id, order2_id);
        assert_eq!(history[0].timestamp, 3000);
        assert_eq!(history[1].timestamp, 2000);
        assert_eq!(history[2].timestamp, 1000);
    }
}
