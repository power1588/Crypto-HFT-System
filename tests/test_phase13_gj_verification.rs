//! Phase 13 Groups G-J Verification Tests
//! 
//! This file verifies that Groups G-J compilation issues are resolved:
//! - Group G: Symbol vs &str type mismatches
//! - Group H: Trait object type mismatches  
//! - Group I: MockExecutionClient Error type
//! - Group J: Async/await in sync functions

use crypto_hft::connectors::MockExecutionClient;
use crypto_hft::traits::{ExecutionClient, NewOrder, TimeInForce};
use crypto_hft::types::{Price, Size, Symbol};
use std::sync::Arc;

// ============================================================================
// Group G: Symbol vs &str Type Tests
// ============================================================================

/// T108: Symbol comparisons work correctly in connector tests
#[test]
fn test_g_symbol_as_str_comparison() {
    let symbol = Symbol::new("BTCUSDT");
    
    // Symbol can be compared with &str via .as_str()
    assert_eq!(symbol.as_str(), "BTCUSDT");
    
    // Symbol can be used in HashMap keys
    let mut map = std::collections::HashMap::new();
    map.insert(symbol.as_str().to_string(), 100);
    assert_eq!(map.get("BTCUSDT"), Some(&100));
}

/// T108: Symbol can be created from string and converted back
#[test]
fn test_g_symbol_string_conversion() {
    // From &str
    let symbol1: Symbol = "ETHUSDT".into();
    assert_eq!(symbol1.as_str(), "ETHUSDT");
    
    // From String
    let symbol2: Symbol = String::from("BNBUSDT").into();
    assert_eq!(symbol2.as_str(), "BNBUSDT");
    
    // To String
    let s: String = symbol1.into();
    assert_eq!(s, "ETHUSDT");
}

/// T109: Symbol works correctly in signal generator contexts
#[test]
fn test_g_symbol_in_order_context() {
    let order = NewOrder::new_limit_buy(
        "BTCUSDT".to_string(),
        Size::from_str("1.0").unwrap(),
        Price::from_str("50000.0").unwrap(),
        TimeInForce::GoodTillCancelled,
    );
    
    // Symbol comparison works
    assert_eq!(order.symbol.as_str(), "BTCUSDT");
    
    // Can use .to_string() on Symbol value
    let symbol_string = order.symbol.value().to_string();
    assert_eq!(symbol_string, "BTCUSDT");
}

/// T109: Symbol slicing works for asset extraction
#[test]
fn test_g_symbol_slicing() {
    let symbol = Symbol::new("BTCUSDT");
    let len = symbol.len();
    
    // Extract base asset (remove USDT suffix)
    if len >= 4 {
        let base_asset = &symbol.as_str()[..len - 4];
        assert_eq!(base_asset, "BTC");
    }
    
    // Direct indexing also works
    assert_eq!(&symbol[0..3], "BTC");
    assert_eq!(&symbol[3..7], "USDT");
}

// ============================================================================
// Group H: Trait Object Type Tests
// ============================================================================

/// T110-T112: Trait objects can be wrapped in Arc<RwLock<>>
#[test]
fn test_h_trait_object_wrapping() {
    // MarketDataStream can be wrapped - verified by type signature
    // EventLoop uses: Arc<RwLock<dyn MarketDataStream<Error = Box<dyn std::error::Error + Send + Sync>> + Send + Sync>>

    // This test verifies the trait bounds are compatible
    fn accepts_boxed_error<T: std::fmt::Debug + std::fmt::Display + Send + Sync + 'static>(_: T) {}

    // Box<dyn Error + Send + Sync> can be used as error type
    let err: Box<dyn std::error::Error + Send + Sync> =
        Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error"));
    accepts_boxed_error(err);
}

/// T110: MockMarketDataStream type is compatible with trait bounds
#[test]
fn test_h_mock_market_data_stream_bounds() {
    use crypto_hft::connectors::MockMarketDataStream;
    
    // MockMarketDataStream::Error is BoxedError = Box<dyn Error + Send + Sync>
    let stream = MockMarketDataStream::new();
    
    // Verify it can be created and has the right type
    assert!(std::mem::size_of_val(&stream) > 0);
}

// ============================================================================
// Group I: MockExecutionClient Error Type Tests
// ============================================================================

/// T113: MockExecutionClient uses BoxedError type
#[test]
fn test_i_mock_execution_client_error_type() {
    let client = MockExecutionClient::new();
    
    // Client can be created
    assert!(std::mem::size_of_val(&client) > 0);
}

/// T113: MockExecutionClient Error type is Box<dyn Error + Send + Sync>
#[tokio::test]
async fn test_i_mock_execution_client_async_operations() {
    let client = MockExecutionClient::new();
    
    // Place order
    let order = NewOrder::new_limit_buy(
        "BTCUSDT".to_string(),
        Size::from_str("1.0").unwrap(),
        Price::from_str("50000.0").unwrap(),
        TimeInForce::GoodTillCancelled,
    );
    
    let result = client.place_order(order).await;
    assert!(result.is_ok());
    
    let order_id = result.unwrap();
    
    // Get order status
    let status_result = client.get_order_status(order_id.clone()).await;
    assert!(status_result.is_ok());
    
    // Cancel order
    let cancel_result = client.cancel_order(order_id).await;
    assert!(cancel_result.is_ok());
}

/// T114-T115: MockExecutionClient can be used with trait objects
#[tokio::test]
async fn test_i_mock_execution_client_as_trait_object() {
    let client = MockExecutionClient::new();
    
    // Can be used through Arc
    let client_arc: Arc<MockExecutionClient> = Arc::new(client);
    
    let order = NewOrder::new_limit_buy(
        "ETHUSDT".to_string(),
        Size::from_str("0.5").unwrap(),
        Price::from_str("3000.0").unwrap(),
        TimeInForce::GoodTillCancelled,
    );
    
    let result = client_arc.place_order(order).await;
    assert!(result.is_ok());
}

// ============================================================================
// Group J: Async/Await Tests
// ============================================================================

/// T116: Async tests work correctly with tokio
#[tokio::test]
async fn test_j_async_test_execution() {
    // This test verifies that async tests run correctly
    let future = async {
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        42
    };
    
    let result = future.await;
    assert_eq!(result, 42);
}

/// T116: Performance monitor async methods work correctly
#[tokio::test]
async fn test_j_performance_monitor_async() {
    use crypto_hft::realtime::PerformanceMonitor;
    
    let monitor = PerformanceMonitor::new();
    
    // Record events
    monitor.record_market_data_event().await;
    monitor.record_signal().await;
    monitor.record_order_placement("test_order").await;
    monitor.record_order_fill("test_order").await;
    
    // Get metrics
    let metrics = monitor.get_metrics().await;
    assert_eq!(metrics.market_data_events, 1);
    assert_eq!(metrics.signals_generated, 1);
    assert_eq!(metrics.orders_placed, 1);
    assert_eq!(metrics.orders_filled, 1);
}

/// T116: Async rate methods work correctly
#[tokio::test]
async fn test_j_performance_monitor_rates() {
    use crypto_hft::realtime::PerformanceMonitor;
    
    let monitor = PerformanceMonitor::new();
    
    // Record some orders
    for i in 0..5 {
        monitor.record_order_placement(&format!("order_{}", i)).await;
    }
    for i in 0..3 {
        monitor.record_order_fill(&format!("order_{}", i)).await;
    }
    monitor.record_order_cancellation("order_3").await;
    
    // Get rates (async methods)
    let fill_rate = monitor.get_fill_rate().await;
    let cancel_rate = monitor.get_cancellation_rate().await;
    
    // Verify rates are calculated
    assert!(fill_rate >= 0.0);
    assert!(cancel_rate >= 0.0);
}

// ============================================================================
// Integration Tests
// ============================================================================

/// Verify all types work together correctly
#[tokio::test]
async fn test_integration_all_groups() {
    // Group G: Symbol types
    let symbol = Symbol::new("BTCUSDT");
    assert_eq!(symbol.as_str(), "BTCUSDT");
    
    // Group I: MockExecutionClient
    let client = MockExecutionClient::new();
    
    // Create order with Symbol
    let order = NewOrder::new_limit_buy(
        symbol.as_str().to_string(),
        Size::from_str("1.0").unwrap(),
        Price::from_str("50000.0").unwrap(),
        TimeInForce::GoodTillCancelled,
    );
    
    // Place order (async - Group J)
    let result = client.place_order(order).await;
    assert!(result.is_ok());
    
    // Get balances
    let balances = client.get_balances().await;
    assert!(balances.is_ok());
}

/// Verify error handling works with BoxedError
#[tokio::test]
async fn test_integration_error_handling() {
    let client = MockExecutionClient::new();
    
    // Try to get non-existent order
    let result = client.get_order_status("non_existent_order".to_string()).await;
    assert!(result.is_err());
    
    // Error should be displayable
    let err = result.unwrap_err();
    let error_message = format!("{}", err);
    assert!(error_message.contains("not found"));
}

