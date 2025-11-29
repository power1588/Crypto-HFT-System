use crypto_hft::strategies::{MarketMakingStrategy, LinearRegressionPredictor};
use crypto_hft::indicators::trade_flow_indicators::{TradeFlowIndicator, TradeFlowMomentum};
use crypto_hft::types::{Price, Size};
use crypto_hft::core::events::{Trade, OrderSide};
use crypto_hft::types::Symbol;
use crypto_hft::strategy::{MarketState, Strategy};
use crypto_hft::orderbook::{OrderBookSnapshot, OrderBookLevel};
use crypto_hft::traits::MarketEvent;
use std::time::Duration;

#[test]
fn test_prediction_integration_with_market_making() {
    // Create market making strategy with prediction enabled
    let mut strategy = MarketMakingStrategy::with_prediction(
        Price::from_str("0.5").unwrap(),
        Size::from_str("0.1").unwrap(),
        Size::from_str("1.0").unwrap(),
        5,
        Duration::from_millis(100),
        60,  // prediction_horizon_seconds
        0.3, // prediction_weight
        100, // predictor_max_history
        10,  // predictor_min_data_points
        100, // trade_flow_max_trades
        60000, // trade_flow_time_window_ms
    );
    
    // Add some trades to build up prediction model
    for i in 0..15 {
        let trade = Trade {
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "binance".to_string(),
            price: Price::new(crate::rust_decimal::Decimal::from_f64(100.0 + i as f64 * 0.1).unwrap()),
            size: Size::from_str("1.0").unwrap(),
            side: if i % 2 == 0 { OrderSide::Buy } else { OrderSide::Sell },
            timestamp: 1000 + i * 1000,
            trade_id: Some(format!("trade_{}", i)),
        };
        
        strategy.update_prediction(&trade);
    }
    
    // Create market state
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
        20000,
    );
    
    let event = MarketEvent::OrderBookSnapshot(snapshot);
    market_state.update(&event);
    
    // Generate signal - should use prediction to adjust prices
    let signal = strategy.generate_signal(&market_state);
    assert!(signal.is_some());
}

#[test]
fn test_trade_flow_indicator_integration() {
    let mut indicator = TradeFlowIndicator::new(100, 60000);
    
    // Add trades with buy pressure
    for i in 0..10 {
        let trade = Trade {
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "binance".to_string(),
            price: Price::from_str("100.00").unwrap(),
            size: Size::from_str("1.0").unwrap(),
            side: OrderSide::Buy,
            timestamp: 1000 + i * 1000,
            trade_id: Some(format!("trade_{}", i)),
        };
        
        indicator.add_trade(trade);
    }
    
    // Add some sell trades
    for i in 0..5 {
        let trade = Trade {
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "binance".to_string(),
            price: Price::from_str("100.00").unwrap(),
            size: Size::from_str("0.5").unwrap(),
            side: OrderSide::Sell,
            timestamp: 11000 + i * 1000,
            trade_id: Some(format!("trade_sell_{}", i)),
        };
        
        indicator.add_trade(trade);
    }
    
    // Check buy pressure
    assert!(indicator.buy_pressure().value() > indicator.sell_pressure().value());
    
    // Check flow ratio (should be positive due to more buys)
    let flow_ratio = indicator.flow_ratio();
    assert!(flow_ratio.is_some());
    assert!(flow_ratio.unwrap() > 0.0);
}

#[test]
fn test_trade_flow_momentum_integration() {
    let mut momentum = TradeFlowMomentum::new(100, 60000, 10);
    
    // Add trades with increasing buy pressure
    for i in 0..10 {
        let buy_size = 1.0 + (i as f64 * 0.1);
        let trade = Trade {
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "binance".to_string(),
            price: Price::from_str("100.00").unwrap(),
            size: Size::new(crate::rust_decimal::Decimal::from_f64(buy_size).unwrap()),
            side: OrderSide::Buy,
            timestamp: 1000 + i * 1000,
            trade_id: Some(format!("trade_{}", i)),
        };
        
        momentum.add_trade(trade);
    }
    
    // Add some sell trades
    for i in 0..3 {
        let trade = Trade {
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "binance".to_string(),
            price: Price::from_str("100.00").unwrap(),
            size: Size::from_str("0.5").unwrap(),
            side: OrderSide::Sell,
            timestamp: 11000 + i * 1000,
            trade_id: Some(format!("trade_sell_{}", i)),
        };
        
        momentum.add_trade(trade);
    }
    
    // Check momentum
    let momentum_value = momentum.momentum();
    // Momentum might be positive or negative depending on the last trades
    assert!(momentum_value.is_some());
    
    // Check average momentum
    let avg_momentum = momentum.average_momentum();
    assert!(avg_momentum.is_some());
}

#[test]
fn test_prediction_workflow_with_historical_data() {
    let mut predictor = LinearRegressionPredictor::new(100, 10);
    
    // Simulate historical price data with an upward trend
    for i in 0..20 {
        let timestamp = 1000 + i * 1000;
        let price_value = 100.0 + (i as f64 * 0.1);
        let price = Price::new(crate::rust_decimal::Decimal::from_f64(price_value).unwrap());
        predictor.update(timestamp, price);
    }
    
    // Verify model is ready
    assert!(predictor.is_ready());
    
    // Get R-squared to verify model quality
    let r_squared = predictor.r_squared();
    assert!(r_squared.is_some());
    assert!(r_squared.unwrap() > 0.8);
    
    // Make a prediction
    let predicted_price = predictor.predict_after_seconds(10);
    assert!(predicted_price.is_some());
    
    // Predicted price should be higher than the last observed price (upward trend)
    let last_price = Price::new(crate::rust_decimal::Decimal::from_f64(101.9).unwrap());
    let predicted_value = predicted_price.unwrap().value().to_f64().unwrap();
    let last_value = last_price.value().to_f64().unwrap();
    assert!(predicted_value >= last_value);
}

#[test]
fn test_prediction_with_market_making_price_adjustment() {
    // Create strategy with prediction
    let mut strategy = MarketMakingStrategy::with_prediction(
        Price::from_str("0.5").unwrap(),
        Size::from_str("0.1").unwrap(),
        Size::from_str("1.0").unwrap(),
        5,
        Duration::from_millis(100),
        60,
        0.5, // Higher prediction weight
        100,
        10,
        100,
        60000,
    );
    
    // Build prediction model with upward trend
    for i in 0..15 {
        let trade = Trade {
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "binance".to_string(),
            price: Price::new(crate::rust_decimal::Decimal::from_f64(100.0 + i as f64 * 0.1).unwrap()),
            size: Size::from_str("1.0").unwrap(),
            side: OrderSide::Buy,
            timestamp: 1000 + i * 1000,
            trade_id: Some(format!("trade_{}", i)),
        };
        
        strategy.update_prediction(&trade);
    }
    
    // Create market state
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
        16000,
    );
    
    let event = MarketEvent::OrderBookSnapshot(snapshot);
    market_state.update(&event);
    
    // Generate signal - prices should be adjusted upward based on prediction
    let signal = strategy.generate_signal(&market_state);
    assert!(signal.is_some());
}

#[test]
fn test_prediction_disabled_behavior() {
    // Create strategy without prediction
    let mut strategy = MarketMakingStrategy::new(
        Price::from_str("0.5").unwrap(),
        Size::from_str("0.1").unwrap(),
        Size::from_str("1.0").unwrap(),
        5,
        Duration::from_millis(100),
    );
    
    // Try to update prediction (should be no-op)
    let trade = Trade {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        price: Price::from_str("100.00").unwrap(),
        size: Size::from_str("1.0").unwrap(),
        side: OrderSide::Buy,
        timestamp: 1000,
        trade_id: Some("trade1".to_string()),
    };
    
    strategy.update_prediction(&trade);
    
    // Strategy should still work normally
    let mut market_state = MarketState::new("BTCUSDT".to_string());
    
    let snapshot = OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        vec![OrderBookLevel::new(
            Price::from_str("100.00").unwrap(),
            Size::from_str("10.0").unwrap())
        ],
        vec![OrderBookLevel::new(
            Price::from_str("101.00").unwrap(),
            Size::from_str("10.0").unwrap())
        ],
        2000,
    );
    
    let event = MarketEvent::OrderBookSnapshot(snapshot);
    market_state.update(&event);
    
    let signal = strategy.generate_signal(&market_state);
    assert!(signal.is_some());
}

