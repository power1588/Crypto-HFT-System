use crypto_hft::orderbook::{OrderBook, OrderBookSnapshot, OrderBookDelta, OrderBookLevel};
use crypto_hft::types::{Price, Size};

#[test]
fn test_orderbook_empty() {
    let mut book = OrderBook::new("BTCUSDT".to_string());
    assert!(book.best_bid().is_none());
    assert!(book.best_ask().is_none());
}

#[test]
fn test_orderbook_single_bid() {
    let mut book = OrderBook::new("BTCUSDT".to_string());
    let snapshot = OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        vec![OrderBookLevel::new(
            Price::from_str("100.0").unwrap(),
            Size::from_str("1.0").unwrap(),
        )],
        vec![],
        123456789,
    );
    book.apply_snapshot(snapshot);
    assert!(book.best_bid().is_some());
    assert!(book.best_ask().is_none());
}

#[test]
fn test_orderbook_single_ask() {
    let mut book = OrderBook::new("BTCUSDT".to_string());
    let snapshot = OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        vec![],
        vec![OrderBookLevel::new(
            Price::from_str("100.0").unwrap(),
            Size::from_str("1.0").unwrap(),
        )],
        123456789,
    );
    book.apply_snapshot(snapshot);
    assert!(book.best_bid().is_none());
    assert!(book.best_ask().is_some());
}

#[test]
fn test_orderbook_zero_size_level() {
    let mut book = OrderBook::new("BTCUSDT".to_string());
    let delta = OrderBookDelta::new(
        "BTCUSDT".to_string(),
        vec![OrderBookLevel::new(
            Price::from_str("100.0").unwrap(),
            Size::from_str("0.0").unwrap(),
        )],
        vec![],
        123456789,
    );
    book.apply_delta(delta);
    // Zero size should remove the level
    assert!(book.best_bid().is_none());
}

#[test]
fn test_orderbook_duplicate_price_levels() {
    let mut book = OrderBook::new("BTCUSDT".to_string());
    let snapshot = OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        vec![
            OrderBookLevel::new(Price::from_str("100.0").unwrap(), Size::from_str("1.0").unwrap()),
            OrderBookLevel::new(Price::from_str("100.0").unwrap(), Size::from_str("2.0").unwrap()),
        ],
        vec![],
        123456789,
    );
    book.apply_snapshot(snapshot);
    // Should keep the last value for duplicate prices
    let best_bid = book.best_bid().unwrap();
    assert_eq!(best_bid, Price::from_str("100.0").unwrap());
}

#[test]
fn test_orderbook_top_levels_empty() {
    let book = OrderBook::new("BTCUSDT".to_string());
    let levels = book.top_levels(10);
    assert!(levels.is_empty());
}

#[test]
fn test_orderbook_top_levels_more_than_available() {
    let mut book = OrderBook::new("BTCUSDT".to_string());
    let snapshot = OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        vec![OrderBookLevel::new(
            Price::from_str("100.0").unwrap(),
            Size::from_str("1.0").unwrap(),
        )],
        vec![],
        123456789,
    );
    book.apply_snapshot(snapshot);
    let levels = book.top_levels(10);
    assert_eq!(levels.len(), 1);
}

#[test]
fn test_orderbook_delta_remove_level() {
    let mut book = OrderBook::new("BTCUSDT".to_string());
    
    // Add a level
    let snapshot = OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        vec![OrderBookLevel::new(
            Price::from_str("100.0").unwrap(),
            Size::from_str("1.0").unwrap(),
        )],
        vec![],
        123456789,
    );
    book.apply_snapshot(snapshot);
    assert!(book.best_bid().is_some());
    
    // Remove it with zero size delta
    let delta = OrderBookDelta::new(
        "BTCUSDT".to_string(),
        vec![OrderBookLevel::new(
            Price::from_str("100.0").unwrap(),
            Size::from_str("0.0").unwrap(),
        )],
        vec![],
        123456790,
    );
    book.apply_delta(delta);
    assert!(book.best_bid().is_none());
}

