use crypto_hft::exchanges::{ConnectionManager, ExchangeAdapter, ConnectionStatus, MockExchangeAdapter};
use crypto_hft::traits::{NewOrder, OrderId};
use std::sync::Arc;
use tokio;

#[tokio::test]
async fn test_connection_manager_add_multiple_exchanges() {
    let manager = ConnectionManager::new();
    
    let mock_adapter1 = Arc::new(MockExchangeAdapter::new("gate"));
    let mock_adapter2 = Arc::new(MockExchangeAdapter::new("bybit"));
    let mock_adapter3 = Arc::new(MockExchangeAdapter::new("hyperliquid"));
    
    manager.add_exchange("gate".to_string(), mock_adapter1).await;
    manager.add_exchange("bybit".to_string(), mock_adapter2).await;
    manager.add_exchange("hyperliquid".to_string(), mock_adapter3).await;
    
    let connections = manager.connections.read().await;
    assert_eq!(connections.len(), 3);
    assert!(connections.contains_key("gate"));
    assert!(connections.contains_key("bybit"));
    assert!(connections.contains_key("hyperliquid"));
}

#[tokio::test]
async fn test_connection_manager_connect_all() {
    let manager = ConnectionManager::new();
    
    let mock_adapter1 = Arc::new(MockExchangeAdapter::new("gate"));
    let mock_adapter2 = Arc::new(MockExchangeAdapter::new("bybit"));
    
    manager.add_exchange("gate".to_string(), mock_adapter1).await;
    manager.add_exchange("bybit".to_string(), mock_adapter2).await;
    
    // Connect all exchanges
    let result = manager.connect_all().await;
    assert!(result.is_ok());
    
    // Check connection statuses
    let statuses = manager.get_all_connection_statuses().await;
    assert_eq!(statuses.len(), 2);
}

#[tokio::test]
async fn test_connection_manager_disconnect_all() {
    let manager = ConnectionManager::new();
    
    let mock_adapter1 = Arc::new(MockExchangeAdapter::new("gate"));
    let mock_adapter2 = Arc::new(MockExchangeAdapter::new("bybit"));
    
    manager.add_exchange("gate".to_string(), mock_adapter1).await;
    manager.add_exchange("bybit".to_string(), mock_adapter2).await;
    
    manager.connect_all().await.unwrap();
    manager.disconnect_all().await.unwrap();
    
    let statuses = manager.get_all_connection_statuses().await;
    for (_, status) in statuses {
        assert_eq!(status, ConnectionStatus::Disconnected);
    }
}

#[tokio::test]
async fn test_connection_manager_individual_connect_disconnect() {
    let manager = ConnectionManager::new();
    
    let mock_adapter = Arc::new(MockExchangeAdapter::new("gate"));
    manager.add_exchange("gate".to_string(), mock_adapter).await;
    
    // Connect
    manager.connect_exchange("gate").await.unwrap();
    let status = manager.get_connection_status("gate").await;
    assert_eq!(status, Some(ConnectionStatus::Connected));
    
    // Disconnect
    manager.disconnect_exchange("gate").await.unwrap();
    let status = manager.get_connection_status("gate").await;
    assert_eq!(status, Some(ConnectionStatus::Disconnected));
}

#[tokio::test]
async fn test_connection_manager_get_all_statuses() {
    let manager = ConnectionManager::new();
    
    let exchanges = vec!["gate", "bybit", "hyperliquid", "dydx", "aster"];
    for exchange in &exchanges {
        let adapter = Arc::new(MockExchangeAdapter::new(exchange));
        manager.add_exchange(exchange.to_string(), adapter).await;
    }
    
    let statuses = manager.get_all_connection_statuses().await;
    assert_eq!(statuses.len(), exchanges.len());
    
    for exchange in &exchanges {
        assert!(statuses.contains_key(*exchange));
        assert_eq!(statuses.get(*exchange), Some(&ConnectionStatus::Disconnected));
    }
}

#[tokio::test]
async fn test_connection_manager_place_order() {
    let manager = ConnectionManager::new();
    
    let mock_adapter = Arc::new(MockExchangeAdapter::new("gate"));
    manager.add_exchange("gate".to_string(), mock_adapter).await;
    
    let order = NewOrder {
        symbol: "BTCUSDT".to_string(),
        side: crypto_hft::traits::OrderSide::Buy,
        order_type: crypto_hft::traits::OrderType::Limit,
        time_in_force: crypto_hft::traits::TimeInForce::GoodTillCancelled,
        quantity: crypto_hft::types::Size::from_str("0.1").unwrap(),
        price: Some(crypto_hft::types::Price::from_str("50000").unwrap()),
        client_order_id: Some("test_order".to_string()),
    };
    
    let result = manager.place_order("gate", order).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_connection_manager_get_balances() {
    let manager = ConnectionManager::new();
    
    let mock_adapter = Arc::new(MockExchangeAdapter::new("gate"));
    manager.add_exchange("gate".to_string(), mock_adapter).await;
    
    let result = manager.get_balances("gate").await;
    assert!(result.is_ok());
    let balances = result.unwrap();
    assert!(balances.is_empty()); // Mock returns empty
}

#[tokio::test]
async fn test_connection_manager_get_order_book() {
    let manager = ConnectionManager::new();
    
    let mock_adapter = Arc::new(MockExchangeAdapter::new("gate"));
    manager.add_exchange("gate".to_string(), mock_adapter).await;
    
    let result = manager.get_order_book("gate", "BTCUSDT", 10).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_connection_manager_get_trading_fees() {
    let manager = ConnectionManager::new();
    
    let mock_adapter = Arc::new(MockExchangeAdapter::new("gate"));
    manager.add_exchange("gate".to_string(), mock_adapter).await;
    
    let result = manager.get_trading_fees("gate", "BTCUSDT").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_connection_manager_nonexistent_exchange() {
    let manager = ConnectionManager::new();
    
    let status = manager.get_connection_status("nonexistent").await;
    assert_eq!(status, None);
    
    let result = manager.connect_exchange("nonexistent").await;
    assert!(result.is_err());
}

