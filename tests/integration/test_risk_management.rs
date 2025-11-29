use crypto_hft::risk::{RiskEngine, ShadowLedger};
use crypto_hft::risk::rules::{
    PositionSizeRule, OrderSizeRule, DailyLossRule, 
    TotalExposureRule, OpenOrdersCountRule, BalanceRule
};
use crypto_hft::realtime::risk_manager::{RiskManager, RiskManagerConfig};
use crypto_hft::types::{Price, Size, Symbol};
use crypto_hft::core::events::{NewOrder, OrderSide, Position};
use crypto_hft::traits::{OrderId, OrderType, TimeInForce, ExecutionReport, OrderStatus};
use std::time::Duration;
use std::collections::HashMap;

#[tokio::test]
async fn test_risk_workflow_order_validation() {
    // Setup risk engine with rules
    let risk_engine = RiskEngine::new();
    let mut position_rule = PositionSizeRule::new();
    position_rule.set_max_position("BTCUSDT", Size::from_str("10.0").unwrap());
    risk_engine.add_rule(Box::new(position_rule)).await;
    
    let mut order_size_rule = OrderSizeRule::new();
    order_size_rule.set_max_order("BTCUSDT", Size::from_str("5.0").unwrap());
    risk_engine.add_rule(Box::new(order_size_rule)).await;
    
    // Configure risk engine limits
    risk_engine.set_max_position_size("BTCUSDT", Size::from_str("10.0").unwrap()).await;
    risk_engine.set_max_order_size("BTCUSDT", Size::from_str("5.0").unwrap()).await;
    
    // Test 1: Valid order should pass
    let valid_order = NewOrder {
        symbol: "BTCUSDT".to_string(),
        exchange_id: "binance".to_string(),
        side: OrderSide::Buy,
        order_type: OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        price: Some(Price::from_str("50000.0").unwrap()),
        quantity: Size::from_str("3.0").unwrap(),
        client_order_id: Some("valid_order".to_string()),
    };
    
    let result = risk_engine.check_order(&valid_order).await;
    assert!(result.is_ok(), "Valid order should pass risk checks");
    
    // Test 2: Order exceeding size limit should fail
    let invalid_order = NewOrder {
        symbol: "BTCUSDT".to_string(),
        exchange_id: "binance".to_string(),
        side: OrderSide::Buy,
        order_type: OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        price: Some(Price::from_str("50000.0").unwrap()),
        quantity: Size::from_str("7.0").unwrap(),
        client_order_id: Some("invalid_order".to_string()),
    };
    
    let result = risk_engine.check_order(&invalid_order).await;
    assert!(result.is_err(), "Order exceeding size limit should fail");
    assert_eq!(result.unwrap_err().rule, "OrderSizeLimit");
}

#[tokio::test]
async fn test_risk_workflow_position_tracking() {
    let risk_engine = RiskEngine::new();
    let shadow_ledger = ShadowLedger::new();
    
    // Setup initial position
    let position = Position {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        size: Size::from_str("5.0").unwrap(),
        average_price: Some(Price::from_str("50000.0").unwrap()),
        unrealized_pnl: Some(crate::rust_decimal::Decimal::ZERO),
    };
    
    risk_engine.update_position("BTCUSDT", position).await;
    
    // Verify position is tracked
    let retrieved_position = risk_engine.get_position("BTCUSDT").await;
    assert!(retrieved_position.is_some());
    assert_eq!(retrieved_position.unwrap().size, Size::from_str("5.0").unwrap());
    
    // Test position size rule with existing position
    let mut position_rule = PositionSizeRule::new();
    position_rule.set_max_position("BTCUSDT", Size::from_str("10.0").unwrap());
    risk_engine.add_rule(Box::new(position_rule)).await;
    risk_engine.set_max_position_size("BTCUSDT", Size::from_str("10.0").unwrap()).await;
    
    // Order that would exceed position limit
    let order = NewOrder {
        symbol: "BTCUSDT".to_string(),
        exchange_id: "binance".to_string(),
        side: OrderSide::Buy,
        order_type: OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        price: Some(Price::from_str("50000.0").unwrap()),
        quantity: Size::from_str("6.0").unwrap(), // 5.0 + 6.0 = 11.0 > 10.0
        client_order_id: Some("test_order".to_string()),
    };
    
    let result = risk_engine.check_order(&order).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().rule, "PositionSizeLimit");
}

#[tokio::test]
async fn test_risk_workflow_daily_loss_tracking() {
    let risk_engine = RiskEngine::new();
    
    // Setup daily loss limit
    risk_engine.set_max_daily_loss("BTCUSDT", Price::from_str("1000.0").unwrap()).await;
    
    let mut daily_loss_rule = DailyLossRule::new();
    daily_loss_rule.set_max_loss("BTCUSDT", Price::from_str("1000.0").unwrap());
    risk_engine.add_rule(Box::new(daily_loss_rule)).await;
    
    // Record some losses
    risk_engine.record_daily_loss("BTCUSDT", Price::from_str("500.0").unwrap()).await;
    risk_engine.record_daily_loss("BTCUSDT", Price::from_str("400.0").unwrap()).await;
    
    let current_loss = risk_engine.get_daily_loss("BTCUSDT").await;
    assert_eq!(current_loss, Price::from_str("900.0").unwrap());
    
    // Reset daily losses
    risk_engine.reset_daily_losses().await;
    let loss_after_reset = risk_engine.get_daily_loss("BTCUSDT").await;
    assert_eq!(loss_after_reset, Price::new(crate::rust_decimal::Decimal::ZERO));
}

#[tokio::test]
async fn test_risk_workflow_total_exposure() {
    let risk_engine = RiskEngine::new();
    
    // Setup total exposure limit
    risk_engine.set_max_total_exposure(Price::from_str("100000.0").unwrap()).await;
    
    let exposure_rule = TotalExposureRule::new(Price::from_str("100000.0").unwrap());
    risk_engine.add_rule(Box::new(exposure_rule)).await;
    
    // Add positions
    let position1 = Position {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        size: Size::from_str("1.5").unwrap(),
        average_price: Some(Price::from_str("50000.0").unwrap()),
        unrealized_pnl: Some(crate::rust_decimal::Decimal::ZERO),
    };
    
    let position2 = Position {
        symbol: Symbol::new("ETHUSDT"),
        exchange_id: "binance".to_string(),
        size: Size::from_str("10.0").unwrap(),
        average_price: Some(Price::from_str("3000.0").unwrap()),
        unrealized_pnl: Some(crate::rust_decimal::Decimal::ZERO),
    };
    
    risk_engine.update_position("BTCUSDT", position1).await;
    risk_engine.update_position("ETHUSDT", position2).await;
    
    // Calculate total exposure: 1.5 * 50000 + 10 * 3000 = 75000 + 30000 = 105000
    let total_exposure = risk_engine.get_total_exposure().await;
    assert_eq!(total_exposure, Price::from_str("105000.0").unwrap());
    
    // Order that would exceed exposure limit
    let order = NewOrder {
        symbol: "BTCUSDT".to_string(),
        exchange_id: "binance".to_string(),
        side: OrderSide::Buy,
        order_type: OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        price: Some(Price::from_str("50000.0").unwrap()),
        quantity: Size::from_str("1.0").unwrap(),
        client_order_id: Some("test_order".to_string()),
    };
    
    let result = risk_engine.check_order(&order).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().rule, "TotalExposureLimit");
}

#[tokio::test]
async fn test_risk_manager_integration() {
    // Setup risk manager
    let mut config = RiskManagerConfig::default();
    config.max_position_sizes.insert("BTCUSDT".to_string(), Size::from_str("10.0").unwrap());
    config.max_order_sizes.insert("BTCUSDT".to_string(), Size::from_str("5.0").unwrap());
    config.max_daily_losses.insert("BTCUSDT".to_string(), Price::from_str("1000.0").unwrap());
    config.max_total_exposure = Price::from_str("100000.0").unwrap();
    
    let risk_engine = RiskEngine::new();
    let shadow_ledger = ShadowLedger::new();
    let risk_manager = RiskManager::new(
        config,
        risk_engine,
        shadow_ledger,
        Duration::from_secs(60),
    );
    
    // Test risk stats
    let stats = risk_manager.get_risk_stats().await;
    assert_eq!(stats.total_positions, 0);
    assert_eq!(stats.total_trades, 0);
    assert_eq!(stats.risk_violations, 0);
}

#[tokio::test]
async fn test_risk_manager_execution_report_handling() {
    let config = RiskManagerConfig::default();
    let risk_engine = RiskEngine::new();
    let shadow_ledger = ShadowLedger::new();
    let risk_manager = RiskManager::new(
        config,
        risk_engine,
        shadow_ledger,
        Duration::from_secs(60),
    );
    
    // Create execution report
    let report = ExecutionReport {
        order_id: OrderId::new("order_123".to_string()),
        client_order_id: Some("client_123".to_string()),
        symbol: "BTCUSDT".to_string(),
        status: OrderStatus::Filled { 
            filled_size: Size::from_str("1.0").unwrap()
        },
        side: OrderSide::Buy,
        order_type: OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        quantity: Size::from_str("1.0").unwrap(),
        price: Some(Price::from_str("50000.0").unwrap()),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    };
    
    // Handle execution report
    let result = risk_manager.handle_execution_report(&report).await;
    assert!(result.is_ok());
    
    // Check that position was updated
    let stats = risk_manager.get_risk_stats().await;
    assert_eq!(stats.total_positions, 1);
    assert_eq!(stats.total_trades, 1);
}

#[tokio::test]
async fn test_risk_manager_risk_limit_checking() {
    let mut config = RiskManagerConfig::default();
    config.max_total_exposure = Price::from_str("100000.0").unwrap();
    config.enable_auto_position_reduction = true;
    config.enable_auto_order_cancellation = true;
    
    let mut risk_engine = RiskEngine::new();
    risk_engine.set_max_total_exposure(Price::from_str("100000.0").unwrap()).await;
    
    // Add a position that exceeds exposure limit
    let position = Position {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        size: Size::from_str("3.0").unwrap(),
        average_price: Some(Price::from_str("50000.0").unwrap()),
        unrealized_pnl: Some(crate::rust_decimal::Decimal::ZERO),
    };
    
    risk_engine.update_position("BTCUSDT", position).await;
    
    let shadow_ledger = ShadowLedger::new();
    let risk_manager = RiskManager::new(
        config,
        risk_engine,
        shadow_ledger,
        Duration::from_secs(0), // Check immediately
    );
    
    // Check risk limits
    let violations = risk_manager.check_risk_limits().await.unwrap();
    assert!(!violations.is_empty());
    assert!(violations.iter().any(|v| v.rule == "TotalExposureLimit"));
}

#[tokio::test]
async fn test_risk_workflow_multiple_symbols() {
    let risk_engine = RiskEngine::new();
    
    // Setup limits for multiple symbols
    risk_engine.set_max_position_size("BTCUSDT", Size::from_str("10.0").unwrap()).await;
    risk_engine.set_max_position_size("ETHUSDT", Size::from_str("100.0").unwrap()).await;
    risk_engine.set_max_order_size("BTCUSDT", Size::from_str("5.0").unwrap()).await;
    risk_engine.set_max_order_size("ETHUSDT", Size::from_str("50.0").unwrap()).await;
    
    // Add positions for multiple symbols
    let btc_position = Position {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        size: Size::from_str("5.0").unwrap(),
        average_price: Some(Price::from_str("50000.0").unwrap()),
        unrealized_pnl: Some(crate::rust_decimal::Decimal::ZERO),
    };
    
    let eth_position = Position {
        symbol: Symbol::new("ETHUSDT"),
        exchange_id: "binance".to_string(),
        size: Size::from_str("50.0").unwrap(),
        average_price: Some(Price::from_str("3000.0").unwrap()),
        unrealized_pnl: Some(crate::rust_decimal::Decimal::ZERO),
    };
    
    risk_engine.update_position("BTCUSDT", btc_position).await;
    risk_engine.update_position("ETHUSDT", eth_position).await;
    
    // Verify positions are tracked separately
    let btc_pos = risk_engine.get_position("BTCUSDT").await;
    assert!(btc_pos.is_some());
    assert_eq!(btc_pos.unwrap().size, Size::from_str("5.0").unwrap());
    
    let eth_pos = risk_engine.get_position("ETHUSDT").await;
    assert!(eth_pos.is_some());
    assert_eq!(eth_pos.unwrap().size, Size::from_str("50.0").unwrap());
}

#[tokio::test]
async fn test_risk_workflow_open_orders_limit() {
    let risk_engine = RiskEngine::new();
    
    // Setup open orders limit
    risk_engine.set_max_open_orders(3).await;
    
    let orders_rule = OpenOrdersCountRule::new(3);
    risk_engine.add_rule(Box::new(orders_rule)).await;
    
    // Increment orders to limit
    risk_engine.increment_open_orders().await;
    risk_engine.increment_open_orders().await;
    risk_engine.increment_open_orders().await;
    
    assert_eq!(risk_engine.get_open_orders_count().await, 3);
    
    // Next order should fail
    let order = NewOrder {
        symbol: "BTCUSDT".to_string(),
        exchange_id: "binance".to_string(),
        side: OrderSide::Buy,
        order_type: OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        price: Some(Price::from_str("50000.0").unwrap()),
        quantity: Size::from_str("1.0").unwrap(),
        client_order_id: Some("test_order".to_string()),
    };
    
    let result = risk_engine.check_order(&order).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().rule, "OpenOrdersCountLimit");
}

