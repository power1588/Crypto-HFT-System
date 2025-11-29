//! Phase 7 TDD Tests - Risk Management Fixes
//!
//! These tests verify the fixes for Phase 7 compilation errors in risk management.

use crypto_hft::types::{Price, Size, Symbol};
use crypto_hft::core::events::{NewOrder, OrderSide, TimeInForce, Position, RiskViolation};
use crypto_hft::risk::{RiskEngine, RiskRule};
use rust_decimal::Decimal;
use std::str::FromStr;

/// T059: Test Symbol API - len(), as_str(), and slicing for risk rules
#[test]
fn test_symbol_len_and_slicing() {
    let symbol = Symbol::new("BTCUSDT");
    
    // Test len() method
    assert_eq!(symbol.len(), 7);
    assert!(!symbol.is_empty());
    
    // Test as_str() method
    assert_eq!(symbol.as_str(), "BTCUSDT");
    
    // Test value() method
    assert_eq!(symbol.value(), "BTCUSDT");
    
    // For extracting base/quote assets, use as_str() or value()
    let symbol_str = symbol.as_str();
    if symbol_str.len() >= 4 {
        let base_asset = &symbol_str[..symbol_str.len() - 4]; // BTC
        let quote_asset = &symbol_str[symbol_str.len() - 4..]; // USDT
        assert_eq!(base_asset, "BTC");
        assert_eq!(quote_asset, "USDT");
    }
}

/// T060: Test Price vs Size operations for balance calculations
#[test]
fn test_price_size_operations_for_balance() {
    let price = Price::from_str("50000.0").unwrap();
    let size = Size::from_str("1.5").unwrap();
    
    // Price * Size returns Decimal
    let order_value: Decimal = price * size;
    assert_eq!(order_value, Decimal::from_str("75000.0").unwrap());
    
    // For balance checks, we compare Size values
    let current_balance = Size::from_str("100000.0").unwrap();
    let min_balance = Size::from_str("1000.0").unwrap();
    
    // Convert order_value (Decimal) to Size for comparison
    let required_balance = Size::new(order_value);
    
    // Check if we have enough balance
    let available = current_balance - min_balance;
    assert!(available >= required_balance);
}

/// T061: Test daily loss calculation with proper Price types
#[test]
fn test_daily_loss_with_price_types() {
    let sell_price = Price::from_str("49000.0").unwrap();
    let avg_price = Price::from_str("50000.0").unwrap();
    let order_size = Size::from_str("1.0").unwrap();
    
    // Price difference is Price
    let loss_per_unit: Price = avg_price - sell_price; // $1000 loss per unit
    assert_eq!(loss_per_unit.value(), Decimal::from_str("1000.0").unwrap());
    
    // Price * Size returns Decimal, wrap in Price for daily loss tracking
    let potential_loss_decimal: Decimal = loss_per_unit * order_size;
    let potential_loss = Price::new(potential_loss_decimal);
    assert_eq!(potential_loss.value(), Decimal::from_str("1000.0").unwrap());
    
    // Daily loss comparisons use Price
    let current_daily_loss = Price::from_str("500.0").unwrap();
    let max_daily_loss = Price::from_str("2000.0").unwrap();
    
    // Check if adding potential loss exceeds limit
    let total_loss = current_daily_loss + potential_loss;
    assert!(total_loss <= max_daily_loss);
}

/// T062: Test exposure calculation with proper type conversions
#[test]
fn test_exposure_calculation() {
    let price = Price::from_str("50000.0").unwrap();
    let size = Size::from_str("2.0").unwrap();
    
    let current_exposure = Price::from_str("100000.0").unwrap();
    
    // Price * Size returns Decimal
    let order_value: Decimal = price * size;
    
    // Wrap in Price for exposure addition
    let order_exposure = Price::new(order_value);
    
    // Add to current exposure
    let new_exposure = current_exposure + order_exposure;
    assert_eq!(new_exposure.value(), Decimal::from_str("200000.0").unwrap());
}

/// Test RiskEngine creation and basic operations
#[tokio::test]
async fn test_risk_engine_basic_operations() {
    let risk_engine = RiskEngine::new();
    
    // Test initial state
    assert_eq!(risk_engine.get_open_orders_count().await, 0);
    
    // Test balance operations
    risk_engine.update_balance("USDT", Size::from_str("10000.0").unwrap()).await;
    let balance = risk_engine.get_balance("USDT").await;
    assert_eq!(balance, Size::from_str("10000.0").unwrap());
    
    // Test position operations
    let position = Position {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        size: Size::from_str("1.0").unwrap(),
        average_price: Some(Price::from_str("50000.0").unwrap()),
        unrealized_pnl: None,
    };
    
    risk_engine.update_position("BTCUSDT", position.clone()).await;
    let retrieved = risk_engine.get_position("BTCUSDT").await;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().size, Size::from_str("1.0").unwrap());
}

/// Test order checking with Symbol API
#[tokio::test]
async fn test_order_check_with_symbol_api() {
    let risk_engine = RiskEngine::new();
    
    // Set max position size using symbol string
    risk_engine.set_max_position_size("BTCUSDT", Size::from_str("10.0").unwrap()).await;
    
    // Create order using Symbol
    let order = NewOrder::new_limit_buy(
        "BTCUSDT",
        Size::from_str("5.0").unwrap(),
        Price::from_str("50000.0").unwrap(),
        TimeInForce::GoodTillCancelled,
    );
    
    // Order should pass (within limits)
    let result = risk_engine.check_order(&order).await;
    assert!(result.is_ok());
}

/// Test position limit enforcement
#[tokio::test]
async fn test_position_limit_enforcement() {
    use crypto_hft::risk::rules::PositionSizeRule;
    
    let risk_engine = RiskEngine::new();
    
    // Set max position size
    risk_engine.set_max_position_size("BTCUSDT", Size::from_str("10.0").unwrap()).await;
    
    // Add rule
    let mut rule = PositionSizeRule::new();
    rule.set_max_position("BTCUSDT", Size::from_str("10.0").unwrap());
    risk_engine.add_rule(Box::new(rule)).await;
    
    // Create order that would exceed limit
    let order = NewOrder::new_limit_buy(
        "BTCUSDT",
        Size::from_str("15.0").unwrap(),
        Price::from_str("50000.0").unwrap(),
        TimeInForce::GoodTillCancelled,
    );
    
    // Order should fail
    let result = risk_engine.check_order(&order).await;
    assert!(result.is_err());
    
    let violation = result.unwrap_err();
    assert_eq!(violation.rule, "PositionSizeLimit");
}

/// Test balance rule with proper type conversions
#[tokio::test]
async fn test_balance_rule_type_conversions() {
    use crypto_hft::risk::rules::BalanceRule;
    
    let risk_engine = RiskEngine::new();
    
    // Set balance (note: BalanceRule extracts base asset from symbol)
    risk_engine.update_balance("BTC", Size::from_str("1.0").unwrap()).await;
    
    // Add balance rule
    let mut rule = BalanceRule::new();
    rule.set_min_balance("BTC", Size::from_str("0.5").unwrap());
    risk_engine.add_rule(Box::new(rule)).await;
    
    // Create buy order (requires balance)
    let order = NewOrder::new_limit_buy(
        "BTCUSDT",
        Size::from_str("0.1").unwrap(), // Small order
        Price::from_str("50000.0").unwrap(),
        TimeInForce::GoodTillCancelled,
    );
    
    // Check should work (balance rule extracts asset from symbol)
    let result = risk_engine.check_order(&order).await;
    // Note: May pass or fail depending on implementation - this tests the type conversions work
    println!("Balance rule check result: {:?}", result);
}

/// Test exposure rule calculations
#[tokio::test]
async fn test_exposure_rule_calculations() {
    use crypto_hft::risk::rules::TotalExposureRule;
    
    let risk_engine = RiskEngine::new();
    
    // Set up position with exposure
    let position = Position {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        size: Size::from_str("1.0").unwrap(),
        average_price: Some(Price::from_str("50000.0").unwrap()),
        unrealized_pnl: None,
    };
    risk_engine.update_position("BTCUSDT", position).await;
    
    // Add exposure rule - limit to 100k
    let rule = TotalExposureRule::new(Price::from_str("100000.0").unwrap());
    risk_engine.add_rule(Box::new(rule)).await;
    
    // Create order that would exceed exposure (current 50k + new 60k = 110k > 100k)
    let order = NewOrder::new_limit_buy(
        "BTCUSDT",
        Size::from_str("1.2").unwrap(),
        Price::from_str("50000.0").unwrap(),
        TimeInForce::GoodTillCancelled,
    );
    
    // Order should fail due to exposure limit
    let result = risk_engine.check_order(&order).await;
    assert!(result.is_err());
    
    let violation = result.unwrap_err();
    assert_eq!(violation.rule, "TotalExposureLimit");
}

/// Test daily loss rule
#[tokio::test]
async fn test_daily_loss_rule() {
    use crypto_hft::risk::rules::DailyLossRule;
    
    let risk_engine = RiskEngine::new();
    
    // Set up position at a higher price
    let position = Position {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        size: Size::from_str("1.0").unwrap(),
        average_price: Some(Price::from_str("50000.0").unwrap()),
        unrealized_pnl: None,
    };
    risk_engine.update_position("BTCUSDT", position).await;
    
    // Set max daily loss
    risk_engine.set_max_daily_loss("BTCUSDT", Price::from_str("1000.0").unwrap()).await;
    
    // Add daily loss rule
    let mut rule = DailyLossRule::new();
    rule.set_max_loss("BTCUSDT", Price::from_str("1000.0").unwrap());
    risk_engine.add_rule(Box::new(rule)).await;
    
    // Create sell order at loss that would exceed daily limit
    let order = NewOrder::new_limit_sell(
        "BTCUSDT",
        Size::from_str("1.0").unwrap(),
        Price::from_str("48000.0").unwrap(), // $2000 loss > $1000 limit
        TimeInForce::GoodTillCancelled,
    );
    
    // Order should fail
    let result = risk_engine.check_order(&order).await;
    assert!(result.is_err());
    
    let violation = result.unwrap_err();
    assert_eq!(violation.rule, "DailyLossLimit");
}

/// Test rate of change rule with Symbol
#[tokio::test]
async fn test_rate_of_change_rule_with_symbol() {
    use crypto_hft::risk::rules::RateOfChangeLimitRule;
    
    let risk_engine = RiskEngine::new();
    
    // Set up position
    let position = Position {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        size: Size::from_str("5.0").unwrap(),
        average_price: Some(Price::from_str("50000.0").unwrap()),
        unrealized_pnl: None,
    };
    risk_engine.update_position("BTCUSDT", position).await;
    
    // Add rate of change rule
    let mut rule = RateOfChangeLimitRule::new(
        Size::from_str("2.0").unwrap(), // Max 2.0 change per period
        60, // 60 second period
    );
    rule.update_last_position("BTCUSDT", Size::from_str("5.0").unwrap());
    risk_engine.add_rule(Box::new(rule)).await;
    
    // Create order that would cause large position change
    let order = NewOrder::new_limit_buy(
        "BTCUSDT",
        Size::from_str("5.0").unwrap(), // Would change from 5 to 10
        Price::from_str("50000.0").unwrap(),
        TimeInForce::GoodTillCancelled,
    );
    
    // Order should fail due to rate of change limit
    let result = risk_engine.check_order(&order).await;
    assert!(result.is_err());
    
    let violation = result.unwrap_err();
    assert_eq!(violation.rule, "RateOfChangeLimit");
}

