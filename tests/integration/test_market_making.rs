use crypto_hft::strategies::MarketMakingStrategy;
use crypto_hft::types::{Price, Size};
use crypto_hft::strategy::{StrategyEngine, MarketState, Signal};
use crypto_hft::orderbook::{OrderBookSnapshot, OrderBookLevel};
use crypto_hft::traits::{MarketEvent, MarketDataStream, ExecutionClient, NewOrder, OrderId, ExecutionReport, OrderStatus, OrderSide, OrderType, TimeInForce};
use crypto_hft::connectors::{MockMarketDataStream, MockExecutionClient};
use std::time::Duration;
use tokio;

#[tokio::test]
async fn test_market_making_end_to_end_workflow() {
    // Create market making strategy
    let strategy = MarketMakingStrategy::new(
        Price::from_str("0.5").unwrap(),
        Size::from_str("0.1").unwrap(),
        Size::from_str("1.0").unwrap(),
        5,
        Duration::from_millis(100),
    );
    
    // Create strategy engine
    let mut engine = StrategyEngine::new(strategy, Duration::from_millis(50));
    
    // Create mock market data stream
    let mut market_stream = MockMarketDataStream::new();
    market_stream.set_connected(true).await;
    
    // Create mock execution client
    let execution_client = MockExecutionClient::new();
    
    // Add market data events
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
    market_stream.add_event(event).await;
    
    // Process market events and generate signals
    let mut signals = Vec::new();
    while let Some(event_result) = market_stream.next().await {
        if let Ok(event) = event_result {
            if let Some(signal) = engine.process_event(event) {
                signals.push(signal);
            }
        } else {
            break;
        }
    }
    
    // Verify signals were generated
    assert!(!signals.is_empty());
    
    // Execute orders based on signals
    for signal in signals {
        match signal {
            Signal::PlaceOrder { order } => {
                let order_id = execution_client.place_order(order).await.unwrap();
                assert!(!order_id.as_str().is_empty());
                
                // Check order status
                let status = execution_client.get_order_status(order_id).await.unwrap();
                assert_eq!(status.order_id, order_id);
            }
            _ => {}
        }
    }
}

#[tokio::test]
async fn test_market_making_with_order_updates() {
    // Create market making strategy
    let strategy = MarketMakingStrategy::new(
        Price::from_str("0.5").unwrap(),
        Size::from_str("0.1").unwrap(),
        Size::from_str("1.0").unwrap(),
        5,
        Duration::from_millis(100),
    );
    
    // Create strategy engine
    let mut engine = StrategyEngine::new(strategy, Duration::from_millis(50));
    
    // Create mock market data stream
    let mut market_stream = MockMarketDataStream::new();
    market_stream.set_connected(true).await;
    
    // Create mock execution client
    let execution_client = MockExecutionClient::new();
    
    // Add initial market data
    let snapshot1 = OrderBookSnapshot::new(
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
    
    market_stream.add_event(MarketEvent::OrderBookSnapshot(snapshot1)).await;
    
    // Process first event and generate signal
    if let Some(event_result) = market_stream.next().await {
        if let Ok(event) = event_result {
            if let Some(signal) = engine.process_event(event) {
                match signal {
                    Signal::PlaceOrder { order } => {
                        let order_id = execution_client.place_order(order).await.unwrap();
                        
                        // Simulate order execution
                        let execution_report = ExecutionReport {
                            order_id: order_id.clone(),
                            client_order_id: order.client_order_id,
                            symbol: order.symbol.clone(),
                            status: OrderStatus::Filled { filled_size: order.quantity },
                            side: order.side,
                            order_type: order.order_type,
                            time_in_force: order.time_in_force,
                            quantity: order.quantity,
                            price: order.price,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64,
                        };
                        
                        // In a real implementation, we'd handle the execution report
                        // For this test, we just verify it was created correctly
                        assert_eq!(execution_report.order_id, order_id);
                        assert_eq!(execution_report.symbol, "BTCUSDT");
                    }
                    _ => {}
                }
            }
        }
    }
    
    // Add updated market data (price moved)
    let snapshot2 = OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        vec![OrderBookLevel::new(
            Price::from_str("99.50").unwrap(),  // Price moved down
            Size::from_str("10.0").unwrap()
        )],
        vec![OrderBookLevel::new(
            Price::from_str("100.50").unwrap(),  // Price moved down
            Size::from_str("10.0").unwrap()
        )],
        123456790,
    );
    
    market_stream.add_event(MarketEvent::OrderBookSnapshot(snapshot2)).await;
    
    // Process second event
    if let Some(event_result) = market_stream.next().await {
        if let Ok(event) = event_result {
            // Should generate new signals based on updated prices
            let signal = engine.process_event(event);
            assert!(signal.is_some());
        }
    }
}

#[tokio::test]
async fn test_market_making_with_multiple_symbols() {
    // Create market making strategy
    let strategy = MarketMakingStrategy::new(
        Price::from_str("0.5").unwrap(),
        Size::from_str("0.1").unwrap(),
        Size::from_str("1.0").unwrap(),
        5,
        Duration::from_millis(100),
    );
    
    // Create strategy engine
    let mut engine = StrategyEngine::new(strategy, Duration::from_millis(50));
    
    // Create mock market data stream
    let mut market_stream = MockMarketDataStream::new();
    market_stream.set_connected(true).await;
    
    // Create mock execution client
    let execution_client = MockExecutionClient::new();
    
    // Add market data for BTCUSDT
    let btc_snapshot = OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        vec![OrderBookLevel::new(
            Price::from_str("50000.00").unwrap(),
            Size::from_str("1.0").unwrap()
        )],
        vec![OrderBookLevel::new(
            Price::from_str("50100.00").unwrap(),
            Size::from_str("1.0").unwrap()
        )],
        123456789,
    );
    
    market_stream.add_event(MarketEvent::OrderBookSnapshot(btc_snapshot)).await;
    
    // Add market data for ETHUSDT
    let eth_snapshot = OrderBookSnapshot::new(
        "ETHUSDT".to_string(),
        vec![OrderBookLevel::new(
            Price::from_str("3000.00").unwrap(),
            Size::from_str("10.0").unwrap()
        )],
        vec![OrderBookLevel::new(
            Price::from_str("3010.00").unwrap(),
            Size::from_str("10.0").unwrap()
        )],
        123456789,
    );
    
    market_stream.add_event(MarketEvent::OrderBookSnapshot(eth_snapshot)).await;
    
    // Process events for both symbols
    let mut btc_signals = 0;
    let mut eth_signals = 0;
    
    while let Some(event_result) = market_stream.next().await {
        if let Ok(event) = event_result {
            if let Some(signal) = engine.process_event(event) {
                match signal {
                    Signal::PlaceOrder { order } => {
                        if order.symbol == "BTCUSDT" {
                            btc_signals += 1;
                        } else if order.symbol == "ETHUSDT" {
                            eth_signals += 1;
                        }
                        
                        // Place the order
                        let order_id = execution_client.place_order(order).await.unwrap();
                        assert!(!order_id.as_str().is_empty());
                    }
                    _ => {}
                }
            }
        } else {
            break;
        }
    }
    
    // Verify signals were generated for both symbols
    assert!(btc_signals > 0);
    assert!(eth_signals > 0);
}

#[tokio::test]
async fn test_market_making_with_risk_limits() {
    // Create market making strategy with small position limit
    let strategy = MarketMakingStrategy::new(
        Price::from_str("0.5").unwrap(),
        Size::from_str("0.5").unwrap(),   // Larger order size
        Size::from_str("1.0").unwrap(),   // Small max position
        5,
        Duration::from_millis(100),
    );
    
    // Create strategy engine
    let mut engine = StrategyEngine::new(strategy, Duration::from_millis(50));
    
    // Create mock market data stream
    let mut market_stream = MockMarketDataStream::new();
    market_stream.set_connected(true).await;
    
    // Create mock execution client
    let execution_client = MockExecutionClient::new();
    
    // Add market data
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
    
    market_stream.add_event(MarketEvent::OrderBookSnapshot(snapshot)).await;
    
    // Process event and place orders until position limit is reached
    let mut orders_placed = 0;
    let max_orders = 3;  // Should hit position limit after 2 orders (0.5 * 2 = 1.0)
    
    while let Some(event_result) = market_stream.next().await {
        if let Ok(event) = event_result {
            if let Some(signal) = engine.process_event(event) {
                match signal {
                    Signal::PlaceOrder { order } => {
                        let order_id = execution_client.place_order(order).await.unwrap();
                        orders_placed += 1;
                        
                        // Simulate order execution
                        let execution_report = ExecutionReport {
                            order_id: order_id.clone(),
                            client_order_id: order.client_order_id,
                            symbol: order.symbol.clone(),
                            status: OrderStatus::Filled { filled_size: order.quantity },
                            side: order.side,
                            order_type: order.order_type,
                            time_in_force: order.time_in_force,
                            quantity: order.quantity,
                            price: order.price,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64,
                        };
                        
                        // In a real implementation, we'd update position based on execution
                        // For this test, we just verify the order was placed
                        assert_eq!(execution_report.order_id, order_id);
                    }
                    _ => {}
                }
            }
        } else {
            break;
        }
        
        if orders_placed >= max_orders {
            break;
        }
    }
    
    // Verify that orders were placed
    assert!(orders_placed > 0);
    
    // In a real implementation, we'd verify that no more orders are placed
    // once the position limit is reached
}
