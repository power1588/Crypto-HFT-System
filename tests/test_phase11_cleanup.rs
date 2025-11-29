//! Phase 11 TDD tests for Cleanup and Optimization
//!
//! These tests verify that the cleanup work in Phase 11 maintains code quality
//! and doesn't break existing functionality.

use crypto_hft::*;
use rust_decimal::Decimal;

/// T074: Test that removed unused imports don't break the build
mod t074_unused_imports {
    use super::*;

    #[test]
    fn test_core_events_types_available() {
        // Verify core event types are still available after import cleanup
        let _price = Price::new(Decimal::new(100, 0));
        let _size = Size::new(Decimal::new(1, 0));
        let _symbol = Symbol::new("BTCUSDT");
    }

    #[test]
    fn test_orderbook_types_available() {
        // Verify orderbook types are still available
        let _book = OrderBook::new("BTCUSDT".to_string());
    }

    #[test]
    fn test_trait_types_available() {
        // Verify trait types are exported
        let _status = OrderStatus::New;
        let _side = OrderSide::Buy;
        let _tif = TimeInForce::GoodTillCancelled;
    }
}

/// T075: Test that underscore-prefixed variables don't cause issues
mod t075_unused_variables {
    use super::*;

    #[test]
    fn test_strategy_works_with_unused_params() {
        // Test that strategies work correctly with unused parameter prefixes
        use crypto_hft::strategy::MarketState;

        let market_state = MarketState::new("BTCUSDT".to_string());
        assert_eq!(market_state.symbol, "BTCUSDT");
    }

    #[test]
    fn test_price_size_operations() {
        let price = Price::new(Decimal::new(10000, 2)); // 100.00
        let size = Size::new(Decimal::new(100, 2)); // 1.00

        // Test that arithmetic operations work
        let value = price * size;
        assert!(value > Decimal::ZERO);
    }
}

/// T076: Test that allow(dead_code) doesn't hide real issues
mod t076_dead_code {
    use super::*;

    #[test]
    fn test_risk_engine_creation() {
        // Verify RiskEngine is still functional
        let engine = RiskEngine::new();
        // Engine should be created without errors
        drop(engine);
    }

    #[test]
    fn test_orderbook_functionality() {
        let mut book = OrderBook::new("BTCUSDT".to_string());
        assert_eq!(book.symbol(), "BTCUSDT");
        assert!(book.best_bid().is_none());
        assert!(book.best_ask().is_none());
    }
}

/// T077: Test that cargo fmt doesn't break functionality
mod t077_formatting {
    use super::*;

    #[test]
    fn test_complex_type_signatures_work() {
        // Test that complex type signatures work after formatting
        let price = Price::from_str("100.50").unwrap();
        let size = Size::from_str("1.5").unwrap();

        assert_eq!(price.value(), Decimal::new(10050, 2));
        assert_eq!(size.value(), Decimal::new(15, 1));
    }

    #[test]
    fn test_multi_line_function_calls() {
        // Test that multi-line formatted code works
        let order = NewOrder::new_limit_buy(
            "BTCUSDT".to_string(),
            Size::from_str("1.0").unwrap(),
            Price::from_str("50000.0").unwrap(),
            TimeInForce::GoodTillCancelled,
        );

        assert_eq!(order.symbol.value(), "BTCUSDT");
        assert_eq!(order.side, OrderSide::Buy);
    }
}

/// T078: Test that clippy suggestions don't break functionality
mod t078_clippy {
    use super::*;

    #[test]
    fn test_default_implementations() {
        // Test types that have Default implementations
        let config = crypto_hft::realtime::event_loop::EventLoopConfig::default();
        assert!(!config.symbols.is_empty());
    }

    #[test]
    fn test_from_str_methods() {
        // Test that from_str methods work (clippy suggests FromStr trait)
        let price = Price::from_str("100.00").unwrap();
        assert_eq!(price.value(), Decimal::new(10000, 2));

        let size = Size::from_str("1.5").unwrap();
        assert_eq!(size.value(), Decimal::new(15, 1));
    }
}

/// T079: Test public API contract compliance
mod t079_api_contract {
    use super::*;

    #[test]
    fn test_price_type_api() {
        // Test Price type implements expected API
        let price = Price::new(Decimal::new(100, 0));
        assert_eq!(price.value(), Decimal::new(100, 0));

        // Test from_str
        let price2 = Price::from_str("100.50").unwrap();
        assert_eq!(price2.value(), Decimal::new(10050, 2));

        // Test arithmetic
        let sum = price + price2;
        assert!(sum.value() > Decimal::ZERO);

        let diff = price2 - price;
        assert_eq!(diff.value(), Decimal::new(50, 2));

        // Test abs
        let neg_price = -price;
        assert_eq!(neg_price.abs(), price);
    }

    #[test]
    fn test_size_type_api() {
        // Test Size type implements expected API
        let size = Size::new(Decimal::new(100, 0));
        assert_eq!(size.value(), Decimal::new(100, 0));

        // Test zero
        let zero = Size::zero();
        assert!(zero.is_zero());

        // Test from_str
        let size2 = Size::from_str("1.5").unwrap();
        assert_eq!(size2.value(), Decimal::new(15, 1));

        // Test negation
        let neg_size = -size;
        assert_eq!(neg_size.abs(), size);
    }

    #[test]
    fn test_symbol_type_api() {
        // Test Symbol type implements expected API
        let symbol = Symbol::new("BTCUSDT");
        assert_eq!(symbol.value(), "BTCUSDT");
        assert_eq!(symbol.as_str(), "BTCUSDT");
        assert_eq!(symbol.len(), 7);
        assert!(!symbol.is_empty());
    }

    #[test]
    fn test_order_types_api() {
        // Test order type enums
        assert_ne!(OrderType::Market, OrderType::Limit);
        assert_ne!(OrderType::StopLoss, OrderType::StopLimit);

        // Test order status
        assert_ne!(OrderStatus::New, OrderStatus::Filled);
        assert_ne!(OrderStatus::Cancelled, OrderStatus::Rejected);

        // Test order side
        assert_ne!(OrderSide::Buy, OrderSide::Sell);

        // Test time in force
        assert_ne!(TimeInForce::GoodTillCancelled, TimeInForce::ImmediateOrCancel);
    }

    #[test]
    fn test_new_order_builder_api() {
        // Test NewOrder builder methods
        let order = NewOrder::new_limit_buy(
            "BTCUSDT",
            Size::from_str("1.0").unwrap(),
            Price::from_str("50000.0").unwrap(),
            TimeInForce::GoodTillCancelled,
        )
        .with_client_order_id("test-123".to_string())
        .with_exchange_id("binance");

        assert_eq!(order.symbol.value(), "BTCUSDT");
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.order_type, OrderType::Limit);
        assert_eq!(order.client_order_id, Some("test-123".to_string()));
        assert_eq!(order.exchange_id, "binance");
    }
}

/// Integration test: Full workflow after cleanup
mod integration_after_cleanup {
    use super::*;

    #[test]
    fn test_full_order_workflow() {
        // Create an order
        let order = NewOrder::new_market_buy("BTCUSDT", Size::from_str("0.1").unwrap());

        // Verify order structure
        assert_eq!(order.symbol.value(), "BTCUSDT");
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.order_type, OrderType::Market);
        assert!(order.price.is_none()); // Market orders have no price
    }

    #[test]
    fn test_orderbook_workflow() {
        let mut book = OrderBook::new("BTCUSDT".to_string());

        // Apply a snapshot
        let snapshot = OrderBookSnapshot::new(
            "BTCUSDT".to_string(),
            "binance",
            vec![OrderBookLevel::new(
                Price::from_str("100.00").unwrap(),
                Size::from_str("10.0").unwrap(),
            )],
            vec![OrderBookLevel::new(
                Price::from_str("101.00").unwrap(),
                Size::from_str("5.0").unwrap(),
            )],
            123456789,
        );

        book.apply_snapshot(snapshot);

        // Verify orderbook state
        let (bid_price, _) = book.best_bid().unwrap();
        let (ask_price, _) = book.best_ask().unwrap();

        assert_eq!(bid_price, Price::from_str("100.00").unwrap());
        assert_eq!(ask_price, Price::from_str("101.00").unwrap());

        // Verify spread
        let spread = book.spread().unwrap();
        assert_eq!(spread, Price::from_str("1.00").unwrap());
    }

    #[test]
    fn test_risk_rule_workflow() {
        let engine = RiskEngine::new();

        // Verify engine is functional
        drop(engine);
    }
}

