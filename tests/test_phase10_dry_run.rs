//! Phase 10: Dry Run Connector TDD Tests
//!
//! This test module verifies the completion of Phase 10 (Connector Fixes)
//! for the DryRunExecutionClient implementation.
//!
//! Test Requirements (from tasks.md T073):
//! - [x] OrderType variants: StopLoss, StopLimit exist in core::events
//! - [x] ExecutionReport fields: filled_size, remaining_size, average_price
//! - [x] Symbol comparison using .as_str()
//! - [x] OrderStatus::Cancelled spelling (not Canceled)
//! - [x] Size::zero() method usage

use crypto_hft::connectors::dry_run::{DryRunError, DryRunExecutionClient};
use crypto_hft::traits::{
    Balance, ExecutionClient, ExecutionReport, NewOrder, OrderId, OrderSide, OrderStatus,
    OrderType, TimeInForce, TradingFees,
};
use crypto_hft::types::{Price, Size, Symbol};

/// Test T073-1: Verify OrderType variants exist
#[test]
fn test_order_type_variants() {
    // Verify all required OrderType variants exist
    let _market = OrderType::Market;
    let _limit = OrderType::Limit;
    let _stop_loss = OrderType::StopLoss; // Required variant
    let _stop_limit = OrderType::StopLimit; // Required variant
}

/// Test T073-2: Verify ExecutionReport has correct fields
#[test]
fn test_execution_report_fields() {
    let report = ExecutionReport {
        order_id: "test_order_1".to_string(),
        client_order_id: Some("client_1".to_string()),
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "dry_run".to_string(),
        status: OrderStatus::New,
        filled_size: Size::zero(),                      // Required field
        remaining_size: Size::from_str("1.0").unwrap(), // Required field
        average_price: None,                            // Required field (Option<Price>)
        timestamp: 1234567890u64,
    };

    // Verify fields are accessible
    assert_eq!(report.order_id, "test_order_1");
    assert_eq!(report.filled_size, Size::zero());
    assert_eq!(report.average_price, None);
}

/// Test T073-3: Verify Symbol comparison using as_str()
#[test]
fn test_symbol_as_str_comparison() {
    let symbol = Symbol::new("BTCUSDT");

    // Verify as_str() method works for comparison
    assert_eq!(symbol.as_str(), "BTCUSDT");

    // Verify string comparison works
    let other_str = "BTCUSDT";
    assert_eq!(symbol.as_str(), other_str);
}

/// Test T073-4: Verify OrderStatus::Cancelled spelling
#[test]
fn test_order_status_cancelled_spelling() {
    // Verify Cancelled is spelled correctly (not Canceled)
    let status = OrderStatus::Cancelled;

    match status {
        OrderStatus::Cancelled => assert!(true),
        _ => panic!("OrderStatus::Cancelled should exist"),
    }
}

/// Test T073-5: Verify Size::zero() method exists
#[test]
fn test_size_zero_method() {
    let zero_size = Size::zero();

    // Verify zero size is actually zero
    assert_eq!(zero_size, Size::from_str("0").unwrap());
}

/// Test T073-6: Verify DryRunExecutionClient can be created
#[test]
fn test_dry_run_client_creation() {
    let client = DryRunExecutionClient::new();
    // If this compiles, the client is properly implemented
    assert!(true);
}

/// Test T073-7: Verify DryRunError types exist
#[test]
fn test_dry_run_error_types() {
    let order_id: OrderId = "test_123".to_string();
    let error = DryRunError::OrderNotFound(order_id);

    // Verify Display trait
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("test_123"));

    // Verify Error trait
    let _: &dyn std::error::Error = &error;
}

/// Test T073-8: Verify TimeInForce variants
#[test]
fn test_time_in_force_variants() {
    // Verify correct TimeInForce variant names
    let _gtc = TimeInForce::GoodTillCancelled; // Not GTC
    let _ioc = TimeInForce::ImmediateOrCancel;
    let _fok = TimeInForce::FillOrKill;
}

/// Test T073-9: Verify Balance struct
#[test]
fn test_balance_struct() {
    let balance = Balance::new(
        "BTC".to_string(),
        Size::from_str("10.0").unwrap(),
        Size::from_str("0.0").unwrap(),
    );

    assert_eq!(balance.asset, "BTC");
}

/// Test T073-10: Verify TradingFees struct
#[test]
fn test_trading_fees_struct() {
    let fees = TradingFees::new(
        "BTCUSDT".to_string(),
        Size::from_str("0.001").unwrap(),
        Size::from_str("0.001").unwrap(),
    );

    assert_eq!(fees.symbol, "BTCUSDT");
}

/// Async Tests - Require tokio runtime

#[tokio::test]
async fn test_dry_run_place_order() {
    let client = DryRunExecutionClient::new();

    let order = NewOrder {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "dry_run".to_string(),
        side: OrderSide::Buy,
        order_type: OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        price: Some(Price::from_str("50000.00").unwrap()),
        size: Size::from_str("1.0").unwrap(),
        client_order_id: Some("test_phase10_1".to_string()),
    };

    let result = client.place_order(order).await;
    assert!(result.is_ok());

    let order_id = result.unwrap();
    assert!(order_id.starts_with("dry_run_"));
}

#[tokio::test]
async fn test_dry_run_cancel_order() {
    let client = DryRunExecutionClient::new();

    // First place an order
    let order = NewOrder {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "dry_run".to_string(),
        side: OrderSide::Buy,
        order_type: OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        price: Some(Price::from_str("50000.00").unwrap()),
        size: Size::from_str("1.0").unwrap(),
        client_order_id: None,
    };

    let order_id = client.place_order(order).await.unwrap();

    // Then cancel it
    let cancel_result = client.cancel_order(order_id.clone()).await;
    assert!(cancel_result.is_ok());

    // Verify status is Cancelled (correct spelling)
    let status = client.get_order_status(order_id).await.unwrap();
    assert_eq!(status.status, OrderStatus::Cancelled);
}

#[tokio::test]
async fn test_dry_run_get_balances() {
    let client = DryRunExecutionClient::new();
    let balances = client.get_balances().await.unwrap();

    assert!(!balances.is_empty());

    // Should have BTC and USDT balances
    let btc_balance = balances.iter().find(|b| b.asset == "BTC");
    assert!(btc_balance.is_some());

    let usdt_balance = balances.iter().find(|b| b.asset == "USDT");
    assert!(usdt_balance.is_some());
}

#[tokio::test]
async fn test_dry_run_get_open_orders() {
    let client = DryRunExecutionClient::new();

    // Place an order
    let order = NewOrder {
        symbol: Symbol::new("ETHUSDT"),
        exchange_id: "dry_run".to_string(),
        side: OrderSide::Sell,
        order_type: OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        price: Some(Price::from_str("3000.00").unwrap()),
        size: Size::from_str("2.0").unwrap(),
        client_order_id: None,
    };

    client.place_order(order).await.unwrap();

    // Get open orders
    let open_orders = client.get_open_orders(None).await.unwrap();
    assert!(!open_orders.is_empty());

    // Get open orders for specific symbol using as_str()
    let eth_orders = client.get_open_orders(Some("ETHUSDT")).await.unwrap();
    assert!(!eth_orders.is_empty());

    // Verify symbol comparison using as_str()
    for order in &eth_orders {
        assert_eq!(order.symbol.as_str(), "ETHUSDT");
    }
}

#[tokio::test]
async fn test_dry_run_get_order_history() {
    let client = DryRunExecutionClient::new();

    // Place and cancel some orders
    for i in 0..3 {
        let order = NewOrder {
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "dry_run".to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Market,
            time_in_force: TimeInForce::ImmediateOrCancel,
            price: None,
            size: Size::from_str("0.1").unwrap(),
            client_order_id: Some(format!("history_test_{}", i)),
        };
        client.place_order(order).await.unwrap();
    }

    // Get order history
    let history = client.get_order_history(None, Some(10)).await.unwrap();
    assert!(history.len() >= 3);

    // Get history for specific symbol
    let btc_history = client
        .get_order_history(Some("BTCUSDT"), Some(5))
        .await
        .unwrap();
    assert!(!btc_history.is_empty());
}

#[tokio::test]
async fn test_dry_run_get_trading_fees() {
    let client = DryRunExecutionClient::new();

    let fees = client.get_trading_fees("BTCUSDT").await.unwrap();

    // Verify fees struct has symbol field
    assert_eq!(fees.symbol, "BTCUSDT");
}

#[tokio::test]
async fn test_dry_run_stop_loss_order() {
    let client = DryRunExecutionClient::new();

    // Test StopLoss order type
    let order = NewOrder {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "dry_run".to_string(),
        side: OrderSide::Sell,
        order_type: OrderType::StopLoss, // Testing StopLoss variant
        time_in_force: TimeInForce::GoodTillCancelled,
        price: Some(Price::from_str("45000.00").unwrap()),
        size: Size::from_str("0.5").unwrap(),
        client_order_id: Some("stop_loss_test".to_string()),
    };

    let result = client.place_order(order).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_dry_run_stop_limit_order() {
    let client = DryRunExecutionClient::new();

    // Test StopLimit order type
    let order = NewOrder {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "dry_run".to_string(),
        side: OrderSide::Sell,
        order_type: OrderType::StopLimit, // Testing StopLimit variant
        time_in_force: TimeInForce::GoodTillCancelled,
        price: Some(Price::from_str("44000.00").unwrap()),
        size: Size::from_str("0.5").unwrap(),
        client_order_id: Some("stop_limit_test".to_string()),
    };

    let result = client.place_order(order).await;
    assert!(result.is_ok());
}
