use crate::traits::{MarketDataStream, MarketEvent, ExecutionClient, OrderManager};
use crate::types::{Price, Size};
use std::collections::HashMap;

/// Mock exchange adapter for testing
pub struct MockExchangeAdapter {
    /// Market data stream
    market_data_stream: Option<Box<dyn MarketDataStream>>,
    /// Execution client
    execution_client: Option<Box<dyn ExecutionClient>>,
    /// Order manager
    order_manager: Option<Box<dyn OrderManager>>,
}

impl MockExchangeAdapter {
    /// Create a new mock exchange adapter
    pub fn new() -> Self {
        Self {
            market_data_stream: None,
            execution_client: None,
            order_manager: None,
        }
    }

    /// Set market data stream
    pub fn with_market_data_stream(
        mut self,
        stream: Box<dyn MarketDataStream>,
    ) -> Self {
        self.market_data_stream = Some(stream);
        self
    }

    /// Set execution client
    pub fn with_execution_client(
        mut self,
        client: Box<dyn ExecutionClient>,
    ) -> Self {
        self.execution_client = Some(client);
        self
    }

    /// Set order manager
    pub fn with_order_manager(
        mut self,
        manager: Box<dyn OrderManager>,
    ) -> Self {
        self.order_manager = Some(manager);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Price, Size};

    struct MockMarketDataStream {
        events: Vec<MarketEvent>,
        connected: bool,
        symbol: String,
    }

    struct MockExecutionClient {
        orders: Vec<crate::traits::ExecutionReport>,
        connected: bool,
    }

    struct MockOrderManager {
        orders: Vec<crate::traits::ExecutionReport>,
        connected: bool,
    }

    impl MockMarketDataStream {
        fn new(symbol: String) -> Self {
            Self {
                events: Vec::new(),
                connected: false,
                symbol,
            }
        }

        async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            self.connected = true;
            Ok(())
        }

        async fn next(&mut self) -> Option<MarketEvent> {
            if self.events.is_empty() {
                None
            } else {
                let event = self.events.remove(0);
                self.events.push(event);
                Some(event)
            }
        }

        async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            self.connected = false;
            Ok(())
        }
    }

    impl MockExecutionClient {
        fn new() -> Self {
            Self {
                orders: Vec::new(),
                connected: false,
            }
        }

        async fn submit_order(&mut self, order: crate::traits::NewOrder) -> Result<crate::traits::OrderId, Box<dyn std::error::Error>> {
            let order_id = order.order_id.clone();
            let report = crate::traits::ExecutionReport {
                order_id: order_id.clone(),
                client_order_id: Some(order.client_order_id.clone()),
                symbol: order.symbol.clone(),
                status: crate::traits::OrderStatus::New,
                side: order.side,
                order_type: order.order_type,
                time_in_force: order.time_in_force,
                quantity: order.quantity,
                price: order.price,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .as_millis() as u64,
            };
            
            self.orders.push(report);
            Ok(order_id)
        }

        async fn get_order(&self, order_id: crate::traits::OrderId) -> Result<crate::traits::ExecutionReport, Box<dyn std::error::Error>> {
            self.orders
                .iter()
                .find(|order| order.order_id == order_id)
                .cloned()
                .ok_or_else(|| Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Order not found: {}", order_id.as_str())
                )))
        }

        async fn get_all_orders(&self) -> Result<Vec<crate::traits::ExecutionReport>, Box<dyn std::error::Error>> {
            Ok(self.orders.clone())
        }

        async fn get_orders_by_symbol(&self, symbol: &str) -> Result<Vec<crate::traits::ExecutionReport>, Box<dyn std::error::Error>> {
            Ok(self
                .orders
                .iter()
                .filter(|order| order.symbol == symbol)
                .cloned()
                .collect()
        }
    }

    impl MockOrderManager {
        fn new() -> Self {
            Self {
                orders: Vec::new(),
                connected: false,
            }
        }

        async fn handle_execution_report(&mut self, report: crate::traits::ExecutionReport) -> Result<(), Box<dyn std::error::Error>> {
            // Just store the report for testing
            self.orders.push(report);
            Ok(())
        }

        async fn get_order(&self, order_id: crate::traits::OrderId) -> Result<crate::traits::ExecutionReport, Box<dyn std::error::Error>> {
            self.orders
                .iter()
                .find(|order| order.order_id == order_id)
                .cloned()
                .ok_or_else(|| Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Order not found: {}", order_id.as_str())
                )))
        }

        async fn get_all_orders(&self) -> Result<Vec<crate::traits::ExecutionReport>, Box<dyn std::error::Error>> {
            Ok(self.orders.clone())
        }

        async fn get_orders_by_symbol(&self, symbol: &str) -> Result<Vec<crate::traits::ExecutionReport>, Box<dyn std::error::Error>> {
            Ok(self
                .orders
                .iter()
                .filter(|order| order.symbol == symbol)
                .cloned()
                .collect()
        }

        async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            self.connected = true;
            Ok(())
        }

        async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            self.connected = false;
            Ok(())
        }
    }

    #[test]
    fn test_mock_exchange_adapter() {
        let mut adapter = MockExchangeAdapter::new();
        
        // Test with mock implementations
        let market_stream = MockMarketDataStream::new("BTCUSDT".to_string());
        let execution_client = MockExecutionClient::new();
        let order_manager = MockOrderManager::new();
        
        adapter = adapter.with_market_data_stream(Box::new(market_stream));
        adapter = adapter.with_execution_client(Box::new(execution_client));
        adapter = adapter.with_order_manager(Box::new(order_manager));
        
        // Test connection
        assert!(adapter.market_data_stream.is_some());
        assert!(adapter.execution_client.is_some());
        assert!(adapter.order_manager.is_some());
        
        // Test that mock implementations are properly set
        let market_stream = adapter.market_data_stream.unwrap();
        let execution_client = adapter.execution_client.unwrap();
        let order_manager = adapter.order_manager.unwrap();
        
        assert!(market_stream.symbol == "BTCUSDT");
        assert!(execution_client.orders.is_empty());
        assert!(order_manager.orders.is_empty());
    }
}
