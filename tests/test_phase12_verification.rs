//! Phase 12 TDD Verification Tests
//!
//! These tests verify the compilation and basic functionality
//! of the crypto_hft library after Phase 11 cleanup.

use crypto_hft::{
    connectors::DryRunExecutionClient,
    core::events::{
        ExecutionReport, NewOrder, OrderBookDelta, OrderBookLevel, OrderBookSnapshot, OrderSide,
        OrderStatus, OrderType, TimeInForce, Trade,
    },
    orderbook::OrderBook,
    risk::{RiskEngine, ShadowLedger},
    strategies::MarketMakingStrategy,
    strategy::{MarketState, Signal, Strategy},
    traits::MarketEvent,
    types::{Price, Size, Symbol},
};
use std::time::Duration;

// ===== T080: Verify core types compile and work =====

#[test]
fn test_t080_price_type() {
    let price = Price::from_str("100.50").unwrap();
    assert_eq!(price.to_string(), "100.50");

    let price2 = Price::from_str("50.25").unwrap();
    let sum = price + price2;
    assert_eq!(sum.to_string(), "150.75");
}

#[test]
fn test_t080_size_type() {
    let size = Size::from_str("1.5").unwrap();
    assert_eq!(size.to_string(), "1.5");

    let size2 = Size::from_str("0.5").unwrap();
    let sum = size + size2;
    assert_eq!(sum.to_string(), "2.0");
}

#[test]
fn test_t080_symbol_type() {
    let symbol = Symbol::new("BTCUSDT");
    assert_eq!(symbol.value(), "BTCUSDT");
    assert_eq!(symbol.as_str(), "BTCUSDT");
    assert_eq!(symbol.len(), 7);
}

#[test]
fn test_t080_order_types() {
    // Test OrderSide
    let buy = OrderSide::Buy;
    let sell = OrderSide::Sell;
    assert_ne!(buy, sell);

    // Test OrderType
    let market = OrderType::Market;
    let limit = OrderType::Limit;
    let stop_loss = OrderType::StopLoss;
    let stop_limit = OrderType::StopLimit;
    assert_ne!(market, limit);
    assert_ne!(stop_loss, stop_limit);

    // Test TimeInForce
    let gtc = TimeInForce::GoodTillCancelled;
    let ioc = TimeInForce::ImmediateOrCancel;
    let fok = TimeInForce::FillOrKill;
    assert_ne!(gtc, ioc);
    assert_ne!(ioc, fok);

    // Test OrderStatus
    let new = OrderStatus::New;
    let filled = OrderStatus::Filled;
    let cancelled = OrderStatus::Cancelled;
    assert_ne!(new, filled);
    assert_ne!(filled, cancelled);
}

// ===== T081: Verify OrderBook functionality =====

#[test]
fn test_t081_orderbook_creation() {
    let ob = OrderBook::new("BTCUSDT".to_string());
    assert_eq!(ob.symbol(), "BTCUSDT");
    assert!(ob.best_bid().is_none());
    assert!(ob.best_ask().is_none());
}

#[test]
fn test_t081_orderbook_snapshot() {
    let mut ob = OrderBook::new("BTCUSDT".to_string());

    let snapshot = OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        "binance".to_string(),
        vec![
            OrderBookLevel::new(
                Price::from_str("100.00").unwrap(),
                Size::from_str("1.0").unwrap(),
            ),
            OrderBookLevel::new(
                Price::from_str("99.00").unwrap(),
                Size::from_str("2.0").unwrap(),
            ),
        ],
        vec![
            OrderBookLevel::new(
                Price::from_str("101.00").unwrap(),
                Size::from_str("1.5").unwrap(),
            ),
            OrderBookLevel::new(
                Price::from_str("102.00").unwrap(),
                Size::from_str("2.5").unwrap(),
            ),
        ],
        1234567890,
    );

    ob.apply_snapshot(snapshot);

    let best_bid = ob.best_bid();
    assert!(best_bid.is_some());
    let (bid_price, bid_size) = best_bid.unwrap();
    assert_eq!(bid_price, Price::from_str("100.00").unwrap());
    assert_eq!(bid_size, Size::from_str("1.0").unwrap());

    let best_ask = ob.best_ask();
    assert!(best_ask.is_some());
    let (ask_price, ask_size) = best_ask.unwrap();
    assert_eq!(ask_price, Price::from_str("101.00").unwrap());
    assert_eq!(ask_size, Size::from_str("1.5").unwrap());
}

// ===== T082: Verify Strategy functionality =====

#[test]
fn test_t082_market_state() {
    let market_state = MarketState::new("BTCUSDT".to_string());
    assert_eq!(market_state.symbol, "BTCUSDT");
}

#[test]
fn test_t082_market_making_strategy_creation() {
    let _strategy = MarketMakingStrategy::new(
        Price::from_str("0.5").unwrap(),
        Size::from_str("0.1").unwrap(),
        Size::from_str("1.0").unwrap(),
        5,
        Duration::from_millis(100),
    );

    // Strategy should be created successfully
    assert!(true);
}

#[test]
fn test_t082_signal_enum() {
    // Test PlaceOrder signal
    let order = NewOrder::new_limit_buy(
        "BTCUSDT",
        Size::from_str("1.0").unwrap(),
        Price::from_str("100.00").unwrap(),
        TimeInForce::GoodTillCancelled,
    );
    let signal = Signal::PlaceOrder { order };
    match signal {
        Signal::PlaceOrder { order: _ } => assert!(true),
        _ => assert!(false, "Expected PlaceOrder signal"),
    }

    // Test CancelOrder signal
    let cancel_signal = Signal::CancelOrder {
        order_id: "12345".to_string(),
        symbol: "BTCUSDT".to_string(),
        exchange_id: "binance".to_string(),
    };
    match cancel_signal {
        Signal::CancelOrder {
            order_id,
            symbol,
            exchange_id,
        } => {
            assert_eq!(order_id, "12345");
            assert_eq!(symbol, "BTCUSDT");
            assert_eq!(exchange_id, "binance");
        }
        _ => assert!(false, "Expected CancelOrder signal"),
    }
}

// ===== T083: Verify Risk Management =====

#[test]
fn test_t083_risk_engine_creation() {
    let risk_engine = RiskEngine::new();
    // Risk engine should be created successfully
    assert!(true);
}

#[test]
fn test_t083_shadow_ledger_creation() {
    let shadow_ledger = ShadowLedger::new();
    // Shadow ledger should be created successfully
    assert!(true);
}

// ===== T084: Verify NewOrder helpers =====

#[test]
fn test_t084_new_order_helpers() {
    // Test new_limit_buy
    let buy_order = NewOrder::new_limit_buy(
        "BTCUSDT",
        Size::from_str("1.0").unwrap(),
        Price::from_str("100.00").unwrap(),
        TimeInForce::GoodTillCancelled,
    );
    assert_eq!(buy_order.side, OrderSide::Buy);
    assert_eq!(buy_order.order_type, OrderType::Limit);
    assert_eq!(buy_order.symbol.value(), "BTCUSDT");

    // Test new_limit_sell
    let sell_order = NewOrder::new_limit_sell(
        "BTCUSDT",
        Size::from_str("1.0").unwrap(),
        Price::from_str("101.00").unwrap(),
        TimeInForce::GoodTillCancelled,
    );
    assert_eq!(sell_order.side, OrderSide::Sell);
    assert_eq!(sell_order.order_type, OrderType::Limit);

    // Test new_market_buy
    let market_buy = NewOrder::new_market_buy("BTCUSDT", Size::from_str("1.0").unwrap());
    assert_eq!(market_buy.side, OrderSide::Buy);
    assert_eq!(market_buy.order_type, OrderType::Market);

    // Test new_market_sell
    let market_sell = NewOrder::new_market_sell("BTCUSDT", Size::from_str("1.0").unwrap());
    assert_eq!(market_sell.side, OrderSide::Sell);
    assert_eq!(market_sell.order_type, OrderType::Market);
}

// ===== T085: Verify DryRun Connector =====

#[test]
fn test_t085_dry_run_execution_client() {
    let client = DryRunExecutionClient::new();
    // Client should be created successfully
    assert!(true);
}

// ===== T086: Verify MarketEvent types =====

#[test]
fn test_t086_market_event_snapshot() {
    let snapshot = OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        "binance".to_string(),
        vec![OrderBookLevel::new(
            Price::from_str("100.00").unwrap(),
            Size::from_str("1.0").unwrap(),
        )],
        vec![OrderBookLevel::new(
            Price::from_str("101.00").unwrap(),
            Size::from_str("1.0").unwrap(),
        )],
        1234567890,
    );

    let event = MarketEvent::OrderBookSnapshot(snapshot);
    match event {
        MarketEvent::OrderBookSnapshot(_) => assert!(true),
        _ => assert!(false, "Expected OrderBookSnapshot"),
    }
}

#[test]
fn test_t086_market_event_delta() {
    let delta = OrderBookDelta::new(
        "BTCUSDT".to_string(),
        "binance".to_string(),
        vec![OrderBookLevel::new(
            Price::from_str("100.00").unwrap(),
            Size::from_str("1.0").unwrap(),
        )],
        vec![OrderBookLevel::new(
            Price::from_str("101.00").unwrap(),
            Size::from_str("1.0").unwrap(),
        )],
        1234567891,
    );

    let event = MarketEvent::OrderBookDelta(delta);
    match event {
        MarketEvent::OrderBookDelta(_) => assert!(true),
        _ => assert!(false, "Expected OrderBookDelta"),
    }
}

#[test]
fn test_t086_market_event_trade() {
    let trade = Trade {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        price: Price::from_str("100.50").unwrap(),
        size: Size::from_str("0.5").unwrap(),
        side: OrderSide::Buy,
        timestamp: 1234567892,
        trade_id: Some("trade_001".to_string()),
    };

    let event = MarketEvent::Trade(trade);
    match event {
        MarketEvent::Trade(_) => assert!(true),
        _ => assert!(false, "Expected Trade"),
    }
}

// ===== T087: Verify ExecutionReport =====

#[test]
fn test_t087_execution_report() {
    let report = ExecutionReport {
        order_id: "12345".to_string(),
        client_order_id: None,
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        status: OrderStatus::Filled,
        filled_size: Size::from_str("1.0").unwrap(),
        remaining_size: Size::zero(),
        average_price: Some(Price::from_str("100.00").unwrap()),
        timestamp: 1234567890,
    };

    assert_eq!(report.order_id, "12345");
    assert_eq!(report.symbol.value(), "BTCUSDT");
    assert_eq!(report.exchange_id, "binance");
    assert_eq!(report.status, OrderStatus::Filled);
}

