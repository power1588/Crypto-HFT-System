use crypto_hft::exchanges::{
    ConnectionManager, ExchangeAdapter, ConnectionStatus,
    MockExchangeAdapter, GateAdapter, BybitAdapter, HyperliquidAdapter, DydxAdapter, AsterAdapter
};
use crypto_hft::traits::{NewOrder, OrderSide, OrderType, TimeInForce};
use crypto_hft::types::{Price, Size};
use std::sync::Arc;
use tokio;

#[tokio::test]
async fn test_multi_exchange_connection() {
    let manager = ConnectionManager::new();
    
    // Add multiple exchanges
    let gate_adapter = Arc::new(MockExchangeAdapter::new("gate"));
    let bybit_adapter = Arc::new(MockExchangeAdapter::new("bybit"));
    let hyperliquid_adapter = Arc::new(MockExchangeAdapter::new("hyperliquid"));
    
    manager.add_exchange("gate".to_string(), gate_adapter).await;
    manager.add_exchange("bybit".to_string(), bybit_adapter).await;
    manager.add_exchange("hyperliquid".to_string(), hyperliquid_adapter).await;
    
    // Connect all exchanges
    let result = manager.connect_all().await;
    assert!(result.is_ok());
    
    // Verify all are connected
    let statuses = manager.get_all_connection_statuses().await;
    assert_eq!(statuses.len(), 3);
}

#[tokio::test]
async fn test_multi_exchange_order_placement() {
    let manager = ConnectionManager::new();
    
    let gate_adapter = Arc::new(MockExchangeAdapter::new("gate"));
    let bybit_adapter = Arc::new(MockExchangeAdapter::new("bybit"));
    
    manager.add_exchange("gate".to_string(), gate_adapter).await;
    manager.add_exchange("bybit".to_string(), bybit_adapter).await;
    
    manager.connect_all().await.unwrap();
    
    // Place order on Gate
    let order1 = NewOrder {
        symbol: "BTCUSDT".to_string(),
        side: OrderSide::Buy,
        order_type: OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        quantity: Size::from_str("0.1").unwrap(),
        price: Some(Price::from_str("50000").unwrap()),
        client_order_id: Some("gate_order_1".to_string()),
    };
    
    let result1 = manager.place_order("gate", order1).await;
    assert!(result1.is_ok());
    
    // Place order on Bybit
    let order2 = NewOrder {
        symbol: "BTCUSDT".to_string(),
        side: OrderSide::Sell,
        order_type: OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        quantity: Size::from_str("0.1").unwrap(),
        price: Some(Price::from_str("51000").unwrap()),
        client_order_id: Some("bybit_order_1".to_string()),
    };
    
    let result2 = manager.place_order("bybit", order2).await;
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_multi_exchange_balance_retrieval() {
    let manager = ConnectionManager::new();
    
    let exchanges = vec!["gate", "bybit", "hyperliquid", "dydx", "aster"];
    for exchange in &exchanges {
        let adapter = Arc::new(MockExchangeAdapter::new(exchange));
        manager.add_exchange(exchange.to_string(), adapter).await;
    }
    
    manager.connect_all().await.unwrap();
    
    // Get balances from each exchange
    for exchange in &exchanges {
        let result = manager.get_balances(exchange).await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_multi_exchange_order_book_retrieval() {
    let manager = ConnectionManager::new();
    
    let gate_adapter = Arc::new(MockExchangeAdapter::new("gate"));
    let bybit_adapter = Arc::new(MockExchangeAdapter::new("bybit"));
    
    manager.add_exchange("gate".to_string(), gate_adapter).await;
    manager.add_exchange("bybit".to_string(), bybit_adapter).await;
    
    manager.connect_all().await.unwrap();
    
    // Get order book from Gate
    let result1 = manager.get_order_book("gate", "BTCUSDT", 10).await;
    assert!(result1.is_ok());
    
    // Get order book from Bybit
    let result2 = manager.get_order_book("bybit", "BTCUSDT", 10).await;
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_multi_exchange_disconnect_all() {
    let manager = ConnectionManager::new();
    
    let exchanges = vec!["gate", "bybit", "hyperliquid"];
    for exchange in &exchanges {
        let adapter = Arc::new(MockExchangeAdapter::new(exchange));
        manager.add_exchange(exchange.to_string(), adapter).await;
    }
    
    manager.connect_all().await.unwrap();
    
    // Disconnect all
    let result = manager.disconnect_all().await;
    assert!(result.is_ok());
    
    // Verify all are disconnected
    let statuses = manager.get_all_connection_statuses().await;
    for (_, status) in statuses {
        assert_eq!(status, ConnectionStatus::Disconnected);
    }
}

#[tokio::test]
async fn test_multi_exchange_individual_operations() {
    let manager = ConnectionManager::new();
    
    let gate_adapter = Arc::new(MockExchangeAdapter::new("gate"));
    let bybit_adapter = Arc::new(MockExchangeAdapter::new("bybit"));
    
    manager.add_exchange("gate".to_string(), gate_adapter).await;
    manager.add_exchange("bybit".to_string(), bybit_adapter).await;
    
    // Connect Gate individually
    manager.connect_exchange("gate").await.unwrap();
    let status = manager.get_connection_status("gate").await;
    assert_eq!(status, Some(ConnectionStatus::Connected));
    
    // Bybit should still be disconnected
    let status = manager.get_connection_status("bybit").await;
    assert_eq!(status, Some(ConnectionStatus::Disconnected));
    
    // Connect Bybit
    manager.connect_exchange("bybit").await.unwrap();
    let status = manager.get_connection_status("bybit").await;
    assert_eq!(status, Some(ConnectionStatus::Connected));
    
    // Disconnect Gate individually
    manager.disconnect_exchange("gate").await.unwrap();
    let status = manager.get_connection_status("gate").await;
    assert_eq!(status, Some(ConnectionStatus::Disconnected));
    
    // Bybit should still be connected
    let status = manager.get_connection_status("bybit").await;
    assert_eq!(status, Some(ConnectionStatus::Connected));
}

#[tokio::test]
async fn test_multi_exchange_trading_fees() {
    let manager = ConnectionManager::new();
    
    let exchanges = vec!["gate", "bybit", "hyperliquid", "dydx", "aster"];
    for exchange in &exchanges {
        let adapter = Arc::new(MockExchangeAdapter::new(exchange));
        manager.add_exchange(exchange.to_string(), adapter).await;
    }
    
    manager.connect_all().await.unwrap();
    
    // Get trading fees from each exchange
    for exchange in &exchanges {
        let result = manager.get_trading_fees(exchange, "BTCUSDT").await;
        assert!(result.is_ok());
    }
}

