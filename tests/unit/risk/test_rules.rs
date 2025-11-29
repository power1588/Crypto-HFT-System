use crypto_hft::risk::{RiskEngine, RiskRule, RiskViolation};
use crypto_hft::risk::rules::{
    PositionSizeRule, OrderSizeRule, DailyLossRule, 
    TotalExposureRule, OpenOrdersCountRule, BalanceRule
};
use crypto_hft::types::{Price, Size, Symbol};
use crypto_hft::core::events::{NewOrder, OrderSide, TimeInForce, Position};
use crypto_hft::traits::OrderId;

#[tokio::test]
async fn test_risk_engine_creation() {
    let risk_engine = RiskEngine::new();
    
    // Verify initial state
    assert_eq!(risk_engine.get_open_orders_count().await, 0);
    assert_eq!(risk_engine.get_max_open_orders().await, 100);
    assert_eq!(risk_engine.get_max_total_exposure().await, Price::new(crate::rust_decimal::Decimal::MAX));
}

#[tokio::test]
async fn test_risk_engine_position_management() {
    let risk_engine = RiskEngine::new();
    
    // Test setting and getting max position size
    risk_engine.set_max_position_size("BTCUSDT", Size::from_str("10.0").unwrap()).await;
    let max_size = risk_engine.get_max_position_size("BTCUSDT").await;
    assert_eq!(max_size, Size::from_str("10.0").unwrap());
    
    // Test updating position
    let position = Position {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        size: Size::from_str("5.0").unwrap(),
        average_price: Some(Price::from_str("50000.0").unwrap()),
        unrealized_pnl: Some(crate::rust_decimal::Decimal::ZERO),
    };
    risk_engine.update_position("BTCUSDT", position.clone()).await;
    
    let retrieved_position = risk_engine.get_position("BTCUSDT").await;
    assert!(retrieved_position.is_some());
    assert_eq!(retrieved_position.unwrap().size, Size::from_str("5.0").unwrap());
}

#[tokio::test]
async fn test_risk_engine_balance_management() {
    let risk_engine = RiskEngine::new();
    
    // Test updating balance
    risk_engine.update_balance("USDT", Size::from_str("10000.0").unwrap()).await;
    let balance = risk_engine.get_balance("USDT").await;
    assert_eq!(balance, Size::from_str("10000.0").unwrap());
    
    // Test getting non-existent balance
    let balance = risk_engine.get_balance("BTC").await;
    assert_eq!(balance, Size::new(crate::rust_decimal::Decimal::ZERO));
}

#[tokio::test]
async fn test_risk_engine_daily_loss_tracking() {
    let risk_engine = RiskEngine::new();
    
    // Set max daily loss
    risk_engine.set_max_daily_loss("BTCUSDT", Price::from_str("1000.0").unwrap()).await;
    
    // Record a loss
    risk_engine.record_daily_loss("BTCUSDT", Price::from_str("500.0").unwrap()).await;
    let daily_loss = risk_engine.get_daily_loss("BTCUSDT").await;
    assert_eq!(daily_loss, Price::from_str("500.0").unwrap());
    
    // Record another loss
    risk_engine.record_daily_loss("BTCUSDT", Price::from_str("300.0").unwrap()).await;
    let daily_loss = risk_engine.get_daily_loss("BTCUSDT").await;
    assert_eq!(daily_loss, Price::from_str("800.0").unwrap());
    
    // Reset daily losses
    risk_engine.reset_daily_losses().await;
    let daily_loss = risk_engine.get_daily_loss("BTCUSDT").await;
    assert_eq!(daily_loss, Price::new(crate::rust_decimal::Decimal::ZERO));
}

#[tokio::test]
async fn test_risk_engine_open_orders_count() {
    let risk_engine = RiskEngine::new();
    
    // Set max open orders
    risk_engine.set_max_open_orders(5).await;
    assert_eq!(risk_engine.get_max_open_orders().await, 5);
    
    // Increment orders
    risk_engine.increment_open_orders().await;
    risk_engine.increment_open_orders().await;
    assert_eq!(risk_engine.get_open_orders_count().await, 2);
    
    // Decrement orders
    risk_engine.decrement_open_orders().await;
    assert_eq!(risk_engine.get_open_orders_count().await, 1);
    
    // Decrement below zero should not go negative
    risk_engine.decrement_open_orders().await;
    risk_engine.decrement_open_orders().await;
    assert_eq!(risk_engine.get_open_orders_count().await, 0);
}

#[tokio::test]
async fn test_risk_engine_total_exposure() {
    let risk_engine = RiskEngine::new();
    
    // Set max total exposure
    risk_engine.set_max_total_exposure(Price::from_str("100000.0").unwrap()).await;
    assert_eq!(risk_engine.get_max_total_exposure().await, Price::from_str("100000.0").unwrap());
    
    // Add positions
    let position1 = Position {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        size: Size::from_str("2.0").unwrap(),
        average_price: Some(Price::from_str("50000.0").unwrap()),
        unrealized_pnl: Some(crate::rust_decimal::Decimal::ZERO),
    };
    risk_engine.update_position("BTCUSDT", position1).await;
    
    let position2 = Position {
        symbol: Symbol::new("ETHUSDT"),
        exchange_id: "binance".to_string(),
        size: Size::from_str("10.0").unwrap(),
        average_price: Some(Price::from_str("3000.0").unwrap()),
        unrealized_pnl: Some(crate::rust_decimal::Decimal::ZERO),
    };
    risk_engine.update_position("ETHUSDT", position2).await;
    
    // Calculate total exposure: 2 * 50000 + 10 * 3000 = 100000 + 30000 = 130000
    let total_exposure = risk_engine.get_total_exposure().await;
    assert_eq!(total_exposure, Price::from_str("130000.0").unwrap());
}

#[tokio::test]
async fn test_position_size_rule() {
    let risk_engine = RiskEngine::new();
    let mut rule = PositionSizeRule::new();
    
    // Set maximum position size
    risk_engine.set_max_position_size("BTCUSDT", Size::from_str("10.0").unwrap()).await;
    rule.set_max_position("BTCUSDT", Size::from_str("10.0").unwrap());
    
    // Add rule to engine
    risk_engine.add_rule(Box::new(rule)).await;
    
    // Create order that would exceed position limit
    let order = NewOrder {
        symbol: "BTCUSDT".to_string(),
        exchange_id: "binance".to_string(),
        side: OrderSide::Buy,
        order_type: crypto_hft::traits::OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        price: Some(Price::from_str("50000.0").unwrap()),
        quantity: Size::from_str("15.0").unwrap(),
        client_order_id: Some("test_order_1".to_string()),
    };
    
    // Check should fail
    let result = risk_engine.check_order(&order).await;
    assert!(result.is_err());
    
    let violation = result.unwrap_err();
    assert_eq!(violation.rule, "PositionSizeLimit");
    
    // Test with valid order size
    let valid_order = NewOrder {
        symbol: "BTCUSDT".to_string(),
        exchange_id: "binance".to_string(),
        side: OrderSide::Buy,
        order_type: crypto_hft::traits::OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        price: Some(Price::from_str("50000.0").unwrap()),
        quantity: Size::from_str("5.0").unwrap(),
        client_order_id: Some("test_order_2".to_string()),
    };
    
    // Should pass
    let result = risk_engine.check_order(&valid_order).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_order_size_rule() {
    let risk_engine = RiskEngine::new();
    let mut rule = OrderSizeRule::new();
    
    // Set maximum order size
    risk_engine.set_max_order_size("BTCUSDT", Size::from_str("5.0").unwrap()).await;
    rule.set_max_order("BTCUSDT", Size::from_str("5.0").unwrap());
    
    // Add rule to engine
    risk_engine.add_rule(Box::new(rule)).await;
    
    // Create order that exceeds size limit
    let order = NewOrder {
        symbol: "BTCUSDT".to_string(),
        exchange_id: "binance".to_string(),
        side: OrderSide::Buy,
        order_type: crypto_hft::traits::OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        price: Some(Price::from_str("50000.0").unwrap()),
        quantity: Size::from_str("10.0").unwrap(),
        client_order_id: Some("test_order_1".to_string()),
    };
    
    // Check should fail
    let result = risk_engine.check_order(&order).await;
    assert!(result.is_err());
    
    let violation = result.unwrap_err();
    assert_eq!(violation.rule, "OrderSizeLimit");
}

#[tokio::test]
async fn test_daily_loss_rule() {
    let risk_engine = RiskEngine::new();
    let mut rule = DailyLossRule::new();
    
    // Set maximum daily loss
    risk_engine.set_max_daily_loss("BTCUSDT", Price::from_str("1000.0").unwrap()).await;
    rule.set_max_loss("BTCUSDT", Price::from_str("1000.0").unwrap());
    
    // Add rule to engine
    risk_engine.add_rule(Box::new(rule)).await;
    
    // Set up a position that would result in a loss
    let position = Position {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        size: Size::from_str("1.0").unwrap(),
        average_price: Some(Price::from_str("50000.0").unwrap()),
        unrealized_pnl: Some(crate::rust_decimal::Decimal::from_str("-500.0").unwrap()),
    };
    
    risk_engine.update_position("BTCUSDT", position).await;
    
    // Record existing loss
    risk_engine.record_daily_loss("BTCUSDT", Price::from_str("500.0").unwrap()).await;
    
    // Create sell order that would exceed daily loss limit
    let order = NewOrder {
        symbol: "BTCUSDT".to_string(),
        exchange_id: "binance".to_string(),
        side: OrderSide::Sell,
        order_type: crypto_hft::traits::OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        price: Some(Price::from_str("49000.0").unwrap()), // $1000 loss
        quantity: Size::from_str("1.0").unwrap(),
        client_order_id: Some("test_order_1".to_string()),
    };
    
    // Check should fail
    let result = risk_engine.check_order(&order).await;
    assert!(result.is_err());
    
    let violation = result.unwrap_err();
    assert_eq!(violation.rule, "DailyLossLimit");
}

#[tokio::test]
async fn test_total_exposure_rule() {
    let risk_engine = RiskEngine::new();
    let rule = TotalExposureRule::new(Price::from_str("100000.0").unwrap());
    
    // Add rule to engine
    risk_engine.add_rule(Box::new(rule)).await;
    
    // Set up a position with high exposure
    let position = Position {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        size: Size::from_str("2.0").unwrap(),
        average_price: Some(Price::from_str("50000.0").unwrap()),
        unrealized_pnl: Some(crate::rust_decimal::Decimal::ZERO),
    };
    
    risk_engine.update_position("BTCUSDT", position).await;
    
    // Create buy order that would exceed total exposure limit
    let order = NewOrder {
        symbol: "BTCUSDT".to_string(),
        exchange_id: "binance".to_string(),
        side: OrderSide::Buy,
        order_type: crypto_hft::traits::OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        price: Some(Price::from_str("50000.0").unwrap()),
        quantity: Size::from_str("1.0").unwrap(),
        client_order_id: Some("test_order_1".to_string()),
    };
    
    // Check should fail
    let result = risk_engine.check_order(&order).await;
    assert!(result.is_err());
    
    let violation = result.unwrap_err();
    assert_eq!(violation.rule, "TotalExposureLimit");
}

#[tokio::test]
async fn test_open_orders_count_rule() {
    let risk_engine = RiskEngine::new();
    let rule = OpenOrdersCountRule::new(2);
    
    // Add rule to engine
    risk_engine.add_rule(Box::new(rule)).await;
    
    // Set open orders count to limit
    risk_engine.increment_open_orders().await;
    risk_engine.increment_open_orders().await;
    assert_eq!(risk_engine.get_open_orders_count().await, 2);
    
    // Create order that would exceed open orders limit
    let order = NewOrder {
        symbol: "BTCUSDT".to_string(),
        exchange_id: "binance".to_string(),
        side: OrderSide::Buy,
        order_type: crypto_hft::traits::OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        price: Some(Price::from_str("50000.0").unwrap()),
        quantity: Size::from_str("1.0").unwrap(),
        client_order_id: Some("test_order_1".to_string()),
    };
    
    // Check should fail
    let result = risk_engine.check_order(&order).await;
    assert!(result.is_err());
    
    let violation = result.unwrap_err();
    assert_eq!(violation.rule, "OpenOrdersCountLimit");
}

#[tokio::test]
async fn test_balance_rule() {
    let risk_engine = RiskEngine::new();
    let mut rule = BalanceRule::new();
    
    // Set minimum balance
    risk_engine.update_balance("USDT", Size::from_str("1000.0").unwrap()).await;
    rule.set_min_balance("USDT", Size::from_str("500.0").unwrap());
    
    // Add rule to engine
    risk_engine.add_rule(Box::new(rule)).await;
    
    // Create buy order that requires more USDT than available
    let order = NewOrder {
        symbol: "BTCUSDT".to_string(),
        exchange_id: "binance".to_string(),
        side: OrderSide::Buy,
        order_type: crypto_hft::traits::OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        price: Some(Price::from_str("50000.0").unwrap()),
        quantity: Size::from_str("0.02").unwrap(), // Requires 1000 USDT but only 500 available after min balance
        client_order_id: Some("test_order_1".to_string()),
    };
    
    // Check should fail
    let result = risk_engine.check_order(&order).await;
    assert!(result.is_err());
    
    let violation = result.unwrap_err();
    assert_eq!(violation.rule, "InsufficientBalance");
}

#[tokio::test]
async fn test_multiple_rules() {
    let risk_engine = RiskEngine::new();
    
    // Add multiple rules
    let mut position_rule = PositionSizeRule::new();
    position_rule.set_max_position("BTCUSDT", Size::from_str("10.0").unwrap());
    risk_engine.add_rule(Box::new(position_rule)).await;
    
    let mut order_size_rule = OrderSizeRule::new();
    order_size_rule.set_max_order("BTCUSDT", Size::from_str("5.0").unwrap());
    risk_engine.add_rule(Box::new(order_size_rule)).await;
    
    // Create order that passes position rule but fails order size rule
    let order = NewOrder {
        symbol: "BTCUSDT".to_string(),
        exchange_id: "binance".to_string(),
        side: OrderSide::Buy,
        order_type: crypto_hft::traits::OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        price: Some(Price::from_str("50000.0").unwrap()),
        quantity: Size::from_str("7.0").unwrap(), // Exceeds max order size but not max position
        client_order_id: Some("test_order_1".to_string()),
    };
    
    // Check should fail due to order size rule
    let result = risk_engine.check_order(&order).await;
    assert!(result.is_err());
    
    let violation = result.unwrap_err();
    assert_eq!(violation.rule, "OrderSizeLimit");
    
    // Create order that passes both rules
    let valid_order = NewOrder {
        symbol: "BTCUSDT".to_string(),
        exchange_id: "binance".to_string(),
        side: OrderSide::Buy,
        order_type: crypto_hft::traits::OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        price: Some(Price::from_str("50000.0").unwrap()),
        quantity: Size::from_str("3.0").unwrap(), // Passes both rules
        client_order_id: Some("test_order_2".to_string()),
    };
    
    // Check should pass
    let result = risk_engine.check_order(&valid_order).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_risk_engine_rule_removal() {
    let risk_engine = RiskEngine::new();
    
    // Add a rule
    let mut rule = OrderSizeRule::new();
    rule.set_max_order("BTCUSDT", Size::from_str("5.0").unwrap());
    risk_engine.add_rule(Box::new(rule)).await;
    
    // Verify rule is active
    let order = NewOrder {
        symbol: "BTCUSDT".to_string(),
        exchange_id: "binance".to_string(),
        side: OrderSide::Buy,
        order_type: crypto_hft::traits::OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        price: Some(Price::from_str("50000.0").unwrap()),
        quantity: Size::from_str("10.0").unwrap(),
        client_order_id: Some("test_order_1".to_string()),
    };
    
    let result = risk_engine.check_order(&order).await;
    assert!(result.is_err());
}

