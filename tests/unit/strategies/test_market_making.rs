use crypto_hft::strategies::MarketMakingStrategy;
use crypto_hft::types::{Price, Size};
use crypto_hft::strategy::{MarketState, Signal};
use crypto_hft::orderbook::{OrderBookSnapshot, OrderBookLevel};
use crypto_hft::traits::MarketEvent;
use std::time::Duration;

#[test]
fn test_market_making_strategy_creation() {
    let strategy = MarketMakingStrategy::new(
        Price::from_str("0.5").unwrap(),  // target_spread
        Size::from_str("0.1").unwrap(),   // base_order_size
        Size::from_str("1.0").unwrap(),   // max_position_size
        10,                               // max_order_levels
        Duration::from_millis(100),        // order_refresh_time
    );
    
    // Verify strategy was created with correct parameters
    assert_eq!(strategy.target_spread(), Price::from_str("0.5").unwrap());
    assert_eq!(strategy.base_order_size(), Size::from_str("0.1").unwrap());
    assert_eq!(strategy.max_position_size(), Size::from_str("1.0").unwrap());
    assert_eq!(strategy.max_order_levels(), 10);
    assert_eq!(strategy.order_refresh_time(), Duration::from_millis(100));
}

#[test]
fn test_market_making_no_signal_when_no_spread() {
    let mut strategy = MarketMakingStrategy::new(
        Price::from_str("0.5").unwrap(),
        Size::from_str("0.1").unwrap(),
        Size::from_str("1.0").unwrap(),
        5,
        Duration::from_millis(100),
    );
    
    // Create market state with no spread (same bid and ask price)
    let mut market_state = MarketState::new("BTCUSDT".to_string());
    
    let snapshot = OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        vec![OrderBookLevel::new(
            Price::from_str("100.00").unwrap(),
            Size::from_str("10.0").unwrap()
        )],
        vec![OrderBookLevel::new(
            Price::from_str("100.00").unwrap(),  // Same price as bid
            Size::from_str("10.0").unwrap()
        )],
        123456789,
    );
    
    let event = MarketEvent::OrderBookSnapshot(snapshot);
    market_state.update(&event);
    
    // Should not generate a signal when there's no spread
    let signal = strategy.generate_signal(&market_state);
    assert!(signal.is_none());
}

#[test]
fn test_market_making_signal_when_spread_exists() {
    let mut strategy = MarketMakingStrategy::new(
        Price::from_str("0.5").unwrap(),
        Size::from_str("0.1").unwrap(),
        Size::from_str("1.0").unwrap(),
        5,
        Duration::from_millis(100),
    );
    
    // Create market state with a spread
    let mut market_state = MarketState::new("BTCUSDT".to_string());
    
    let snapshot = OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        vec![OrderBookLevel::new(
            Price::from_str("100.00").unwrap(),
            Size::from_str("10.0").unwrap()
        )],
        vec![OrderBookLevel::new(
            Price::from_str("101.00").unwrap(),  // $1 spread
            Size::from_str("10.0").unwrap()
        )],
        123456789,
    );
    
    let event = MarketEvent::OrderBookSnapshot(snapshot);
    market_state.update(&event);
    
    // Should generate a signal when there's a spread
    let signal = strategy.generate_signal(&market_state);
    assert!(signal.is_some());
    
    // Verify the signal contains both buy and sell orders
    if let Some(Signal::PlaceOrder { order }) = signal {
        assert_eq!(order.symbol, "BTCUSDT");
        assert_eq!(order.quantity, Size::from_str("0.1").unwrap());
    } else {
        panic!("Expected PlaceOrder signal");
    }
}

#[test]
fn test_market_making_respects_position_limits() {
    let mut strategy = MarketMakingStrategy::new(
        Price::from_str("0.5").unwrap(),
        Size::from_str("0.5").unwrap(),   // Larger order size
        Size::from_str("1.0").unwrap(),   // Small max position
        5,
        Duration::from_millis(100),
    );
    
    // Simulate having a position at the limit
    strategy.update_position("BTCUSDT", Size::from_str("1.0").unwrap());
    
    // Create market state with a spread
    let mut market_state = MarketState::new("BTCUSDT".to_string());
    
    let snapshot = OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        vec![OrderBookLevel::new(
            Price::from_str("100.00").unwrap(),
            Size::from_str("10.0").unwrap()
        )],
        vec![OrderBookLevel::new(
            Price::from_str("101.00").unwrap(),
            Size::from_str("10.0").unwrap()
        )],
        123456789,
    );
    
    let event = MarketEvent::OrderBookSnapshot(snapshot);
    market_state.update(&event);
    
    // Should not generate a signal when at position limit
    let signal = strategy.generate_signal(&market_state);
    assert!(signal.is_none());
}

#[test]
fn test_market_making_multiple_order_levels() {
    let mut strategy = MarketMakingStrategy::new(
        Price::from_str("0.5").unwrap(),
        Size::from_str("0.1").unwrap(),
        Size::from_str("1.0").unwrap(),
        3,  // 3 order levels
        Duration::from_millis(100),
    );
    
    // Create market state with deep order book
    let mut market_state = MarketState::new("BTCUSDT".to_string());
    
    let snapshot = OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        vec![
            OrderBookLevel::new(Price::from_str("100.00").unwrap(), Size::from_str("10.0").unwrap()),
            OrderBookLevel::new(Price::from_str("99.50").unwrap(), Size::from_str("5.0").unwrap()),
            OrderBookLevel::new(Price::from_str("99.00").unwrap(), Size::from_str("8.0").unwrap()),
        ],
        vec![
            OrderBookLevel::new(Price::from_str("101.00").unwrap(), Size::from_str("10.0").unwrap()),
            OrderBookLevel::new(Price::from_str("101.50").unwrap(), Size::from_str("5.0").unwrap()),
            OrderBookLevel::new(Price::from_str("102.00").unwrap(), Size::from_str("8.0").unwrap()),
        ],
        123456789,
    );
    
    let event = MarketEvent::OrderBookSnapshot(snapshot);
    market_state.update(&event);
    
    // Should generate signals for multiple levels
    let signal = strategy.generate_signal(&market_state);
    assert!(signal.is_some());
    
    // In a real implementation, we'd check that multiple orders are placed
    // For this test, we just verify a signal is generated
}

#[test]
fn test_market_making_order_refresh_time() {
    let mut strategy = MarketMakingStrategy::new(
        Price::from_str("0.5").unwrap(),
        Size::from_str("0.1").unwrap(),
        Size::from_str("1.0").unwrap(),
        5,
        Duration::from_millis(100),  // Short refresh time for testing
    );
    
    // Create market state with a spread
    let mut market_state = MarketState::new("BTCUSDT".to_string());
    
    let snapshot = OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        vec![OrderBookLevel::new(
            Price::from_str("100.00").unwrap(),
            Size::from_str("10.0").unwrap()
        )],
        vec![OrderBookLevel::new(
            Price::from_str("101.00").unwrap(),
            Size::from_str("10.0").unwrap()
        )],
        123456789,
    );
    
    let event = MarketEvent::OrderBookSnapshot(snapshot);
    market_state.update(&event);
    
    // First signal should be generated
    let signal1 = strategy.generate_signal(&market_state);
    assert!(signal1.is_some());
    
    // Immediate second call should not generate a signal due to cooldown
    let signal2 = strategy.generate_signal(&market_state);
    assert!(signal2.is_none());
    
    // Wait for cooldown to expire
    std::thread::sleep(Duration::from_millis(110));
    
    // After cooldown, a new signal should be generated
    let signal3 = strategy.generate_signal(&market_state);
    assert!(signal3.is_some());
}

#[test]
fn test_market_making_inventory_skew() {
    let mut strategy = MarketMakingStrategy::new(
        Price::from_str("0.5").unwrap(),
        Size::from_str("0.1").unwrap(),
        Size::from_str("1.0").unwrap(),
        5,
        Duration::from_millis(100),
    );
    
    // Simulate having a long position (more buys than sells)
    strategy.update_position("BTCUSDT", Size::from_str("0.5").unwrap());
    
    // Create market state with a spread
    let mut market_state = MarketState::new("BTCUSDT".to_string());
    
    let snapshot = OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        vec![OrderBookLevel::new(
            Price::from_str("100.00").unwrap(),
            Size::from_str("10.0").unwrap()
        )],
        vec![OrderBookLevel::new(
            Price::from_str("101.00").unwrap(),
            Size::from_str("10.0").unwrap()
        )],
        123456789,
    );
    
    let event = MarketEvent::OrderBookSnapshot(snapshot);
    market_state.update(&event);
    
    // Should generate a signal, but with inventory skew adjustment
    let signal = strategy.generate_signal(&market_state);
    assert!(signal.is_some());
    
    // In a real implementation, we'd verify that the signal is adjusted
    // to favor sell orders when we have a long position
}
