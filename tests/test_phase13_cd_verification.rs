//! Phase 13 Groups C and D TDD Verification Tests
//!
//! This test file verifies the fixes for:
//! - Group C: ExecutionReport struct field corrections
//! - Group D: NewOrder field name corrections (size instead of quantity)
//!
//! ExecutionReport correct fields:
//! - order_id: OrderId (String)
//! - client_order_id: Option<String>
//! - symbol: Symbol
//! - exchange_id: ExchangeId (String)
//! - status: OrderStatus
//! - filled_size: Size
//! - remaining_size: Size
//! - average_price: Option<Price>
//! - timestamp: Timestamp (u64)
//!
//! NewOrder correct fields:
//! - symbol: Symbol
//! - exchange_id: ExchangeId (String)
//! - side: OrderSide
//! - order_type: OrderType
//! - time_in_force: TimeInForce
//! - price: Option<Price>
//! - size: Size (NOT quantity!)
//! - client_order_id: Option<String>

use crypto_hft::{
    ExecutionReport, NewOrder, OrderSide, OrderStatus, OrderType, Price, Size, Symbol, TimeInForce,
};

// ============================================================================
// Group C: ExecutionReport Struct Field Tests (T099, T100, T101)
// ============================================================================

/// T099-1: Verify ExecutionReport has correct fields for order_executor tests
#[test]
fn test_execution_report_correct_fields() {
    let report = ExecutionReport {
        order_id: "order_123".to_string(),
        client_order_id: Some("client_456".to_string()),
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        status: OrderStatus::Filled,
        filled_size: Size::from_str("1.0").unwrap(),
        remaining_size: Size::from_str("0.0").unwrap(),
        average_price: Some(Price::from_str("50000.0").unwrap()),
        timestamp: 1700000000000,
    };

    assert_eq!(report.order_id, "order_123");
    assert_eq!(report.client_order_id, Some("client_456".to_string()));
    assert_eq!(report.symbol.as_str(), "BTCUSDT");
    assert_eq!(report.exchange_id, "binance");
    assert_eq!(report.status, OrderStatus::Filled);
    assert_eq!(report.filled_size, Size::from_str("1.0").unwrap());
    assert_eq!(report.remaining_size, Size::from_str("0.0").unwrap());
    assert_eq!(
        report.average_price,
        Some(Price::from_str("50000.0").unwrap())
    );
    assert_eq!(report.timestamp, 1700000000000);
}

/// T099-2: Verify ExecutionReport with no average_price (for unfilled orders)
#[test]
fn test_execution_report_no_average_price() {
    let report = ExecutionReport {
        order_id: "order_123".to_string(),
        client_order_id: None,
        symbol: Symbol::new("ETHUSDT"),
        exchange_id: "binance".to_string(),
        status: OrderStatus::New,
        filled_size: Size::from_str("0.0").unwrap(),
        remaining_size: Size::from_str("1.0").unwrap(),
        average_price: None,
        timestamp: 1700000000000,
    };

    assert_eq!(report.status, OrderStatus::New);
    assert!(report.average_price.is_none());
    assert!(report.client_order_id.is_none());
}

/// T099-3: Verify all OrderStatus variants work with ExecutionReport
#[test]
fn test_execution_report_all_status_variants() {
    let statuses = vec![
        OrderStatus::New,
        OrderStatus::PartiallyFilled,
        OrderStatus::Filled,
        OrderStatus::Cancelled,
        OrderStatus::Rejected,
        OrderStatus::Expired,
    ];

    for status in statuses {
        let report = ExecutionReport {
            order_id: "test".to_string(),
            client_order_id: None,
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "test".to_string(),
            status,
            filled_size: Size::from_str("0.0").unwrap(),
            remaining_size: Size::from_str("1.0").unwrap(),
            average_price: None,
            timestamp: 0,
        };

        // Just verify it compiles and status matches
        assert_eq!(report.status, status);
    }
}

/// T100: Verify ExecutionReport used correctly in risk_manager context
#[test]
fn test_execution_report_for_risk_manager() {
    // Create a filled execution report as used in risk_manager tests
    let report = ExecutionReport {
        order_id: "12345".to_string(),
        client_order_id: Some("client_123".to_string()),
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        status: OrderStatus::Filled,
        filled_size: Size::from_str("1.0").unwrap(),
        remaining_size: Size::from_str("0.0").unwrap(),
        average_price: Some(Price::from_str("50000.0").unwrap()),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    };

    // Verify fields are accessible as used in risk_manager
    assert_eq!(report.symbol.as_str(), "BTCUSDT");
    assert_eq!(report.filled_size, Size::from_str("1.0").unwrap());
    assert!(report.average_price.is_some());
}

/// T101: Verify ExecutionReport used correctly in shadow_ledger context
#[test]
fn test_execution_report_for_shadow_ledger() {
    // Create ExecutionReport as used in shadow_ledger
    let report = ExecutionReport {
        order_id: "order_456".to_string(),
        client_order_id: Some("client_123".to_string()),
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        status: OrderStatus::Filled,
        filled_size: Size::from_str("1.0").unwrap(),
        remaining_size: Size::from_str("0.0").unwrap(),
        average_price: Some(Price::from_str("50000.0").unwrap()),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    };

    // Verify all fields needed by shadow_ledger are present and correct
    assert!(!report.order_id.is_empty());
    assert_eq!(report.symbol.as_str(), "BTCUSDT");
    assert_eq!(report.status, OrderStatus::Filled);

    // Verify timestamp can be converted to DateTime
    let timestamp_secs = (report.timestamp / 1000) as i64;
    let datetime = chrono::DateTime::from_timestamp(timestamp_secs, 0);
    assert!(datetime.is_some());
}

// ============================================================================
// Group D: NewOrder Field Name Tests (T102)
// ============================================================================

/// T102-1: Verify NewOrder uses 'size' field (not 'quantity')
#[test]
fn test_new_order_uses_size_field() {
    let order = NewOrder {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        side: OrderSide::Buy,
        order_type: OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        price: Some(Price::from_str("50000.0").unwrap()),
        size: Size::from_str("1.0").unwrap(), // NOTE: This is 'size', NOT 'quantity'
        client_order_id: Some("client_123".to_string()),
    };

    // Verify size field is accessible and correct
    assert_eq!(order.size, Size::from_str("1.0").unwrap());
}

/// T102-2: Verify NewOrder helper methods use size correctly
#[test]
fn test_new_order_helper_methods_use_size() {
    // Test new_limit_buy
    let buy_order = NewOrder::new_limit_buy(
        "BTCUSDT".to_string(),
        Size::from_str("0.5").unwrap(),
        Price::from_str("50000.0").unwrap(),
        TimeInForce::GoodTillCancelled,
    );
    assert_eq!(buy_order.size, Size::from_str("0.5").unwrap());

    // Test new_limit_sell
    let sell_order = NewOrder::new_limit_sell(
        "ETHUSDT".to_string(),
        Size::from_str("2.0").unwrap(),
        Price::from_str("3000.0").unwrap(),
        TimeInForce::ImmediateOrCancel,
    );
    assert_eq!(sell_order.size, Size::from_str("2.0").unwrap());

    // Test new_market_buy
    let market_buy = NewOrder::new_market_buy("BTCUSDT".to_string(), Size::from_str("0.1").unwrap());
    assert_eq!(market_buy.size, Size::from_str("0.1").unwrap());

    // Test new_market_sell
    let market_sell =
        NewOrder::new_market_sell("BTCUSDT".to_string(), Size::from_str("0.2").unwrap());
    assert_eq!(market_sell.size, Size::from_str("0.2").unwrap());
}

/// T102-3: Verify NewOrder size field used in signal_generator context
#[test]
fn test_new_order_size_in_signal_context() {
    // This tests the pattern used in signal_generator.rs
    let order = NewOrder::new_limit_buy(
        "BTCUSDT".to_string(),
        Size::from_str("0.1").unwrap(),
        Price::from_str("50000.0").unwrap(),
        TimeInForce::GoodTillCancelled,
    );

    // Access order.size (the correct field name)
    let order_size = order.size;
    assert_eq!(order_size, Size::from_str("0.1").unwrap());

    // Verify other fields are correct too
    assert_eq!(order.symbol.as_str(), "BTCUSDT");
    assert_eq!(order.side, OrderSide::Buy);
    assert_eq!(order.order_type, OrderType::Limit);
}

// ============================================================================
// Combined Tests: ExecutionReport and NewOrder together
// ============================================================================

/// Combined test: Order creation and execution report flow
#[test]
fn test_order_to_execution_report_flow() {
    // Step 1: Create a new order with 'size' field
    let new_order = NewOrder {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        side: OrderSide::Buy,
        order_type: OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        price: Some(Price::from_str("50000.0").unwrap()),
        size: Size::from_str("1.0").unwrap(),
        client_order_id: Some("client_123".to_string()),
    };

    // Step 2: Simulate receiving an execution report
    let execution_report = ExecutionReport {
        order_id: "order_789".to_string(),
        client_order_id: new_order.client_order_id.clone(),
        symbol: new_order.symbol.clone(),
        exchange_id: new_order.exchange_id.clone(),
        status: OrderStatus::Filled,
        filled_size: new_order.size, // Uses size from NewOrder
        remaining_size: Size::from_str("0.0").unwrap(),
        average_price: new_order.price,
        timestamp: 1700000000000,
    };

    // Verify the flow works correctly
    assert_eq!(execution_report.symbol, new_order.symbol);
    assert_eq!(execution_report.filled_size, new_order.size);
    assert_eq!(execution_report.status, OrderStatus::Filled);
}

/// Test partial fill scenario
#[test]
fn test_partial_fill_execution_report() {
    let original_size = Size::from_str("10.0").unwrap();
    let filled_size = Size::from_str("3.0").unwrap();
    let remaining_size = Size::from_str("7.0").unwrap();

    let report = ExecutionReport {
        order_id: "order_123".to_string(),
        client_order_id: None,
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        status: OrderStatus::PartiallyFilled,
        filled_size,
        remaining_size,
        average_price: Some(Price::from_str("50000.0").unwrap()),
        timestamp: 1700000000000,
    };

    // Verify partial fill accounting
    assert_eq!(report.status, OrderStatus::PartiallyFilled);
    assert_eq!(
        report.filled_size.value() + report.remaining_size.value(),
        original_size.value()
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

/// Test zero size order (edge case)
#[test]
fn test_zero_size_handling() {
    let zero_size = Size::from_str("0.0").unwrap();

    let report = ExecutionReport {
        order_id: "test".to_string(),
        client_order_id: None,
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "test".to_string(),
        status: OrderStatus::Cancelled,
        filled_size: zero_size,
        remaining_size: Size::from_str("1.0").unwrap(),
        average_price: None,
        timestamp: 0,
    };

    assert!(report.filled_size.is_zero());
}

/// Test very large size values
#[test]
fn test_large_size_values() {
    let large_size = Size::from_str("1000000.0").unwrap();

    let order = NewOrder::new_market_buy("BTCUSDT".to_string(), large_size);
    assert_eq!(order.size, large_size);

    let report = ExecutionReport {
        order_id: "test".to_string(),
        client_order_id: None,
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "test".to_string(),
        status: OrderStatus::Filled,
        filled_size: large_size,
        remaining_size: Size::from_str("0.0").unwrap(),
        average_price: Some(Price::from_str("50000.0").unwrap()),
        timestamp: 0,
    };

    assert_eq!(report.filled_size, large_size);
}

