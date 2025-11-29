//! TDD Tests for Phase 8: Order Management System Fixes
//!
//! These tests verify the fixes for:
//! - T064: OrderInfo mutability for update calls
//! - T065: ExecutionReport field names (average_price vs average_fill_price)
//! - T066: ToPrimitive import for fill_percentage()
//! - T067: AdaptiveRateLimiter notify_rate_limit_hit() with interior mutability
//! - T068: AdaptiveRateLimiter reset() with interior mutability

use crypto_hft::oms::rate_limiter::{AdaptiveRateLimiter, RateLimiter};
use crypto_hft::oms::{order_manager::OrderInfo, OrderManagerImpl};
use crypto_hft::traits::{
    ExecutionReport, OrderManager, OrderSide, OrderStatus, OrderType, TimeInForce,
};
use crypto_hft::types::{Price, Size, Symbol};
use std::time::Duration;

// =============================================================================
// T064: OrderInfo Mutability Tests
// =============================================================================

#[test]
fn test_order_info_creation_and_update() {
    // Test that OrderInfo can be created and updated (requires mut)
    let mut order_info = OrderInfo::new(
        "order_123".to_string(),
        Some("client_123".to_string()),
        Symbol::new("BTCUSDT"),
        OrderSide::Buy,
        OrderType::Limit,
        TimeInForce::GoodTillCancelled,
        Size::from_str("1.0").unwrap(),
        Some(Price::from_str("50000.0").unwrap()),
        "binance".to_string(),
    );

    assert_eq!(order_info.status, OrderStatus::New);
    assert!(order_info.is_active());

    // Create an ExecutionReport with partial fill
    let report = ExecutionReport {
        order_id: "order_123".to_string(),
        client_order_id: Some("client_123".to_string()),
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        status: OrderStatus::PartiallyFilled,
        filled_size: Size::from_str("0.5").unwrap(),
        remaining_size: Size::from_str("0.5").unwrap(),
        average_price: Some(Price::from_str("50000.0").unwrap()),
        timestamp: 1638368000000,
    };

    // Update should work on mutable order_info
    order_info.update(&report);

    assert_eq!(order_info.status, OrderStatus::PartiallyFilled);
    assert_eq!(order_info.filled_quantity, Size::from_str("0.5").unwrap());
    assert!(order_info.is_active());
}

// =============================================================================
// T065: ExecutionReport Field Tests (average_price, not average_fill_price)
// =============================================================================

#[test]
fn test_execution_report_average_price_field() {
    // Test that ExecutionReport uses average_price (not average_fill_price)
    let report = ExecutionReport {
        order_id: "order_123".to_string(),
        client_order_id: Some("client_123".to_string()),
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        status: OrderStatus::Filled,
        filled_size: Size::from_str("1.0").unwrap(),
        remaining_size: Size::from_str("0.0").unwrap(),
        average_price: Some(Price::from_str("50000.0").unwrap()),
        timestamp: 1638368000000,
    };

    // Verify average_price field exists and is correct
    assert_eq!(
        report.average_price,
        Some(Price::from_str("50000.0").unwrap())
    );
    assert_eq!(report.filled_size, Size::from_str("1.0").unwrap());
    assert_eq!(report.remaining_size, Size::from_str("0.0").unwrap());
}

#[test]
fn test_order_info_update_with_average_price() {
    let mut order_info = OrderInfo::new(
        "order_123".to_string(),
        Some("client_123".to_string()),
        Symbol::new("BTCUSDT"),
        OrderSide::Buy,
        OrderType::Limit,
        TimeInForce::GoodTillCancelled,
        Size::from_str("1.0").unwrap(),
        Some(Price::from_str("50000.0").unwrap()),
        "binance".to_string(),
    );

    // Create an ExecutionReport with filled status
    let report = ExecutionReport {
        order_id: "order_123".to_string(),
        client_order_id: Some("client_123".to_string()),
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        status: OrderStatus::Filled,
        filled_size: Size::from_str("1.0").unwrap(),
        remaining_size: Size::from_str("0.0").unwrap(),
        average_price: Some(Price::from_str("50100.0").unwrap()),
        timestamp: 1638368000000,
    };

    order_info.update(&report);

    // Average fill price should be updated from report.average_price
    assert_eq!(
        order_info.average_fill_price,
        Some(Price::from_str("50100.0").unwrap())
    );
    assert_eq!(order_info.status, OrderStatus::Filled);
    assert!(order_info.is_filled());
}

// =============================================================================
// T066: ToPrimitive for fill_percentage() Tests
// =============================================================================

#[test]
fn test_order_info_fill_percentage() {
    let mut order_info = OrderInfo::new(
        "order_123".to_string(),
        Some("client_123".to_string()),
        Symbol::new("BTCUSDT"),
        OrderSide::Buy,
        OrderType::Limit,
        TimeInForce::GoodTillCancelled,
        Size::from_str("1.0").unwrap(),
        Some(Price::from_str("50000.0").unwrap()),
        "binance".to_string(),
    );

    // Initially 0% filled
    assert_eq!(order_info.fill_percentage(), 0.0);

    // Partial fill - 50%
    let report = ExecutionReport {
        order_id: "order_123".to_string(),
        client_order_id: Some("client_123".to_string()),
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        status: OrderStatus::PartiallyFilled,
        filled_size: Size::from_str("0.5").unwrap(),
        remaining_size: Size::from_str("0.5").unwrap(),
        average_price: Some(Price::from_str("50000.0").unwrap()),
        timestamp: 1638368000000,
    };

    order_info.update(&report);
    assert_eq!(order_info.fill_percentage(), 50.0);

    // Full fill - 100%
    let report = ExecutionReport {
        order_id: "order_123".to_string(),
        client_order_id: Some("client_123".to_string()),
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        status: OrderStatus::Filled,
        filled_size: Size::from_str("1.0").unwrap(),
        remaining_size: Size::from_str("0.0").unwrap(),
        average_price: Some(Price::from_str("50000.0").unwrap()),
        timestamp: 1638368000000,
    };

    order_info.update(&report);
    assert_eq!(order_info.fill_percentage(), 100.0);
}

#[test]
fn test_order_info_fill_percentage_zero_quantity() {
    // Edge case: zero quantity order should return 0% fill
    let order_info = OrderInfo::new(
        "order_123".to_string(),
        Some("client_123".to_string()),
        Symbol::new("BTCUSDT"),
        OrderSide::Buy,
        OrderType::Limit,
        TimeInForce::GoodTillCancelled,
        Size::new(rust_decimal::Decimal::ZERO),
        Some(Price::from_str("50000.0").unwrap()),
        "binance".to_string(),
    );

    assert_eq!(order_info.fill_percentage(), 0.0);
}

// =============================================================================
// T067: AdaptiveRateLimiter notify_rate_limit_hit() with Interior Mutability Tests
// =============================================================================

#[test]
fn test_adaptive_rate_limiter_notify_rate_limit_hit() {
    // No mut needed - uses interior mutability via Mutex
    let limiter = AdaptiveRateLimiter::new(10, Duration::from_secs(1));

    // Initial backoff multiplier should be 1.0
    assert_eq!(limiter.backoff_multiplier(), 1.0);

    // Notify rate limit hit - backoff should increase
    limiter.notify_rate_limit_hit();
    assert!(limiter.backoff_multiplier() > 1.0);

    // Store the current backoff
    let first_backoff = limiter.backoff_multiplier();

    // Notify again - backoff should increase further
    limiter.notify_rate_limit_hit();
    assert!(limiter.backoff_multiplier() > first_backoff);
}

#[test]
fn test_adaptive_rate_limiter_backoff_has_max() {
    // No mut needed - uses interior mutability via Mutex
    let limiter = AdaptiveRateLimiter::new(10, Duration::from_secs(1));

    // Notify rate limit hit many times
    for _ in 0..20 {
        limiter.notify_rate_limit_hit();
    }

    // Backoff should be capped at max (10.0)
    assert!(limiter.backoff_multiplier() <= 10.0);
}

// =============================================================================
// T068: AdaptiveRateLimiter reset() with Interior Mutability Tests
// =============================================================================

#[test]
fn test_adaptive_rate_limiter_reset() {
    // No mut needed - uses interior mutability via Mutex
    let limiter = AdaptiveRateLimiter::new(10, Duration::from_secs(1));

    // Increase backoff
    limiter.notify_rate_limit_hit();
    limiter.notify_rate_limit_hit();
    assert!(limiter.backoff_multiplier() > 1.0);

    // Reset should restore backoff to 1.0
    limiter.reset();
    assert_eq!(limiter.backoff_multiplier(), 1.0);
}

// =============================================================================
// OrderManager Integration Tests
// =============================================================================

#[tokio::test]
async fn test_order_manager_handle_execution_report() {
    let mut order_manager = OrderManagerImpl::new("binance".to_string());

    // Create and add an order
    let order_info = OrderInfo::new(
        "order_123".to_string(),
        Some("client_123".to_string()),
        Symbol::new("BTCUSDT"),
        OrderSide::Buy,
        OrderType::Limit,
        TimeInForce::GoodTillCancelled,
        Size::from_str("1.0").unwrap(),
        Some(Price::from_str("50000.0").unwrap()),
        "binance".to_string(),
    );
    order_manager.add_order(order_info).await;

    // Handle execution report for partial fill
    let report = ExecutionReport {
        order_id: "order_123".to_string(),
        client_order_id: Some("client_123".to_string()),
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        status: OrderStatus::PartiallyFilled,
        filled_size: Size::from_str("0.5").unwrap(),
        remaining_size: Size::from_str("0.5").unwrap(),
        average_price: Some(Price::from_str("50000.0").unwrap()),
        timestamp: 1638368000000,
    };

    order_manager.handle_execution_report(report).await.unwrap();

    // Verify order was updated
    let order = order_manager
        .get_order(&"order_123".to_string())
        .await
        .unwrap();
    assert_eq!(order.status, OrderStatus::PartiallyFilled);
    assert_eq!(order.filled_quantity, Size::from_str("0.5").unwrap());
}

#[tokio::test]
async fn test_order_manager_cancelled_order() {
    let mut order_manager = OrderManagerImpl::new("binance".to_string());

    // Create and add an order
    let order_info = OrderInfo::new(
        "order_123".to_string(),
        Some("client_123".to_string()),
        Symbol::new("BTCUSDT"),
        OrderSide::Buy,
        OrderType::Limit,
        TimeInForce::GoodTillCancelled,
        Size::from_str("1.0").unwrap(),
        Some(Price::from_str("50000.0").unwrap()),
        "binance".to_string(),
    );
    order_manager.add_order(order_info).await;

    // Handle execution report for cancel (note: Cancelled, not Canceled)
    let report = ExecutionReport {
        order_id: "order_123".to_string(),
        client_order_id: Some("client_123".to_string()),
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        status: OrderStatus::Cancelled,
        filled_size: Size::from_str("0.0").unwrap(),
        remaining_size: Size::from_str("1.0").unwrap(),
        average_price: None,
        timestamp: 1638368000000,
    };

    order_manager.handle_execution_report(report).await.unwrap();

    // Verify order was updated
    let order = order_manager
        .get_order(&"order_123".to_string())
        .await
        .unwrap();
    assert_eq!(order.status, OrderStatus::Cancelled);
    assert!(order.is_canceled());
}

// =============================================================================
// RateLimiter Basic Tests
// =============================================================================

#[tokio::test]
async fn test_rate_limiter_basic() {
    let limiter = RateLimiter::new(5, Duration::from_millis(100));

    // Should allow the first 5 requests
    for _ in 0..5 {
        assert!(limiter.check_limit().await);
    }

    // Should reject the 6th request
    assert!(!limiter.check_limit().await);
}

#[test]
fn test_rate_limiter_reset() {
    let limiter = RateLimiter::new(5, Duration::from_secs(1));

    // Fill up the limiter
    for _ in 0..5 {
        let _ = futures::executor::block_on(limiter.check_limit());
    }
    assert_eq!(limiter.current_requests(), 5);

    // Reset
    limiter.reset();
    assert_eq!(limiter.current_requests(), 0);
}

// =============================================================================
// OrderStatus Tests
// =============================================================================

#[test]
fn test_order_status_variants() {
    // Test that OrderStatus has the expected variants (simple enum, no fields)
    let status_new = OrderStatus::New;
    let status_partial = OrderStatus::PartiallyFilled;
    let status_filled = OrderStatus::Filled;
    let status_cancelled = OrderStatus::Cancelled;
    let status_rejected = OrderStatus::Rejected;
    let status_expired = OrderStatus::Expired;

    // All should be valid variants
    assert_eq!(status_new, OrderStatus::New);
    assert_eq!(status_partial, OrderStatus::PartiallyFilled);
    assert_eq!(status_filled, OrderStatus::Filled);
    assert_eq!(status_cancelled, OrderStatus::Cancelled);
    assert_eq!(status_rejected, OrderStatus::Rejected);
    assert_eq!(status_expired, OrderStatus::Expired);
}
