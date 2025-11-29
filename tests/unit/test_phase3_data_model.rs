/// Phase 3 TDD Tests: Data Model Corrections
/// 
/// These tests verify that all data structures have the correct fields and methods
/// according to the spec and compilation requirements.
/// 
/// Run with: cargo test test_phase3_data_model --lib
/// 
/// Note: These tests require the full project to compile. The tests in 
/// src/types/symbol.rs and src/core/events.rs can be run as unit tests.

use crypto_hft::core::events::*;
use crypto_hft::types::*;
use rust_decimal::Decimal;

#[cfg(test)]
mod symbol_api_tests {
    use super::*;

    #[test]
    fn test_symbol_len() {
        let symbol = Symbol::new("BTCUSDT");
        assert_eq!(symbol.len(), 7);
        
        let empty = Symbol::new("");
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn test_symbol_as_str() {
        let symbol = Symbol::new("ETHUSDT");
        assert_eq!(symbol.as_str(), "ETHUSDT");
    }

    #[test]
    fn test_symbol_indexing() {
        let symbol = Symbol::new("BTCUSDT");
        
        // Test slicing
        assert_eq!(&symbol[0..3], "BTC");
        assert_eq!(&symbol[3..7], "USDT");
        assert_eq!(&symbol[..3], "BTC");
        assert_eq!(&symbol[3..], "USDT");
    }

    #[test]
    fn test_symbol_index_range() {
        let symbol = Symbol::new("BTCUSDT");
        let len = symbol.len();
        
        if len >= 3 {
            let _prefix = &symbol[..len - 4];
        }
    }
}

#[cfg(test)]
mod new_order_tests {
    use super::*;

    #[test]
    fn test_new_order_has_size_field() {
        let symbol = Symbol::new("BTCUSDT");
        let size = Size::new(Decimal::new(100, 2)); // 1.00
        let price = Price::new(Decimal::new(5000000, 2)); // 50000.00
        
        let order = NewOrder {
            symbol: symbol.clone(),
            exchange_id: "binance".to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::GoodTillCancelled,
            price: Some(price),
            size: size.clone(),
            client_order_id: Some("test-123".to_string()),
        };
        
        // Verify we can access size field
        assert_eq!(order.size, size);
    }

    #[test]
    fn test_new_order_size_not_quantity() {
        // This test ensures that NewOrder uses 'size' field, not 'quantity'
        // If this compiles, it means the field is named correctly
        let order = NewOrder {
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "binance".to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Market,
            time_in_force: TimeInForce::ImmediateOrCancel,
            price: None,
            size: Size::new(Decimal::new(100, 2)),
            client_order_id: None,
        };
        
        let _order_size = order.size; // Should compile with 'size', not 'quantity'
    }
}

#[cfg(test)]
mod trading_fees_tests {
    use super::*;

    #[test]
    fn test_trading_fees_has_symbol_field() {
        let fees = TradingFees {
            symbol: "BTCUSDT".to_string(),
            maker_fee: Decimal::new(1, 4), // 0.0001
            taker_fee: Decimal::new(2, 4), // 0.0002
        };
        
        assert_eq!(fees.symbol, "BTCUSDT");
        assert_eq!(fees.maker_fee, Decimal::new(1, 4));
        assert_eq!(fees.taker_fee, Decimal::new(2, 4));
    }

    #[test]
    fn test_trading_fees_new_constructor() {
        let fees = TradingFees::new(
            "ETHUSDT".to_string(),
            Size::new(Decimal::new(1, 4)),
            Size::new(Decimal::new(2, 4)),
        );
        
        assert_eq!(fees.symbol, "ETHUSDT");
    }
}

#[cfg(test)]
mod execution_report_tests {
    use super::*;

    #[test]
    fn test_execution_report_has_average_price() {
        let report = ExecutionReport {
            order_id: "order-123".to_string(),
            client_order_id: Some("client-123".to_string()),
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "binance".to_string(),
            status: OrderStatus::PartiallyFilled,
            filled_size: Size::new(Decimal::new(50, 2)),
            remaining_size: Size::new(Decimal::new(50, 2)),
            average_price: Some(Price::new(Decimal::new(5000000, 2))),
            timestamp: 1638368000000,
        };
        
        assert!(report.average_price.is_some());
        assert_eq!(
            report.average_price.unwrap(),
            Price::new(Decimal::new(5000000, 2))
        );
    }

    #[test]
    fn test_execution_report_average_price_optional() {
        // Test that average_price can be None (e.g., for New orders)
        let report = ExecutionReport {
            order_id: "order-123".to_string(),
            client_order_id: None,
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "binance".to_string(),
            status: OrderStatus::New,
            filled_size: Size::new(Decimal::ZERO),
            remaining_size: Size::new(Decimal::new(100, 2)),
            average_price: None,
            timestamp: 1638368000000,
        };
        
        assert!(report.average_price.is_none());
    }
}

#[cfg(test)]
mod orderbook_snapshot_tests {
    use super::*;

    #[test]
    fn test_orderbook_snapshot_new_constructor() {
        let symbol = Symbol::new("BTCUSDT");
        let exchange_id = "binance".to_string();
        let bids = vec![
            OrderBookLevel::new(
                Price::new(Decimal::new(5000000, 2)),
                Size::new(Decimal::new(100, 2)),
            ),
        ];
        let asks = vec![
            OrderBookLevel::new(
                Price::new(Decimal::new(5000100, 2)),
                Size::new(Decimal::new(100, 2)),
            ),
        ];
        let timestamp = 1638368000000;
        
        let snapshot = OrderBookSnapshot::new(
            symbol.clone(),
            exchange_id.clone(),
            bids.clone(),
            asks.clone(),
            timestamp,
        );
        
        assert_eq!(snapshot.symbol, symbol);
        assert_eq!(snapshot.exchange_id, exchange_id);
        assert_eq!(snapshot.bids.len(), 1);
        assert_eq!(snapshot.asks.len(), 1);
        assert_eq!(snapshot.timestamp, timestamp);
    }

    #[test]
    fn test_orderbook_snapshot_new_with_string() {
        // Test that new() accepts impl Into<Symbol> and impl Into<String>
        let snapshot = OrderBookSnapshot::new(
            "BTCUSDT",
            "binance",
            vec![],
            vec![],
            1638368000000,
        );
        
        assert_eq!(snapshot.symbol, Symbol::new("BTCUSDT"));
        assert_eq!(snapshot.exchange_id, "binance");
    }
}

#[cfg(test)]
mod order_field_tests {
    use super::*;

    #[test]
    fn test_order_has_size_field() {
        let order = Order {
            order_id: "order-123".to_string(),
            client_order_id: Some("client-123".to_string()),
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "binance".to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::GoodTillCancelled,
            price: Some(Price::new(Decimal::new(5000000, 2))),
            size: Size::new(Decimal::new(100, 2)),
            filled_size: Size::new(Decimal::new(50, 2)),
            status: OrderStatus::PartiallyFilled,
            timestamp: 1638368000000,
        };
        
        // Verify size field exists and is accessible
        assert_eq!(order.size, Size::new(Decimal::new(100, 2)));
    }
}

