// Phase 6: Exchange Integration Fixes - TDD Tests
// These tests verify the compilation fixes for exchange-related modules

use crypto_hft::core::events::{
    OrderBookSnapshot, OrderBookDelta, OrderBookLevel, Trade, MarketEvent,
    OrderSide, OrderType, TimeInForce, OrderStatus, NewOrder, ExecutionReport,
    Balance, TradingFees,
};
use crypto_hft::types::{Price, Size, Symbol};

#[cfg(test)]
mod binance_websocket_tests {
    use super::*;

    #[test]
    fn test_order_book_delta_creation_with_exchange_id() {
        // T049: Verify OrderBookDelta includes exchange_id
        let bids = vec![
            OrderBookLevel::new(
                Price::from_str("50000.00").unwrap(),
                Size::from_str("1.5").unwrap(),
            ),
        ];
        let asks = vec![
            OrderBookLevel::new(
                Price::from_str("50001.00").unwrap(),
                Size::from_str("2.0").unwrap(),
            ),
        ];

        let delta = OrderBookDelta::new(
            "BTCUSDT",
            "binance", // exchange_id required
            bids,
            asks,
            1699000000000u64,
        );

        assert_eq!(delta.symbol.as_str(), "BTCUSDT");
        assert_eq!(delta.exchange_id, "binance");
        assert_eq!(delta.timestamp, 1699000000000u64);
    }

    #[test]
    fn test_order_book_snapshot_creation_with_exchange_id() {
        // Verify OrderBookSnapshot includes exchange_id
        let bids = vec![];
        let asks = vec![];

        let snapshot = OrderBookSnapshot::new(
            "ETHUSDT",
            "binance",
            bids,
            asks,
            1699000000000u64,
        );

        assert_eq!(snapshot.symbol.as_str(), "ETHUSDT");
        assert_eq!(snapshot.exchange_id, "binance");
    }

    #[test]
    fn test_market_event_order_book_delta() {
        // Verify MarketEvent can contain OrderBookDelta with exchange_id
        let delta = OrderBookDelta::new(
            "BTCUSDT",
            "binance",
            vec![],
            vec![],
            1699000000000u64,
        );

        let event = MarketEvent::OrderBookDelta(delta);
        match event {
            MarketEvent::OrderBookDelta(d) => {
                assert_eq!(d.symbol.as_str(), "BTCUSDT");
                assert_eq!(d.exchange_id, "binance");
            }
            _ => panic!("Expected OrderBookDelta event"),
        }
    }
}

#[cfg(test)]
mod dry_run_connector_tests {
    use super::*;

    #[test]
    fn test_new_order_uses_size_field() {
        // T073: Verify NewOrder uses 'size' field, not 'quantity'
        let order = NewOrder {
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "dry_run".to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::GoodTillCancelled,
            price: Some(Price::from_str("50000.00").unwrap()),
            size: Size::from_str("1.0").unwrap(), // Use 'size', not 'quantity'
            client_order_id: Some("test_123".to_string()),
        };

        assert_eq!(order.size, Size::from_str("1.0").unwrap());
        assert!(order.price.is_some());
    }

    #[test]
    fn test_execution_report_structure() {
        // Verify ExecutionReport has correct fields
        let report = ExecutionReport {
            order_id: "test_order_1".to_string(),
            client_order_id: Some("client_123".to_string()),
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "dry_run".to_string(),
            status: OrderStatus::New,
            filled_size: Size::from_str("0.0").unwrap(),
            remaining_size: Size::from_str("1.0").unwrap(),
            average_price: None,
            timestamp: 1699000000000u64,
        };

        assert_eq!(report.order_id, "test_order_1");
        assert_eq!(report.symbol.as_str(), "BTCUSDT");
        assert_eq!(report.exchange_id, "dry_run");
        assert_eq!(report.status, OrderStatus::New);
    }

    #[test]
    fn test_order_status_cancelled_variant() {
        // T073: Verify OrderStatus::Cancelled spelling (not Canceled)
        let status = OrderStatus::Cancelled;
        assert_eq!(status, OrderStatus::Cancelled);
    }

    #[test]
    fn test_symbol_comparison_with_string() {
        // Verify Symbol can be compared with string via conversion
        let symbol = Symbol::new("BTCUSDT");
        let s = "BTCUSDT";
        
        // Use as_str() for comparison
        assert_eq!(symbol.as_str(), s);
        
        // Or use Into for conversion
        let symbol_from_str: Symbol = s.into();
        assert_eq!(symbol, symbol_from_str);
    }
}

#[cfg(test)]
mod mock_exchange_tests {
    use super::*;

    #[test]
    fn test_trading_fees_structure() {
        // T054: Verify TradingFees structure
        let fees = TradingFees::new(
            "BTCUSDT".to_string(),
            Size::from_str("0.001").unwrap(), // maker fee
            Size::from_str("0.001").unwrap(), // taker fee
        );

        assert_eq!(fees.symbol, "BTCUSDT");
        assert_eq!(fees.maker_fee, rust_decimal::Decimal::new(1, 3)); // 0.001
        assert_eq!(fees.taker_fee, rust_decimal::Decimal::new(1, 3)); // 0.001
    }

    #[test]
    fn test_balance_creation() {
        // Verify Balance structure
        let balance = Balance::new(
            "BTC".to_string(),
            Size::from_str("10.0").unwrap(),  // total
            Size::from_str("1.0").unwrap(),   // used
        );

        assert_eq!(balance.asset, "BTC");
        assert_eq!(balance.total, rust_decimal::Decimal::new(100, 1)); // 10.0
        assert_eq!(balance.used, rust_decimal::Decimal::new(10, 1));   // 1.0
        assert_eq!(balance.free, rust_decimal::Decimal::new(90, 1));   // 9.0
    }
}

#[cfg(test)]
mod connection_manager_tests {
    use super::*;

    #[test]
    fn test_market_event_variants() {
        // Verify all MarketEvent variants work correctly
        let snapshot = OrderBookSnapshot::new(
            "BTCUSDT",
            "test",
            vec![],
            vec![],
            1699000000000u64,
        );
        let event1 = MarketEvent::OrderBookSnapshot(snapshot);
        assert!(matches!(event1, MarketEvent::OrderBookSnapshot(_)));

        let delta = OrderBookDelta::new(
            "BTCUSDT",
            "test",
            vec![],
            vec![],
            1699000000000u64,
        );
        let event2 = MarketEvent::OrderBookDelta(delta);
        assert!(matches!(event2, MarketEvent::OrderBookDelta(_)));

        let trade = Trade {
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "test".to_string(),
            price: Price::from_str("50000.00").unwrap(),
            size: Size::from_str("1.0").unwrap(),
            side: OrderSide::Buy,
            timestamp: 1699000000000u64,
            trade_id: None,
        };
        let event3 = MarketEvent::Trade(trade);
        assert!(matches!(event3, MarketEvent::Trade(_)));
    }
}

