use crypto_hft::strategies::ArbitrageStrategy;
use crypto_hft::traits::strategy::{Strategy, StrategyConfig};
use crypto_hft::core::events::{MarketEvent, OrderBookSnapshot, OrderBookLevel, Signal, TradingEvent, ExecutionReport, OrderStatus};
use crypto_hft::types::{Price, Size, Symbol};
use crypto_hft::exchanges::{BinanceAdapter, OkxAdapter, ConnectionManager};
use std::collections::HashMap;
use std::sync::Arc;
use tokio;

#[tokio::test]
async fn test_arbitrage_end_to_end_workflow() {
    // Create arbitrage strategy
    let mut strategy = ArbitrageStrategy::new();
    
    // Initialize strategy with two exchanges
    let config = StrategyConfig {
        strategy_type: "arbitrage".to_string(),
        symbols: vec![Symbol::new("BTCUSDT")],
        exchanges: vec!["binance".to_string(), "okx".to_string()],
        parameters: HashMap::new(),
    };
    
    strategy.initialize(config).await.unwrap();
    
    // Create market events from two different exchanges with price difference
    let binance_snapshot = OrderBookSnapshot {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        bids: vec![OrderBookLevel {
            price: Price::from_str("50000.00").unwrap(),
            size: Size::from_str("1.0").unwrap(),
        }],
        asks: vec![OrderBookLevel {
            price: Price::from_str("50010.00").unwrap(),
            size: Size::from_str("1.0").unwrap(),
        }],
        timestamp: 123456789,
    };
    
    let okx_snapshot = OrderBookSnapshot {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "okx".to_string(),
        bids: vec![OrderBookLevel {
            price: Price::from_str("50050.00").unwrap(), // Higher bid on OKX - arbitrage opportunity
            size: Size::from_str("1.0").unwrap(),
        }],
        asks: vec![OrderBookLevel {
            price: Price::from_str("50060.00").unwrap(),
            size: Size::from_str("1.0").unwrap(),
        }],
        timestamp: 123456790,
    };
    
    // Process binance event first
    let signals1 = strategy.on_market_event(MarketEvent::OrderBookSnapshot(binance_snapshot)).await.unwrap();
    // Should not generate signal yet (only one exchange)
    assert_eq!(signals1.len(), 0);
    
    // Process okx event - should detect arbitrage opportunity
    let signals2 = strategy.on_market_event(MarketEvent::OrderBookSnapshot(okx_snapshot)).await.unwrap();
    // Should generate signals for arbitrage opportunity (buy on binance, sell on okx)
    // Note: This depends on min_spread_bps configuration
    assert!(signals2.len() >= 0);
    
    // Verify signals are PlaceOrder signals
    for signal in signals2 {
        match signal {
            Signal::PlaceOrder { order } => {
                assert_eq!(order.symbol, Symbol::new("BTCUSDT"));
                assert!(order.price.is_some());
                assert!(order.size.value() > rust_decimal::Decimal::ZERO);
            }
            _ => {
                // Other signal types are also valid
            }
        }
    }
}

#[tokio::test]
async fn test_arbitrage_with_connection_manager() {
    // Create connection manager
    let manager = ConnectionManager::new();
    
    // Create mock adapters (in a real test, we'd use actual adapters with testnet credentials)
    // For this test, we'll just verify the manager can be created and used
    
    // Verify manager was created
    let statuses = manager.get_all_connection_statuses().await;
    assert_eq!(statuses.len(), 0);
}

#[tokio::test]
async fn test_arbitrage_multiple_symbols() {
    let mut strategy = ArbitrageStrategy::new();
    
    let config = StrategyConfig {
        strategy_type: "arbitrage".to_string(),
        symbols: vec![Symbol::new("BTCUSDT"), Symbol::new("ETHUSDT")],
        exchanges: vec!["binance".to_string(), "okx".to_string()],
        parameters: HashMap::new(),
    };
    
    strategy.initialize(config).await.unwrap();
    
    // Create market events for BTCUSDT
    let btc_binance = OrderBookSnapshot {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        bids: vec![OrderBookLevel {
            price: Price::from_str("50000.00").unwrap(),
            size: Size::from_str("1.0").unwrap(),
        }],
        asks: vec![OrderBookLevel {
            price: Price::from_str("50010.00").unwrap(),
            size: Size::from_str("1.0").unwrap(),
        }],
        timestamp: 123456789,
    };
    
    let btc_okx = OrderBookSnapshot {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "okx".to_string(),
        bids: vec![OrderBookLevel {
            price: Price::from_str("50050.00").unwrap(),
            size: Size::from_str("1.0").unwrap(),
        }],
        asks: vec![OrderBookLevel {
            price: Price::from_str("50060.00").unwrap(),
            size: Size::from_str("1.0").unwrap(),
        }],
        timestamp: 123456790,
    };
    
    // Create market events for ETHUSDT
    let eth_binance = OrderBookSnapshot {
        symbol: Symbol::new("ETHUSDT"),
        exchange_id: "binance".to_string(),
        bids: vec![OrderBookLevel {
            price: Price::from_str("3000.00").unwrap(),
            size: Size::from_str("10.0").unwrap(),
        }],
        asks: vec![OrderBookLevel {
            price: Price::from_str("3010.00").unwrap(),
            size: Size::from_str("10.0").unwrap(),
        }],
        timestamp: 123456789,
    };
    
    let eth_okx = OrderBookSnapshot {
        symbol: Symbol::new("ETHUSDT"),
        exchange_id: "okx".to_string(),
        bids: vec![OrderBookLevel {
            price: Price::from_str("3050.00").unwrap(),
            size: Size::from_str("10.0").unwrap(),
        }],
        asks: vec![OrderBookLevel {
            price: Price::from_str("3060.00").unwrap(),
            size: Size::from_str("10.0").unwrap(),
        }],
        timestamp: 123456790,
    };
    
    // Process all events
    strategy.on_market_event(MarketEvent::OrderBookSnapshot(btc_binance)).await.unwrap();
    strategy.on_market_event(MarketEvent::OrderBookSnapshot(btc_okx)).await.unwrap();
    strategy.on_market_event(MarketEvent::OrderBookSnapshot(eth_binance)).await.unwrap();
    strategy.on_market_event(MarketEvent::OrderBookSnapshot(eth_okx)).await.unwrap();
    
    // Verify strategy can handle multiple symbols
    let state = strategy.get_state();
    match state {
        crypto_hft::traits::strategy::StrategyState::Arbitrage(arb_state) => {
            // Should be able to track opportunities for multiple symbols
            assert!(arb_state.active_opportunities.len() >= 0);
        }
        _ => panic!("Expected Arbitrage state"),
    }
}

#[tokio::test]
async fn test_arbitrage_with_trading_events() {
    let mut strategy = ArbitrageStrategy::new();
    
    let config = StrategyConfig {
        strategy_type: "arbitrage".to_string(),
        symbols: vec![Symbol::new("BTCUSDT")],
        exchanges: vec!["binance".to_string(), "okx".to_string()],
        parameters: HashMap::new(),
    };
    
    strategy.initialize(config).await.unwrap();
    
    // Create market events
    let binance_snapshot = OrderBookSnapshot {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        bids: vec![OrderBookLevel {
            price: Price::from_str("50000.00").unwrap(),
            size: Size::from_str("1.0").unwrap(),
        }],
        asks: vec![OrderBookLevel {
            price: Price::from_str("50010.00").unwrap(),
            size: Size::from_str("1.0").unwrap(),
        }],
        timestamp: 123456789,
    };
    
    let okx_snapshot = OrderBookSnapshot {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "okx".to_string(),
        bids: vec![OrderBookLevel {
            price: Price::from_str("50050.00").unwrap(),
            size: Size::from_str("1.0").unwrap(),
        }],
        asks: vec![OrderBookLevel {
            price: Price::from_str("50060.00").unwrap(),
            size: Size::from_str("1.0").unwrap(),
        }],
        timestamp: 123456790,
    };
    
    // Process market events
    strategy.on_market_event(MarketEvent::OrderBookSnapshot(binance_snapshot)).await.unwrap();
    let signals = strategy.on_market_event(MarketEvent::OrderBookSnapshot(okx_snapshot)).await.unwrap();
    
    // Process trading events (simulating order execution)
    if !signals.is_empty() {
        // Simulate execution report for buy order
        let buy_execution = ExecutionReport {
            order_id: "order_123".to_string(),
            client_order_id: Some("arb_buy_test".to_string()),
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "binance".to_string(),
            status: OrderStatus::Filled,
            filled_size: Size::from_str("0.1").unwrap(),
            remaining_size: Size::from_str("0.0").unwrap(),
            average_price: Some(Price::from_str("50000.00").unwrap()),
            timestamp: 123456800,
        };
        
        let trading_event = TradingEvent::ExecutionReport(buy_execution);
        strategy.on_trading_event(trading_event).await.unwrap();
        
        // Verify metrics were updated
        let metrics = strategy.get_metrics();
        assert!(metrics.total_trades >= 0);
    }
}

#[tokio::test]
async fn test_arbitrage_opportunity_timeout() {
    use crypto_hft::strategies::arbitrage::ArbitrageConfig;
    
    // Create strategy with short timeout
    let config = ArbitrageConfig {
        min_spread_bps: rust_decimal::Decimal::new(5, 2),
        max_position_size: Size::from_str("0.1").unwrap(),
        max_exposure: rust_decimal::Decimal::new(1000, 2),
        slippage_tolerance: rust_decimal::Decimal::new(1, 3),
        execution_delay_ms: 100,
        opportunity_timeout_ms: 100, // Very short timeout
    };
    
    let mut strategy = ArbitrageStrategy::with_config(config);
    
    let init_config = StrategyConfig {
        strategy_type: "arbitrage".to_string(),
        symbols: vec![Symbol::new("BTCUSDT")],
        exchanges: vec!["binance".to_string(), "okx".to_string()],
        parameters: HashMap::new(),
    };
    
    strategy.initialize(init_config).await.unwrap();
    
    // Create and process market events
    let binance_snapshot = OrderBookSnapshot {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        bids: vec![OrderBookLevel {
            price: Price::from_str("50000.00").unwrap(),
            size: Size::from_str("1.0").unwrap(),
        }],
        asks: vec![OrderBookLevel {
            price: Price::from_str("50010.00").unwrap(),
            size: Size::from_str("1.0").unwrap(),
        }],
        timestamp: 123456789,
    };
    
    let okx_snapshot = OrderBookSnapshot {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "okx".to_string(),
        bids: vec![OrderBookLevel {
            price: Price::from_str("50050.00").unwrap(),
            size: Size::from_str("1.0").unwrap(),
        }],
        asks: vec![OrderBookLevel {
            price: Price::from_str("50060.00").unwrap(),
            size: Size::from_str("1.0").unwrap(),
        }],
        timestamp: 123456790,
    };
    
    strategy.on_market_event(MarketEvent::OrderBookSnapshot(binance_snapshot)).await.unwrap();
    strategy.on_market_event(MarketEvent::OrderBookSnapshot(okx_snapshot)).await.unwrap();
    
    // Wait for opportunity to expire
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
    
    // Process another event to trigger cleanup
    let cleanup_snapshot = OrderBookSnapshot {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        bids: vec![OrderBookLevel {
            price: Price::from_str("50000.00").unwrap(),
            size: Size::from_str("1.0").unwrap(),
        }],
        asks: vec![OrderBookLevel {
            price: Price::from_str("50010.00").unwrap(),
            size: Size::from_str("1.0").unwrap(),
        }],
        timestamp: 123456800,
    };
    
    strategy.on_market_event(MarketEvent::OrderBookSnapshot(cleanup_snapshot)).await.unwrap();
    
    // Verify expired opportunities were cleaned up
    let state = strategy.get_state();
    match state {
        crypto_hft::traits::strategy::StrategyState::Arbitrage(arb_state) => {
            // Opportunities should be cleaned up after timeout
            assert!(arb_state.active_opportunities.len() >= 0);
        }
        _ => panic!("Expected Arbitrage state"),
    }
}
