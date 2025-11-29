use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, warn};

use crate::core::events::OrderBookSnapshot;
use crate::exchanges::error::BoxedError;
use crate::traits::{
    Balance, ExecutionReport, MarketDataStream, MarketEvent, NewOrder, OrderId, TradingFees,
};

/// Connection status for an exchange
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// Exchange connection information
#[derive(Debug, Clone)]
pub struct ExchangeConnection {
    /// Exchange name
    pub name: String,
    /// Connection status
    pub status: ConnectionStatus,
    /// Last connected timestamp
    pub last_connected: Option<std::time::Instant>,
    /// Last error (if any)
    pub last_error: Option<String>,
    /// Number of reconnection attempts
    pub reconnect_attempts: u32,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
    /// Reconnection interval in milliseconds
    pub reconnect_interval_ms: u64,
}

impl ExchangeConnection {
    /// Create a new exchange connection
    pub fn new(name: String) -> Self {
        Self {
            name,
            status: ConnectionStatus::Disconnected,
            last_connected: None,
            last_error: None,
            reconnect_attempts: 0,
            max_reconnect_attempts: 5,
            reconnect_interval_ms: 5000,
        }
    }

    /// Set the connection status
    pub fn set_status(&mut self, status: ConnectionStatus) {
        self.status = status.clone();

        match status {
            ConnectionStatus::Connected => {
                self.last_connected = Some(std::time::Instant::now());
                self.reconnect_attempts = 0;
                self.last_error = None;
            }
            ConnectionStatus::Failed => {
                self.reconnect_attempts += 1;
            }
            _ => {}
        }
    }

    /// Set the last error
    pub fn set_error(&mut self, error: String) {
        self.last_error = Some(error);
        self.status = ConnectionStatus::Failed;
        self.reconnect_attempts += 1;
    }

    /// Check if the connection should be reconnected
    pub fn should_reconnect(&self) -> bool {
        self.status == ConnectionStatus::Failed || self.status == ConnectionStatus::Disconnected
    }

    /// Check if the maximum reconnection attempts have been reached
    pub fn max_reconnects_reached(&self) -> bool {
        self.reconnect_attempts >= self.max_reconnect_attempts
    }
}

/// Connection manager for multiple exchanges
pub struct ConnectionManager {
    /// Exchange connections
    connections: Arc<RwLock<HashMap<String, ExchangeConnection>>>,
    /// Exchange adapters
    adapters: Arc<RwLock<HashMap<String, Arc<dyn ExchangeAdapter + Send + Sync>>>>,
    /// Market data streams
    streams: Arc<
        RwLock<HashMap<String, Arc<Mutex<dyn MarketDataStream<Error = BoxedError> + Send + Sync>>>>,
    >,
    /// Event handlers
    event_handlers: Arc<Mutex<Vec<Box<dyn Fn(MarketEvent) + Send + Sync>>>>,
    /// Shutdown flag
    shutdown: Arc<RwLock<bool>>,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            adapters: Arc::new(RwLock::new(HashMap::new())),
            streams: Arc::new(RwLock::new(HashMap::new())),
            event_handlers: Arc::new(Mutex::new(Vec::new())),
            shutdown: Arc::new(RwLock::new(false)),
        }
    }

    /// Add an exchange adapter
    pub async fn add_exchange(
        &self,
        name: String,
        adapter: Arc<dyn ExchangeAdapter + Send + Sync>,
    ) {
        let mut connections = self.connections.write().await;
        connections.insert(name.clone(), ExchangeConnection::new(name.clone()));

        let mut adapters = self.adapters.write().await;
        adapters.insert(name.clone(), adapter);

        info!("Added exchange: {}", name);
    }

    /// Connect to an exchange
    pub async fn connect_exchange(
        &self,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut connections = self.connections.write().await;

        if let Some(connection) = connections.get_mut(name) {
            connection.set_status(ConnectionStatus::Connecting);

            let adapters = self.adapters.read().await;
            if let Some(adapter) = adapters.get(name) {
                // Connect to the exchange
                match adapter.connect().await {
                    Ok(_) => {
                        connection.set_status(ConnectionStatus::Connected);
                        info!("Connected to exchange: {}", name);

                        // Start market data stream
                        self.start_market_data_stream(name).await?;

                        Ok(())
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to connect to {}: {}", name, e);
                        connection.set_error(error_msg.clone());
                        error!("{}", error_msg);
                        Err(e)
                    }
                }
            } else {
                let error_msg = format!("Exchange adapter not found: {}", name);
                connection.set_error(error_msg.clone());
                error!("{}", error_msg);
                Err(error_msg.into())
            }
        } else {
            let error_msg = format!("Exchange connection not found: {}", name);
            error!("{}", error_msg);
            Err(error_msg.into())
        }
    }

    /// Disconnect from an exchange
    pub async fn disconnect_exchange(
        &self,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut connections = self.connections.write().await;

        if let Some(connection) = connections.get_mut(name) {
            connection.set_status(ConnectionStatus::Disconnected);

            let adapters = self.adapters.read().await;
            if let Some(adapter) = adapters.get(name) {
                // Disconnect from the exchange
                match adapter.disconnect().await {
                    Ok(_) => {
                        info!("Disconnected from exchange: {}", name);

                        // Stop market data stream
                        self.stop_market_data_stream(name).await?;

                        Ok(())
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to disconnect from {}: {}", name, e);
                        connection.set_error(error_msg.clone());
                        error!("{}", error_msg);
                        Err(e)
                    }
                }
            } else {
                let error_msg = format!("Exchange adapter not found: {}", name);
                connection.set_error(error_msg.clone());
                error!("{}", error_msg);
                Err(error_msg.into())
            }
        } else {
            let error_msg = format!("Exchange connection not found: {}", name);
            error!("{}", error_msg);
            Err(error_msg.into())
        }
    }

    /// Connect to all exchanges
    pub async fn connect_all(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let connections = self.connections.read().await;
        let exchange_names: Vec<String> = connections.keys().cloned().collect();
        drop(connections);

        for name in exchange_names {
            if let Err(e) = self.connect_exchange(&name).await {
                warn!("Failed to connect to {}: {}", name, e);
            }
        }

        Ok(())
    }

    /// Disconnect from all exchanges
    pub async fn disconnect_all(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let connections = self.connections.read().await;
        let exchange_names: Vec<String> = connections.keys().cloned().collect();
        drop(connections);

        for name in exchange_names {
            if let Err(e) = self.disconnect_exchange(&name).await {
                warn!("Failed to disconnect from {}: {}", name, e);
            }
        }

        Ok(())
    }

    /// Get connection status for an exchange
    pub async fn get_connection_status(&self, name: &str) -> Option<ConnectionStatus> {
        let connections = self.connections.read().await;
        connections.get(name).map(|c| c.status.clone())
    }

    /// Get all connection statuses
    pub async fn get_all_connection_statuses(&self) -> HashMap<String, ConnectionStatus> {
        let connections = self.connections.read().await;
        connections
            .iter()
            .map(|(name, conn)| (name.clone(), conn.status.clone()))
            .collect()
    }

    /// Start market data stream for an exchange
    async fn start_market_data_stream(
        &self,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let adapters = self.adapters.read().await;
        if let Some(adapter) = adapters.get(name) {
            let stream = adapter.get_market_data_stream().await?;

            let mut streams = self.streams.write().await;
            streams.insert(name.to_string(), stream);

            // Start processing market data in the background
            let name_clone = name.to_string();
            let streams_clone = self.streams.clone();
            let event_handlers = self.event_handlers.clone();
            let shutdown = self.shutdown.clone();

            tokio::spawn(async move {
                loop {
                    // Check if shutdown is requested
                    {
                        let shutdown_flag = shutdown.read().await;
                        if *shutdown_flag {
                            break;
                        }
                    }

                    // Get the stream
                    let streams_guard = streams_clone.read().await;
                    if let Some(stream) = streams_guard.get(&name_clone) {
                        let mut stream_guard = stream.lock().await;

                        // Process next market event
                        match stream_guard.next().await {
                            Some(Ok(event)) => {
                                // Handle the event
                                let handlers = event_handlers.lock().await;
                                for handler in handlers.iter() {
                                    handler(event.clone());
                                }
                            }
                            Some(Err(e)) => {
                                error!("Error in market data stream for {}: {}", name_clone, e);
                                // In a real implementation, you might want to reconnect here
                                break;
                            }
                            None => {
                                debug!("Market data stream for {} ended", name_clone);
                                break;
                            }
                        }
                    } else {
                        break;
                    }
                }
            });

            info!("Started market data stream for exchange: {}", name);
            Ok(())
        } else {
            let error_msg = format!("Exchange adapter not found: {}", name);
            error!("{}", error_msg);
            Err(error_msg.into())
        }
    }

    /// Stop market data stream for an exchange
    async fn stop_market_data_stream(
        &self,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut streams = self.streams.write().await;
        if streams.remove(name).is_some() {
            info!("Stopped market data stream for exchange: {}", name);
            Ok(())
        } else {
            let error_msg = format!("Market data stream not found for exchange: {}", name);
            error!("{}", error_msg);
            Err(error_msg.into())
        }
    }

    /// Add an event handler
    pub async fn add_event_handler<F>(&self, handler: F)
    where
        F: Fn(MarketEvent) + Send + Sync + 'static,
    {
        let mut handlers = self.event_handlers.lock().await;
        handlers.push(Box::new(handler));
    }

    /// Place an order on a specific exchange
    pub async fn place_order(
        &self,
        exchange: &str,
        order: NewOrder,
    ) -> Result<OrderId, Box<dyn std::error::Error + Send + Sync>> {
        let adapters = self.adapters.read().await;
        if let Some(adapter) = adapters.get(exchange) {
            adapter.place_order(order).await
        } else {
            let error_msg = format!("Exchange adapter not found: {}", exchange);
            error!("{}", error_msg);
            Err(error_msg.into())
        }
    }

    /// Cancel an order on a specific exchange
    pub async fn cancel_order(
        &self,
        exchange: &str,
        order_id: OrderId,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let adapters = self.adapters.read().await;
        if let Some(adapter) = adapters.get(exchange) {
            adapter.cancel_order(order_id).await
        } else {
            let error_msg = format!("Exchange adapter not found: {}", exchange);
            error!("{}", error_msg);
            Err(error_msg.into())
        }
    }

    /// Get order status from a specific exchange
    pub async fn get_order_status(
        &self,
        exchange: &str,
        order_id: OrderId,
    ) -> Result<ExecutionReport, Box<dyn std::error::Error + Send + Sync>> {
        let adapters = self.adapters.read().await;
        if let Some(adapter) = adapters.get(exchange) {
            adapter.get_order_status(order_id).await
        } else {
            let error_msg = format!("Exchange adapter not found: {}", exchange);
            error!("{}", error_msg);
            Err(error_msg.into())
        }
    }

    /// Get balances from a specific exchange
    pub async fn get_balances(
        &self,
        exchange: &str,
    ) -> Result<Vec<Balance>, Box<dyn std::error::Error + Send + Sync>> {
        let adapters = self.adapters.read().await;
        if let Some(adapter) = adapters.get(exchange) {
            adapter.get_balances().await
        } else {
            let error_msg = format!("Exchange adapter not found: {}", exchange);
            error!("{}", error_msg);
            Err(error_msg.into())
        }
    }

    /// Get open orders from a specific exchange
    pub async fn get_open_orders(
        &self,
        exchange: &str,
        symbol: Option<&str>,
    ) -> Result<Vec<ExecutionReport>, Box<dyn std::error::Error + Send + Sync>> {
        let adapters = self.adapters.read().await;
        if let Some(adapter) = adapters.get(exchange) {
            adapter.get_open_orders(symbol).await
        } else {
            let error_msg = format!("Exchange adapter not found: {}", exchange);
            error!("{}", error_msg);
            Err(error_msg.into())
        }
    }

    /// Get order book snapshot from a specific exchange
    pub async fn get_order_book(
        &self,
        exchange: &str,
        symbol: &str,
        limit: u32,
    ) -> Result<OrderBookSnapshot, Box<dyn std::error::Error + Send + Sync>> {
        let adapters = self.adapters.read().await;
        if let Some(adapter) = adapters.get(exchange) {
            adapter.get_order_book(symbol, limit).await
        } else {
            let error_msg = format!("Exchange adapter not found: {}", exchange);
            error!("{}", error_msg);
            Err(error_msg.into())
        }
    }

    /// Get trading fees from a specific exchange
    pub async fn get_trading_fees(
        &self,
        exchange: &str,
        symbol: &str,
    ) -> Result<TradingFees, Box<dyn std::error::Error + Send + Sync>> {
        let adapters = self.adapters.read().await;
        if let Some(adapter) = adapters.get(exchange) {
            adapter.get_trading_fees(symbol).await
        } else {
            let error_msg = format!("Exchange adapter not found: {}", exchange);
            error!("{}", error_msg);
            Err(error_msg.into())
        }
    }

    /// Start the connection manager
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting connection manager");

        // Connect to all exchanges
        self.connect_all().await?;

        // Start reconnection task
        self.start_reconnection_task().await?;

        Ok(())
    }

    /// Stop the connection manager
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Stopping connection manager");

        // Set shutdown flag
        {
            let mut shutdown = self.shutdown.write().await;
            *shutdown = true;
        }

        // Disconnect from all exchanges
        self.disconnect_all().await?;

        Ok(())
    }

    /// Start the reconnection task
    async fn start_reconnection_task(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let connections = self.connections.clone();
        let shutdown = self.shutdown.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));

            loop {
                // Check if shutdown is requested
                {
                    let shutdown_flag = shutdown.read().await;
                    if *shutdown_flag {
                        break;
                    }
                }

                interval.tick().await;

                // Check for disconnected exchanges
                let mut connections_guard = connections.write().await;
                for (name, connection) in connections_guard.iter_mut() {
                    if connection.should_reconnect() && !connection.max_reconnects_reached() {
                        info!("Attempting to reconnect to exchange: {}", name);
                        connection.set_status(ConnectionStatus::Reconnecting);

                        // In a real implementation, you would attempt to reconnect here
                        // For now, we'll just update the status
                        connection.set_status(ConnectionStatus::Failed);
                    }
                }
            }
        });

        Ok(())
    }
}

/// Exchange adapter trait that all exchange adapters must implement
#[async_trait]
pub trait ExchangeAdapter: Send + Sync {
    /// Connect to the exchange
    async fn connect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Disconnect from the exchange
    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Get the market data stream
    async fn get_market_data_stream(
        &self,
    ) -> Result<
        Arc<Mutex<dyn MarketDataStream<Error = BoxedError> + Send + Sync>>,
        Box<dyn std::error::Error + Send + Sync>,
    >;

    /// Place an order
    async fn place_order(
        &self,
        order: NewOrder,
    ) -> Result<OrderId, Box<dyn std::error::Error + Send + Sync>>;

    /// Cancel an order
    async fn cancel_order(
        &self,
        order_id: OrderId,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Get order status
    async fn get_order_status(
        &self,
        order_id: OrderId,
    ) -> Result<ExecutionReport, Box<dyn std::error::Error + Send + Sync>>;

    /// Get balances
    async fn get_balances(&self) -> Result<Vec<Balance>, Box<dyn std::error::Error + Send + Sync>>;

    /// Get open orders
    async fn get_open_orders(
        &self,
        symbol: Option<&str>,
    ) -> Result<Vec<ExecutionReport>, Box<dyn std::error::Error + Send + Sync>>;

    /// Get order book
    async fn get_order_book(
        &self,
        symbol: &str,
        limit: u32,
    ) -> Result<OrderBookSnapshot, Box<dyn std::error::Error + Send + Sync>>;

    /// Get trading fees
    async fn get_trading_fees(
        &self,
        symbol: &str,
    ) -> Result<TradingFees, Box<dyn std::error::Error + Send + Sync>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exchanges::mock::MockExchangeAdapter;
    use std::sync::Arc;

    #[test]
    fn test_exchange_connection_new() {
        let connection = ExchangeConnection::new("test".to_string());
        assert_eq!(connection.name, "test");
        assert_eq!(connection.status, ConnectionStatus::Disconnected);
        assert!(connection.last_connected.is_none());
        assert!(connection.last_error.is_none());
        assert_eq!(connection.reconnect_attempts, 0);
        assert_eq!(connection.max_reconnect_attempts, 5);
        assert_eq!(connection.reconnect_interval_ms, 5000);
    }

    #[test]
    fn test_exchange_connection_set_status() {
        let mut connection = ExchangeConnection::new("test".to_string());

        connection.set_status(ConnectionStatus::Connected);
        assert_eq!(connection.status, ConnectionStatus::Connected);
        assert!(connection.last_connected.is_some());
        assert_eq!(connection.reconnect_attempts, 0);
        assert!(connection.last_error.is_none());

        connection.set_status(ConnectionStatus::Failed);
        assert_eq!(connection.status, ConnectionStatus::Failed);
        assert_eq!(connection.reconnect_attempts, 1);
    }

    #[test]
    fn test_exchange_connection_set_error() {
        let mut connection = ExchangeConnection::new("test".to_string());

        connection.set_error("Test error".to_string());
        assert_eq!(connection.status, ConnectionStatus::Failed);
        assert_eq!(connection.last_error, Some("Test error".to_string()));
        assert_eq!(connection.reconnect_attempts, 1);
    }

    #[test]
    fn test_exchange_connection_should_reconnect() {
        let mut connection = ExchangeConnection::new("test".to_string());

        assert!(connection.should_reconnect());

        connection.set_status(ConnectionStatus::Connected);
        assert!(!connection.should_reconnect());

        connection.set_status(ConnectionStatus::Failed);
        assert!(connection.should_reconnect());
    }

    #[test]
    fn test_exchange_connection_max_reconnects_reached() {
        let mut connection = ExchangeConnection::new("test".to_string());

        assert!(!connection.max_reconnects_reached());

        connection.reconnect_attempts = 5;
        assert!(connection.max_reconnects_reached());
    }

    #[tokio::test]
    async fn test_connection_manager_new() {
        let manager = ConnectionManager::new();

        // Verify manager was created
        let connections = manager.connections.read().await;
        assert!(connections.is_empty());

        let adapters = manager.adapters.read().await;
        assert!(adapters.is_empty());

        let streams = manager.streams.read().await;
        assert!(streams.is_empty());

        let handlers = manager.event_handlers.lock().await;
        assert!(handlers.is_empty());

        let shutdown = manager.shutdown.read().await;
        assert!(!*shutdown);
    }

    #[tokio::test]
    async fn test_connection_manager_add_exchange() {
        let manager = ConnectionManager::new();
        let mock_adapter = Arc::new(MockExchangeAdapter::new("test"));

        manager.add_exchange("test".to_string(), mock_adapter).await;

        let connections = manager.connections.read().await;
        assert!(connections.contains_key("test"));

        let adapters = manager.adapters.read().await;
        assert!(adapters.contains_key("test"));
    }

    #[tokio::test]
    async fn test_connection_manager_get_connection_status() {
        let manager = ConnectionManager::new();
        let mock_adapter = Arc::new(MockExchangeAdapter::new("test"));

        manager.add_exchange("test".to_string(), mock_adapter).await;

        let status = manager.get_connection_status("test").await;
        assert_eq!(status, Some(ConnectionStatus::Disconnected));

        let status = manager.get_connection_status("nonexistent").await;
        assert_eq!(status, None);
    }

    #[tokio::test]
    async fn test_connection_manager_get_all_connection_statuses() {
        let manager = ConnectionManager::new();
        let mock_adapter1 = Arc::new(MockExchangeAdapter::new("test1"));
        let mock_adapter2 = Arc::new(MockExchangeAdapter::new("test2"));

        manager
            .add_exchange("test1".to_string(), mock_adapter1)
            .await;
        manager
            .add_exchange("test2".to_string(), mock_adapter2)
            .await;

        let statuses = manager.get_all_connection_statuses().await;
        assert_eq!(statuses.len(), 2);
        assert_eq!(statuses.get("test1"), Some(&ConnectionStatus::Disconnected));
        assert_eq!(statuses.get("test2"), Some(&ConnectionStatus::Disconnected));
    }

    #[tokio::test]
    async fn test_connection_manager_add_event_handler() {
        let manager = ConnectionManager::new();

        manager
            .add_event_handler(|_event| {
                // Handle event
            })
            .await;

        let handlers = manager.event_handlers.lock().await;
        assert_eq!(handlers.len(), 1);
    }
}
