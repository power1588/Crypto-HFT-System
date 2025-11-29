//! TDD Tests for Phase 9: Real-time Event Loop Fixes
//!
//! These tests verify the fixes for:
//! - T069: StrategyEngine::new() call signature in event_loop.rs
//! - T070: Generic type parameter usage for StrategyEngine<S>
//! - T071: Signal variants (PlaceOrder, CancelOrder, etc.)
//! - T072: Signal handling in signal_generator.rs

use crypto_hft::strategy::{MarketState, Signal, Strategy, StrategyEngine};
use crypto_hft::traits::{NewOrder, OrderSide, OrderType, TimeInForce};
use crypto_hft::types::{Price, Size};
use std::collections::HashMap;
use std::time::Duration;

/// Mock strategy for testing
struct MockStrategy {
    should_generate_signal: bool,
}

impl MockStrategy {
    fn new(should_generate_signal: bool) -> Self {
        Self {
            should_generate_signal,
        }
    }
}

impl Strategy for MockStrategy {
    fn generate_signal(&mut self, _market_state: &MarketState) -> Option<Signal> {
        if self.should_generate_signal {
            Some(Signal::Custom {
                name: "test_signal".to_string(),
                data: HashMap::new(),
            })
        } else {
            None
        }
    }
}

/// T069: Test StrategyEngine::new() takes strategy and duration parameters
#[test]
fn test_strategy_engine_new_signature() {
    let strategy = MockStrategy::new(true);
    let cooldown = Duration::from_millis(100);

    // StrategyEngine::new() should accept (strategy, signal_cooldown)
    let engine = StrategyEngine::new(strategy, cooldown);

    // Engine should be able to process events
    assert!(engine.get_market_state("BTCUSDT").is_none());
}

/// T070: Test StrategyEngine generic type parameter works correctly
#[test]
fn test_strategy_engine_generic_type() {
    let strategy = MockStrategy::new(true);
    let cooldown = Duration::from_millis(100);

    // StrategyEngine<S> should work with any S: Strategy
    let engine: StrategyEngine<MockStrategy> = StrategyEngine::new(strategy, cooldown);

    // Verify type is correct
    let _: &HashMap<String, MarketState> = engine.get_all_market_states();
}

/// T071: Test Signal enum has all required variants
#[test]
fn test_signal_variants_exist() {
    // Test PlaceOrder variant
    let order = NewOrder::new_limit_buy(
        "BTCUSDT".to_string(),
        Size::from_str("1.0").unwrap(),
        Price::from_str("50000.0").unwrap(),
        TimeInForce::GoodTillCancelled,
    );

    let place_order_signal = Signal::PlaceOrder {
        order: order.clone(),
    };
    assert!(matches!(place_order_signal, Signal::PlaceOrder { .. }));

    // Test CancelOrder variant
    let cancel_order_signal = Signal::CancelOrder {
        order_id: "order_123".to_string(),
        symbol: "BTCUSDT".to_string(),
        exchange_id: "binance".to_string(),
    };
    assert!(matches!(cancel_order_signal, Signal::CancelOrder { .. }));

    // Test CancelAllOrders variant
    let cancel_all_signal = Signal::CancelAllOrders {
        symbol: "BTCUSDT".to_string(),
        exchange_id: "binance".to_string(),
    };
    assert!(matches!(cancel_all_signal, Signal::CancelAllOrders { .. }));

    // Test UpdateOrder variant
    let update_order_signal = Signal::UpdateOrder {
        order_id: "order_123".to_string(),
        price: Some(Price::from_str("51000.0").unwrap()),
        size: Some(Size::from_str("0.5").unwrap()),
    };
    assert!(matches!(update_order_signal, Signal::UpdateOrder { .. }));

    // Test Arbitrage variant
    let arbitrage_signal = Signal::Arbitrage {
        buy_exchange: "binance".to_string(),
        sell_exchange: "coinbase".to_string(),
        symbol: "BTCUSDT".to_string(),
        buy_price: Price::from_str("50000.0").unwrap(),
        sell_price: Price::from_str("50100.0").unwrap(),
        quantity: Size::from_str("0.1").unwrap(),
        expected_profit: Price::from_str("10.0").unwrap(),
    };
    assert!(matches!(arbitrage_signal, Signal::Arbitrage { .. }));

    // Test Custom variant
    let custom_signal = Signal::Custom {
        name: "custom".to_string(),
        data: HashMap::new(),
    };
    assert!(matches!(custom_signal, Signal::Custom { .. }));
}

/// T072: Test TimeInForce uses correct variant names
#[test]
fn test_time_in_force_variants() {
    // TimeInForce should use GoodTillCancelled, not GTC
    let gtc = TimeInForce::GoodTillCancelled;
    assert!(matches!(gtc, TimeInForce::GoodTillCancelled));

    // Test other variants
    let ioc = TimeInForce::ImmediateOrCancel;
    assert!(matches!(ioc, TimeInForce::ImmediateOrCancel));

    let fok = TimeInForce::FillOrKill;
    assert!(matches!(fok, TimeInForce::FillOrKill));
}

/// Test StrategyEngine process_event method
#[test]
fn test_strategy_engine_process_event() {
    let strategy = MockStrategy::new(true);
    let cooldown = Duration::from_millis(100);
    let mut engine = StrategyEngine::new(strategy, cooldown);

    // Create a market event
    let snapshot = crypto_hft::orderbook::OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        vec![crypto_hft::orderbook::OrderBookLevel::new(
            Price::from_str("50000.0").unwrap(),
            Size::from_str("1.0").unwrap(),
        )],
        vec![crypto_hft::orderbook::OrderBookLevel::new(
            Price::from_str("50100.0").unwrap(),
            Size::from_str("1.0").unwrap(),
        )],
        12345678,
        "binance".to_string(),
    );

    let event = crypto_hft::traits::MarketEvent::OrderBookSnapshot(snapshot);

    // Process event should return a signal if strategy generates one
    let signal = engine.process_event(event);
    assert!(signal.is_some());

    // Second call should be blocked by cooldown
    let snapshot2 = crypto_hft::orderbook::OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        vec![crypto_hft::orderbook::OrderBookLevel::new(
            Price::from_str("50000.0").unwrap(),
            Size::from_str("1.0").unwrap(),
        )],
        vec![crypto_hft::orderbook::OrderBookLevel::new(
            Price::from_str("50100.0").unwrap(),
            Size::from_str("1.0").unwrap(),
        )],
        12345679,
        "binance".to_string(),
    );

    let event2 = crypto_hft::traits::MarketEvent::OrderBookSnapshot(snapshot2);
    let signal2 = engine.process_event(event2);
    assert!(signal2.is_none()); // Blocked by cooldown
}

/// Test MarketState updates correctly
#[test]
fn test_market_state_update() {
    let mut market_state = MarketState::new("BTCUSDT".to_string());

    // Create a snapshot
    let snapshot = crypto_hft::orderbook::OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        vec![crypto_hft::orderbook::OrderBookLevel::new(
            Price::from_str("50000.0").unwrap(),
            Size::from_str("1.0").unwrap(),
        )],
        vec![crypto_hft::orderbook::OrderBookLevel::new(
            Price::from_str("50100.0").unwrap(),
            Size::from_str("1.0").unwrap(),
        )],
        12345678,
        "binance".to_string(),
    );

    let event = crypto_hft::traits::MarketEvent::OrderBookSnapshot(snapshot);
    market_state.update(&event);

    // Verify best bid and ask
    let (bid_price, bid_size) = market_state.best_bid().unwrap();
    assert_eq!(bid_price, Price::from_str("50000.0").unwrap());
    assert_eq!(bid_size, Size::from_str("1.0").unwrap());

    let (ask_price, ask_size) = market_state.best_ask().unwrap();
    assert_eq!(ask_price, Price::from_str("50100.0").unwrap());
    assert_eq!(ask_size, Size::from_str("1.0").unwrap());

    // Verify spread
    let spread = market_state.spread().unwrap();
    assert_eq!(spread, Price::from_str("100.0").unwrap());
}

/// Test NewOrder::new_limit_buy helper method
#[test]
fn test_new_order_limit_buy() {
    let order = NewOrder::new_limit_buy(
        "BTCUSDT".to_string(),
        Size::from_str("1.0").unwrap(),
        Price::from_str("50000.0").unwrap(),
        TimeInForce::GoodTillCancelled,
    );

    assert_eq!(order.symbol.as_str(), "BTCUSDT");
    assert_eq!(order.side, OrderSide::Buy);
    assert_eq!(order.order_type, OrderType::Limit);
    assert_eq!(order.size, Size::from_str("1.0").unwrap());
    assert_eq!(order.price, Some(Price::from_str("50000.0").unwrap()));
    assert_eq!(order.time_in_force, TimeInForce::GoodTillCancelled);
}

/// Test NewOrder::new_limit_sell helper method
#[test]
fn test_new_order_limit_sell() {
    let order = NewOrder::new_limit_sell(
        "BTCUSDT".to_string(),
        Size::from_str("1.0").unwrap(),
        Price::from_str("50000.0").unwrap(),
        TimeInForce::GoodTillCancelled,
    );

    assert_eq!(order.symbol.as_str(), "BTCUSDT");
    assert_eq!(order.side, OrderSide::Sell);
    assert_eq!(order.order_type, OrderType::Limit);
    assert_eq!(order.size, Size::from_str("1.0").unwrap());
    assert_eq!(order.price, Some(Price::from_str("50000.0").unwrap()));
}

/// Test NewOrder uses 'size' field (not 'quantity')
#[test]
fn test_new_order_size_field() {
    let order = NewOrder::new_limit_buy(
        "BTCUSDT".to_string(),
        Size::from_str("1.5").unwrap(),
        Price::from_str("50000.0").unwrap(),
        TimeInForce::GoodTillCancelled,
    );

    // Access size field directly
    assert_eq!(order.size, Size::from_str("1.5").unwrap());
}
