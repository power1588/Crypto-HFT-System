use crypto_hft::risk::ShadowLedger;
use crypto_hft::risk::shadow_ledger::{TradeRecord, PositionRecord, PositionStats, TradeStats};
use crypto_hft::types::{Price, Size, Symbol};
use crypto_hft::core::events::{OrderSide, OrderStatus, ExecutionReport};
use crypto_hft::traits::{OrderId, OrderType, TimeInForce};
use chrono::Utc;
use std::collections::HashMap;

#[tokio::test]
async fn test_shadow_ledger_creation() {
    let ledger = ShadowLedger::new();
    
    // Initially no positions or trades
    let positions = ledger.get_all_positions().await;
    assert!(positions.is_empty());
    
    let trades = ledger.get_all_trades().await;
    assert!(trades.is_empty());
    
    // Daily P&L should be zero
    let daily_pnl = ledger.get_daily_pnl("2023-01-01").await;
    assert_eq!(daily_pnl, crate::rust_decimal::Decimal::ZERO);
}

#[tokio::test]
async fn test_trade_record_creation() {
    let trade = TradeRecord::new(
        "trade_123".to_string(),
        Symbol::new("BTCUSDT"),
        "binance".to_string(),
        "order_456".to_string(),
        OrderSide::Buy,
        Size::from_str("1.0").unwrap(),
        Price::from_str("50000.0").unwrap(),
        Utc::now(),
        Size::from_str("0.001").unwrap(),
        "BTC".to_string(),
    );
    
    assert_eq!(trade.trade_id, "trade_123");
    assert_eq!(trade.symbol.value(), "BTCUSDT");
    assert_eq!(trade.exchange_id, "binance");
    assert_eq!(trade.order_id, "order_456");
    assert_eq!(trade.side, OrderSide::Buy);
    assert_eq!(trade.quantity, Size::from_str("1.0").unwrap());
    assert_eq!(trade.price, Price::from_str("50000.0").unwrap());
    
    // Check trade value
    assert_eq!(trade.value(), crate::rust_decimal::Decimal::from_str("50000.0").unwrap());
    
    // Check net value (buy includes fee)
    assert_eq!(trade.net_value(), crate::rust_decimal::Decimal::from_str("50000.001").unwrap());
}

#[tokio::test]
async fn test_position_record_creation() {
    let position = PositionRecord::new(
        Symbol::new("BTCUSDT"),
        "binance".to_string(),
    );
    
    assert_eq!(position.symbol.value(), "BTCUSDT");
    assert_eq!(position.exchange_id, "binance");
    assert_eq!(position.size, Size::new(crate::rust_decimal::Decimal::ZERO));
    assert!(position.average_price.is_none());
    assert_eq!(position.total_cost, crate::rust_decimal::Decimal::ZERO);
    assert_eq!(position.realized_pnl, crate::rust_decimal::Decimal::ZERO);
}

#[tokio::test]
async fn test_position_record_apply_trade() {
    let mut position = PositionRecord::new(
        Symbol::new("BTCUSDT"),
        "binance".to_string(),
    );
    
    // Apply a buy trade
    let buy_trade = TradeRecord::new(
        "trade_123".to_string(),
        Symbol::new("BTCUSDT"),
        "binance".to_string(),
        "order_456".to_string(),
        OrderSide::Buy,
        Size::from_str("1.0").unwrap(),
        Price::from_str("50000.0").unwrap(),
        Utc::now(),
        Size::from_str("0.001").unwrap(),
        "BTC".to_string(),
    );
    
    position.apply_trade(&buy_trade);
    
    assert_eq!(position.size, Size::from_str("1.0").unwrap());
    assert_eq!(position.total_cost, crate::rust_decimal::Decimal::from_str("50000.001").unwrap());
    assert_eq!(position.average_price, Some(Price::from_str("50000.001").unwrap()));
    
    // Apply a sell trade (partial)
    let sell_trade = TradeRecord::new(
        "trade_124".to_string(),
        Symbol::new("BTCUSDT"),
        "binance".to_string(),
        "order_457".to_string(),
        OrderSide::Sell,
        Size::from_str("0.3").unwrap(),
        Price::from_str("51000.0").unwrap(),
        Utc::now(),
        Size::from_str("0.001").unwrap(),
        "BTC".to_string(),
    );
    
    position.apply_trade(&sell_trade);
    
    assert_eq!(position.size, Size::from_str("0.7").unwrap()); // 1.0 - 0.3
    // Realized P&L should be approximately (51000 - 50000.001) * 0.3 = 299.9997
    assert!(position.realized_pnl > crate::rust_decimal::Decimal::from_str("299.0").unwrap());
    assert!(position.realized_pnl < crate::rust_decimal::Decimal::from_str("301.0").unwrap());
}

#[tokio::test]
async fn test_position_record_unrealized_pnl() {
    let mut position = PositionRecord::new(
        Symbol::new("BTCUSDT"),
        "binance".to_string(),
    );
    
    // Apply a buy trade
    let buy_trade = TradeRecord::new(
        "trade_123".to_string(),
        Symbol::new("BTCUSDT"),
        "binance".to_string(),
        "order_456".to_string(),
        OrderSide::Buy,
        Size::from_str("1.0").unwrap(),
        Price::from_str("50000.0").unwrap(),
        Utc::now(),
        Size::from_str("0.001").unwrap(),
        "BTC".to_string(),
    );
    
    position.apply_trade(&buy_trade);
    
    // Check unrealized P&L at higher price
    let higher_price = Price::from_str("51000.0").unwrap();
    let unrealized_pnl = position.unrealized_pnl(higher_price).unwrap();
    // Should be approximately (51000 - 50000.001) * 1.0 = 999.999
    assert!(unrealized_pnl > crate::rust_decimal::Decimal::from_str("999.0").unwrap());
    assert!(unrealized_pnl < crate::rust_decimal::Decimal::from_str("1001.0").unwrap());
    
    // Check unrealized P&L at lower price
    let lower_price = Price::from_str("49000.0").unwrap();
    let unrealized_pnl = position.unrealized_pnl(lower_price).unwrap();
    // Should be approximately (49000 - 50000.001) * 1.0 = -1000.001
    assert!(unrealized_pnl < crate::rust_decimal::Decimal::from_str("-999.0").unwrap());
    assert!(unrealized_pnl > crate::rust_decimal::Decimal::from_str("-1001.0").unwrap());
}

#[tokio::test]
async fn test_shadow_ledger_add_trade() {
    let ledger = ShadowLedger::new();
    
    // Add a trade
    let trade = TradeRecord::new(
        "trade_123".to_string(),
        Symbol::new("BTCUSDT"),
        "binance".to_string(),
        "order_456".to_string(),
        OrderSide::Buy,
        Size::from_str("1.0").unwrap(),
        Price::from_str("50000.0").unwrap(),
        Utc::now(),
        Size::from_str("0.001").unwrap(),
        "BTC".to_string(),
    );
    
    ledger.add_trade(trade).await;
    
    // Check trade was added
    let trades = ledger.get_all_trades().await;
    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].trade_id, "trade_123");
    
    // Check position was created
    let position = ledger.get_position("BTCUSDT", "binance").await;
    assert!(position.is_some());
    assert_eq!(position.unwrap().size, Size::from_str("1.0").unwrap());
}

#[tokio::test]
async fn test_shadow_ledger_process_execution_report() {
    let ledger = ShadowLedger::new();
    
    // Create a filled execution report
    let report = ExecutionReport {
        order_id: OrderId::new("order_456".to_string()),
        client_order_id: Some("client_123".to_string()),
        symbol: "BTCUSDT".to_string(),
        status: OrderStatus::Filled { 
            filled_size: Size::from_str("1.0").unwrap()
        },
        side: OrderSide::Buy,
        order_type: OrderType::Limit,
        time_in_force: TimeInForce::GoodTillCancelled,
        quantity: Size::from_str("1.0").unwrap(),
        price: Some(Price::from_str("50000.0").unwrap()),
        timestamp: Utc::now().timestamp_millis() as u64,
    };
    
    // Process the execution report
    ledger.process_execution_report(&report).await;
    
    // Check trade was added
    let trades = ledger.get_all_trades().await;
    assert_eq!(trades.len(), 1);
    
    // Check position was created
    let position = ledger.get_position("BTCUSDT", "binance").await;
    assert!(position.is_some());
    assert_eq!(position.unwrap().size, Size::from_str("1.0").unwrap());
}

#[tokio::test]
async fn test_shadow_ledger_pnl_calculation() {
    let ledger = ShadowLedger::new();
    
    // Add a buy trade
    let buy_trade = TradeRecord::new(
        "trade_123".to_string(),
        Symbol::new("BTCUSDT"),
        "binance".to_string(),
        "order_456".to_string(),
        OrderSide::Buy,
        Size::from_str("1.0").unwrap(),
        Price::from_str("50000.0").unwrap(),
        Utc::now(),
        Size::from_str("0.001").unwrap(),
        "BTC".to_string(),
    );
    
    ledger.add_trade(buy_trade).await;
    
    // Add a sell trade at profit
    let sell_trade = TradeRecord::new(
        "trade_124".to_string(),
        Symbol::new("BTCUSDT"),
        "binance".to_string(),
        "order_457".to_string(),
        OrderSide::Sell,
        Size::from_str("1.0").unwrap(),
        Price::from_str("51000.0").unwrap(),
        Utc::now(),
        Size::from_str("0.001").unwrap(),
        "BTC".to_string(),
    );
    
    ledger.add_trade(sell_trade).await;
    
    // Check realized P&L
    let realized_pnl = ledger.get_total_realized_pnl().await;
    // Should be approximately (51000 - 50000.001) * 1.0 = 999.999
    assert!(realized_pnl > crate::rust_decimal::Decimal::from_str("999.0").unwrap());
    assert!(realized_pnl < crate::rust_decimal::Decimal::from_str("1001.0").unwrap());
    
    // Check unrealized P&L with current market price
    let mut market_prices = HashMap::new();
    market_prices.insert("BTCUSDT".to_string(), Price::from_str("50500.0").unwrap());
    
    // Position should be closed, so unrealized should be zero
    let unrealized_pnl = ledger.get_total_unrealized_pnl(&market_prices).await;
    assert_eq!(unrealized_pnl, crate::rust_decimal::Decimal::ZERO);
}

#[tokio::test]
async fn test_shadow_ledger_stats() {
    let ledger = ShadowLedger::new();
    
    // Add multiple trades for different symbols
    for i in 1..=3 {
        let symbol = format!("SYMBOL{}USDT", i);
        let side = if i % 2 == 0 { OrderSide::Buy } else { OrderSide::Sell };
        
        let trade = TradeRecord::new(
            format!("trade_{}", i),
            Symbol::new(symbol.clone()),
            "binance".to_string(),
            format!("order_{}", i),
            side,
            Size::from_str("1.0").unwrap(),
            Price::from_str("50000.0").unwrap(),
            Utc::now(),
            Size::from_str("0.001").unwrap(),
            "USDT".to_string(),
        );
        
        ledger.add_trade(trade).await;
    }
    
    // Check position stats
    let position_stats = ledger.get_position_stats().await;
    assert_eq!(position_stats.total_positions, 3);
    
    // Check trade stats
    let trade_stats = ledger.get_trade_stats().await;
    assert_eq!(trade_stats.total_trades, 3);
    assert_eq!(trade_stats.buy_trades, 1);
    assert_eq!(trade_stats.sell_trades, 2);
    assert_eq!(trade_stats.total_volume, Size::from_str("3.0").unwrap());
    assert_eq!(trade_stats.total_value, crate::rust_decimal::Decimal::from_str("150000.0").unwrap());
    assert_eq!(trade_stats.total_fees, Size::from_str("0.003").unwrap());
}

#[tokio::test]
async fn test_shadow_ledger_get_trades_for_symbol() {
    let ledger = ShadowLedger::new();
    
    // Add trades for different symbols
    let trade1 = TradeRecord::new(
        "trade_1".to_string(),
        Symbol::new("BTCUSDT"),
        "binance".to_string(),
        "order_1".to_string(),
        OrderSide::Buy,
        Size::from_str("1.0").unwrap(),
        Price::from_str("50000.0").unwrap(),
        Utc::now(),
        Size::from_str("0.001").unwrap(),
        "BTC".to_string(),
    );
    
    let trade2 = TradeRecord::new(
        "trade_2".to_string(),
        Symbol::new("ETHUSDT"),
        "binance".to_string(),
        "order_2".to_string(),
        OrderSide::Buy,
        Size::from_str("1.0").unwrap(),
        Price::from_str("3000.0").unwrap(),
        Utc::now(),
        Size::from_str("0.001").unwrap(),
        "ETH".to_string(),
    );
    
    let trade3 = TradeRecord::new(
        "trade_3".to_string(),
        Symbol::new("BTCUSDT"),
        "binance".to_string(),
        "order_3".to_string(),
        OrderSide::Sell,
        Size::from_str("0.5").unwrap(),
        Price::from_str("51000.0").unwrap(),
        Utc::now(),
        Size::from_str("0.001").unwrap(),
        "BTC".to_string(),
    );
    
    ledger.add_trade(trade1).await;
    ledger.add_trade(trade2).await;
    ledger.add_trade(trade3).await;
    
    // Get trades for BTCUSDT
    let btc_trades = ledger.get_trades_for_symbol("BTCUSDT").await;
    assert_eq!(btc_trades.len(), 2);
    
    // Get trades for ETHUSDT
    let eth_trades = ledger.get_trades_for_symbol("ETHUSDT").await;
    assert_eq!(eth_trades.len(), 1);
}

#[tokio::test]
async fn test_shadow_ledger_daily_pnl() {
    let ledger = ShadowLedger::new();
    
    // Add trades on the same day
    let today = Utc::now().format("%Y-%m-%d").to_string();
    
    let buy_trade = TradeRecord::new(
        "trade_1".to_string(),
        Symbol::new("BTCUSDT"),
        "binance".to_string(),
        "order_1".to_string(),
        OrderSide::Buy,
        Size::from_str("1.0").unwrap(),
        Price::from_str("50000.0").unwrap(),
        Utc::now(),
        Size::from_str("0.001").unwrap(),
        "BTC".to_string(),
    );
    
    ledger.add_trade(buy_trade).await;
    
    let sell_trade = TradeRecord::new(
        "trade_2".to_string(),
        Symbol::new("BTCUSDT"),
        "binance".to_string(),
        "order_2".to_string(),
        OrderSide::Sell,
        Size::from_str("1.0").unwrap(),
        Price::from_str("51000.0").unwrap(),
        Utc::now(),
        Size::from_str("0.001").unwrap(),
        "BTC".to_string(),
    );
    
    ledger.add_trade(sell_trade).await;
    
    // Check daily P&L
    let daily_pnl = ledger.get_daily_pnl(&today).await;
    // Should be positive (profit)
    assert!(daily_pnl > crate::rust_decimal::Decimal::ZERO);
    
    // Reset daily P&L
    ledger.reset_daily_pnl().await;
    let daily_pnl_after_reset = ledger.get_daily_pnl(&today).await;
    assert_eq!(daily_pnl_after_reset, crate::rust_decimal::Decimal::ZERO);
}

