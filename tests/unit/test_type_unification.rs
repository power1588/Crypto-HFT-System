/// Tests for Phase 2: Type System Unification
/// These tests verify that there is only one canonical definition of each type
/// and that all types are accessible from the correct modules.

#[cfg(test)]
mod type_unification_tests {
    use crypto_hft::core::events::*;
    use crypto_hft::types::{Price, Size, Symbol};
    
    #[test]
    fn test_market_event_unified() {
        // Verify MarketEvent is defined in core::events
        let symbol = Symbol::new("BTCUSDT");
        let exchange_id = "binance".to_string();
        let timestamp = 1638368000000u64;
        
        // Test OrderBookSnapshot variant
        let snapshot = OrderBookSnapshot {
            symbol: symbol.clone(),
            exchange_id: exchange_id.clone(),
            bids: vec![],
            asks: vec![],
            timestamp,
        };
        
        let event = MarketEvent::OrderBookSnapshot(snapshot);
        match event {
            MarketEvent::OrderBookSnapshot(s) => {
                assert_eq!(s.symbol, symbol);
                assert_eq!(s.exchange_id, exchange_id);
            }
            _ => panic!("Expected OrderBookSnapshot"),
        }
    }
    
    #[test]
    fn test_market_event_trade_unified() {
        // Verify Trade variant has all required fields
        let symbol = Symbol::new("BTCUSDT");
        let exchange_id = "binance".to_string();
        let price = Price::new(rust_decimal::Decimal::new(10000, 2));
        let size = Size::new(rust_decimal::Decimal::new(100, 2));
        let timestamp = 1638368000000u64;
        
        let trade = Trade {
            symbol: symbol.clone(),
            exchange_id: exchange_id.clone(),
            price,
            size,
            side: OrderSide::Buy,
            timestamp,
            trade_id: Some("trade-123".to_string()),
        };
        
        let event = MarketEvent::Trade(trade);
        match event {
            MarketEvent::Trade(t) => {
                assert_eq!(t.symbol, symbol);
                assert_eq!(t.exchange_id, exchange_id);
                assert_eq!(t.price, price);
                assert_eq!(t.size, size);
                assert!(t.trade_id.is_some());
            }
            _ => panic!("Expected Trade"),
        }
    }
    
    #[test]
    fn test_order_type_has_all_variants() {
        // Verify OrderType has all required variants including Stop and StopLimit
        let market_order = OrderType::Market;
        let limit_order = OrderType::Limit;
        let stop_loss_order = OrderType::StopLoss;
        let stop_limit_order = OrderType::StopLimit;
        
        assert_eq!(market_order, OrderType::Market);
        assert_eq!(limit_order, OrderType::Limit);
        assert_eq!(stop_loss_order, OrderType::StopLoss);
        assert_eq!(stop_limit_order, OrderType::StopLimit);
    }
    
    #[test]
    fn test_new_order_unified() {
        // Verify NewOrder uses Symbol type and has all required fields
        let symbol = Symbol::new("BTCUSDT");
        let exchange_id = "binance".to_string();
        let price = Price::new(rust_decimal::Decimal::new(10000, 2));
        let size = Size::new(rust_decimal::Decimal::new(100, 2));
        
        let order = NewOrder {
            symbol: symbol.clone(),
            exchange_id: exchange_id.clone(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::GoodTillCancelled,
            price: Some(price),
            size,  // Should be 'size' not 'quantity'
            client_order_id: Some("client-123".to_string()),
        };
        
        assert_eq!(order.symbol, symbol);
        assert_eq!(order.exchange_id, exchange_id);
        assert_eq!(order.size, size);
    }
    
    #[test]
    fn test_execution_report_has_average_price() {
        // Verify ExecutionReport has average_price field (needed per T016)
        let symbol = Symbol::new("BTCUSDT");
        let exchange_id = "binance".to_string();
        let filled_size = Size::new(rust_decimal::Decimal::new(100, 2));
        let remaining_size = Size::new(rust_decimal::Decimal::new(0, 2));
        let average_price = Price::new(rust_decimal::Decimal::new(10000, 2));
        
        let report = ExecutionReport {
            order_id: "order-123".to_string(),
            client_order_id: Some("client-123".to_string()),
            symbol,
            exchange_id,
            status: OrderStatus::Filled,
            filled_size,
            remaining_size,
            average_price: Some(average_price),
            timestamp: 1638368000000u64,
        };
        
        assert!(report.average_price.is_some());
        assert_eq!(report.average_price.unwrap(), average_price);
    }
    
    #[test]
    fn test_trading_fees_has_symbol() {
        // Verify TradingFees has symbol field (needed per T015)
        let symbol = "BTCUSDT".to_string();
        let maker_fee = Size::new(rust_decimal::Decimal::new(10, 4)); // 0.001
        let taker_fee = Size::new(rust_decimal::Decimal::new(20, 4)); // 0.002
        
        let fees = TradingFees::new(symbol.clone(), maker_fee, taker_fee);
        
        assert_eq!(fees.symbol, symbol);
        assert_eq!(fees.maker_fee, maker_fee.value());
        assert_eq!(fees.taker_fee, taker_fee.value());
    }
    
    #[test]
    fn test_types_accessible_from_traits_module() {
        // Verify types are accessible from traits module for backwards compatibility
        use crypto_hft::traits::{MarketEvent as TraitsMarketEvent, OrderSide as TraitsOrderSide};
        
        // These should be the same types, not different definitions
        let side = TraitsOrderSide::Buy;
        assert_eq!(format!("{:?}", side), "Buy");
    }
    
    #[test]
    fn test_order_book_snapshot_construction() {
        // Verify OrderBookSnapshot can be constructed properly
        let symbol = Symbol::new("BTCUSDT");
        let exchange_id = "binance".to_string();
        let timestamp = 1638368000000u64;
        
        let price1 = Price::new(rust_decimal::Decimal::new(10000, 2));
        let size1 = Size::new(rust_decimal::Decimal::new(100, 2));
        
        let level = OrderBookLevel {
            price: price1,
            size: size1,
        };
        
        let snapshot = OrderBookSnapshot {
            symbol,
            exchange_id,
            bids: vec![level.clone()],
            asks: vec![level],
            timestamp,
        };
        
        assert_eq!(snapshot.bids.len(), 1);
        assert_eq!(snapshot.asks.len(), 1);
    }
    
    #[test]
    fn test_signal_enum_complete() {
        // Verify Signal enum has all required variants (per T071)
        let symbol = Symbol::new("BTCUSDT");
        let exchange_id = "binance".to_string();
        let price = Price::new(rust_decimal::Decimal::new(10000, 2));
        let size = Size::new(rust_decimal::Decimal::new(100, 2));
        
        let order = NewOrder {
            symbol: symbol.clone(),
            exchange_id: exchange_id.clone(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::GoodTillCancelled,
            price: Some(price),
            size,
            client_order_id: None,
        };
        
        // Test all Signal variants exist
        let _place_order = Signal::PlaceOrder { order: order.clone() };
        let _cancel_order = Signal::CancelOrder {
            order_id: "order-123".to_string(),
            symbol: symbol.clone(),
            exchange_id: exchange_id.clone(),
        };
        let _cancel_all = Signal::CancelAllOrders {
            symbol: symbol.clone(),
            exchange_id: exchange_id.clone(),
        };
        let _update_order = Signal::UpdateOrder {
            order_id: "order-123".to_string(),
            price: Some(price),
            size: Some(size),
        };
    }
}

