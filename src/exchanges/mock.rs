use crate::core::events::OrderBookSnapshot;
use crate::exchanges::connection_manager::ExchangeAdapter;
use crate::exchanges::error::BoxedError;
use crate::traits::{
    Balance, ExecutionReport, MarketDataStream, MarketEvent, NewOrder, OrderId, TradingFees,
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Mock exchange adapter for testing
#[allow(dead_code)]
pub struct MockExchangeAdapter {
    /// Exchange name
    name: String,
    /// Connected status
    connected: Arc<RwLock<bool>>,
}

impl MockExchangeAdapter {
    /// Create a new mock exchange adapter
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            connected: Arc::new(RwLock::new(false)),
        }
    }
}

/// Mock WebSocket stream
pub struct MockWebSocket {
    connected: Arc<RwLock<bool>>,
}

impl MockWebSocket {
    pub fn new(connected: Arc<RwLock<bool>>) -> Self {
        Self { connected }
    }
}

#[async_trait]
impl MarketDataStream for MockWebSocket {
    type Error = BoxedError;

    async fn subscribe(&mut self, _symbols: &[&str]) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn unsubscribe(&mut self, _symbols: &[&str]) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn next(&mut self) -> Option<Result<MarketEvent, Self::Error>> {
        None
    }

    fn is_connected(&self) -> bool {
        self.connected
            .try_read()
            .map(|guard| *guard)
            .unwrap_or(false)
    }

    fn last_update(&self, _symbol: &str) -> Option<u64> {
        None
    }
}

#[async_trait]
impl ExchangeAdapter for MockExchangeAdapter {
    async fn connect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut connected = self.connected.write().await;
        *connected = true;
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut connected = self.connected.write().await;
        *connected = false;
        Ok(())
    }

    async fn get_market_data_stream(
        &self,
    ) -> Result<
        Arc<Mutex<dyn MarketDataStream<Error = BoxedError> + Send + Sync>>,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        Ok(Arc::new(Mutex::new(MockWebSocket::new(
            self.connected.clone(),
        ))))
    }

    async fn place_order(
        &self,
        _order: NewOrder,
    ) -> Result<OrderId, Box<dyn std::error::Error + Send + Sync>> {
        Ok("mock_order_123".to_string())
    }

    async fn cancel_order(
        &self,
        _order_id: OrderId,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    async fn get_order_status(
        &self,
        _order_id: OrderId,
    ) -> Result<ExecutionReport, Box<dyn std::error::Error + Send + Sync>> {
        Err("Not implemented".into())
    }

    async fn get_balances(&self) -> Result<Vec<Balance>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(vec![])
    }

    async fn get_open_orders(
        &self,
        _symbol: Option<&str>,
    ) -> Result<Vec<ExecutionReport>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(vec![])
    }

    async fn get_order_book(
        &self,
        symbol: &str,
        _limit: u32,
    ) -> Result<OrderBookSnapshot, Box<dyn std::error::Error + Send + Sync>> {
        Ok(OrderBookSnapshot::new(symbol, "mock", vec![], vec![], 0))
    }

    async fn get_trading_fees(
        &self,
        symbol: &str,
    ) -> Result<TradingFees, Box<dyn std::error::Error + Send + Sync>> {
        Ok(TradingFees {
            symbol: symbol.to_string(),
            maker_fee: rust_decimal::Decimal::new(1, 4), // 0.0001
            taker_fee: rust_decimal::Decimal::new(1, 4), // 0.0001
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_exchange_adapter() {
        let adapter = MockExchangeAdapter::new("test");
        assert_eq!(adapter.name, "test");

        adapter.connect().await.unwrap();
        let connected = adapter.connected.read().await;
        assert!(*connected);
    }

    #[tokio::test]
    async fn test_mock_websocket_is_connected() {
        let connected = Arc::new(RwLock::new(false));
        let ws = MockWebSocket::new(connected.clone());

        assert!(!ws.is_connected());

        {
            let mut guard = connected.write().await;
            *guard = true;
        }

        assert!(ws.is_connected());
    }
}
