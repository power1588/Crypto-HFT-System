use crate::types::{Price, Size, Symbol};
use serde::{Deserialize, Serialize};

/// Exchange identifier
pub type ExchangeId = String;

/// Order identifier
pub type OrderId = String;

/// Timestamp in milliseconds
pub type Timestamp = u64;

/// Order side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
    StopLoss,
    StopLimit,
}

/// Time in force
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    GoodTillCancelled,
    ImmediateOrCancel,
    FillOrKill,
}

/// Order status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
    Expired,
}

/// Order book level
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderBookLevel {
    pub price: Price,
    pub size: Size,
}

impl OrderBookLevel {
    pub fn new(price: Price, size: Size) -> Self {
        Self { price, size }
    }
}

/// Order book snapshot
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderBookSnapshot {
    pub symbol: Symbol,
    pub exchange_id: ExchangeId,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub timestamp: Timestamp,
}

impl OrderBookSnapshot {
    pub fn new(
        symbol: impl Into<Symbol>,
        exchange_id: impl Into<String>,
        bids: Vec<OrderBookLevel>,
        asks: Vec<OrderBookLevel>,
        timestamp: Timestamp,
    ) -> Self {
        Self {
            symbol: symbol.into(),
            exchange_id: exchange_id.into(),
            bids,
            asks,
            timestamp,
        }
    }
}

/// Order book delta
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderBookDelta {
    pub symbol: Symbol,
    pub exchange_id: ExchangeId,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub timestamp: Timestamp,
}

impl OrderBookDelta {
    pub fn new(
        symbol: impl Into<Symbol>,
        exchange_id: impl Into<String>,
        bids: Vec<OrderBookLevel>,
        asks: Vec<OrderBookLevel>,
        timestamp: Timestamp,
    ) -> Self {
        Self {
            symbol: symbol.into(),
            exchange_id: exchange_id.into(),
            bids,
            asks,
            timestamp,
        }
    }
}

/// Trade
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trade {
    pub symbol: Symbol,
    pub exchange_id: ExchangeId,
    pub price: Price,
    pub size: Size,
    pub side: OrderSide,
    pub timestamp: Timestamp,
    pub trade_id: Option<String>,
}

/// New order
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewOrder {
    pub symbol: Symbol,
    pub exchange_id: ExchangeId,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub time_in_force: TimeInForce,
    pub price: Option<Price>,
    pub size: Size,
    pub client_order_id: Option<String>,
}

impl NewOrder {
    /// Create a new limit buy order
    pub fn new_limit_buy(
        symbol: impl Into<String>,
        size: Size,
        price: Price,
        time_in_force: TimeInForce,
    ) -> Self {
        Self {
            symbol: Symbol::new(symbol),
            exchange_id: "default".to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            time_in_force,
            price: Some(price),
            size,
            client_order_id: None,
        }
    }

    /// Create a new limit sell order
    pub fn new_limit_sell(
        symbol: impl Into<String>,
        size: Size,
        price: Price,
        time_in_force: TimeInForce,
    ) -> Self {
        Self {
            symbol: Symbol::new(symbol),
            exchange_id: "default".to_string(),
            side: OrderSide::Sell,
            order_type: OrderType::Limit,
            time_in_force,
            price: Some(price),
            size,
            client_order_id: None,
        }
    }

    /// Create a new market buy order
    pub fn new_market_buy(symbol: impl Into<String>, size: Size) -> Self {
        Self {
            symbol: Symbol::new(symbol),
            exchange_id: "default".to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Market,
            time_in_force: TimeInForce::ImmediateOrCancel,
            price: None,
            size,
            client_order_id: None,
        }
    }

    /// Create a new market sell order
    pub fn new_market_sell(symbol: impl Into<String>, size: Size) -> Self {
        Self {
            symbol: Symbol::new(symbol),
            exchange_id: "default".to_string(),
            side: OrderSide::Sell,
            order_type: OrderType::Market,
            time_in_force: TimeInForce::ImmediateOrCancel,
            price: None,
            size,
            client_order_id: None,
        }
    }

    /// Set the client order ID (builder pattern)
    pub fn with_client_order_id(mut self, client_order_id: String) -> Self {
        self.client_order_id = Some(client_order_id);
        self
    }

    /// Set the exchange ID (builder pattern)
    pub fn with_exchange_id(mut self, exchange_id: impl Into<String>) -> Self {
        self.exchange_id = exchange_id.into();
        self
    }
}

/// Order
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Order {
    pub order_id: OrderId,
    pub client_order_id: Option<String>,
    pub symbol: Symbol,
    pub exchange_id: ExchangeId,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub time_in_force: TimeInForce,
    pub price: Option<Price>,
    pub size: Size,
    pub filled_size: Size,
    pub status: OrderStatus,
    pub timestamp: Timestamp,
}

/// Execution report
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionReport {
    pub order_id: OrderId,
    pub client_order_id: Option<String>,
    pub symbol: Symbol,
    pub exchange_id: ExchangeId,
    pub status: OrderStatus,
    pub filled_size: Size,
    pub remaining_size: Size,
    pub average_price: Option<Price>,
    pub timestamp: Timestamp,
}

/// Balance
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Balance {
    pub asset: String,
    pub exchange_id: ExchangeId,
    pub total: rust_decimal::Decimal,
    pub free: rust_decimal::Decimal,
    pub used: rust_decimal::Decimal,
}

impl Balance {
    pub fn new(asset: String, total: Size, used: Size) -> Self {
        let total_dec = total.value();
        let used_dec = used.value();
        Self {
            asset,
            exchange_id: "default".to_string(),
            total: total_dec,
            free: total_dec - used_dec,
            used: used_dec,
        }
    }
}

/// Trading fees
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradingFees {
    pub symbol: String,
    pub maker_fee: rust_decimal::Decimal,
    pub taker_fee: rust_decimal::Decimal,
}

impl TradingFees {
    pub fn new(symbol: String, maker_fee: Size, taker_fee: Size) -> Self {
        Self {
            symbol,
            maker_fee: maker_fee.value(),
            taker_fee: taker_fee.value(),
        }
    }
}

/// Market event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketEvent {
    OrderBookSnapshot(OrderBookSnapshot),
    OrderBookDelta(OrderBookDelta),
    Trade(Trade),
}

/// Trading event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradingEvent {
    OrderCreated(NewOrder),
    OrderUpdated(Order),
    ExecutionReport(ExecutionReport),
}

/// System event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemEvent {
    ExchangeConnected(ExchangeId),
    ExchangeDisconnected(ExchangeId),
    Error(String),
}

/// Signal generated by strategy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Signal {
    PlaceOrder {
        order: NewOrder,
    },
    CancelOrder {
        order_id: OrderId,
        symbol: Symbol,
        exchange_id: ExchangeId,
    },
    CancelAllOrders {
        symbol: Symbol,
        exchange_id: ExchangeId,
    },
    UpdateOrder {
        order_id: OrderId,
        price: Option<Price>,
        size: Option<Size>,
    },
}

/// Risk violation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskViolation {
    pub rule: String,
    pub details: String,
    pub timestamp: Timestamp,
}

impl RiskViolation {
    /// Create a new risk violation
    pub fn new(rule: String, details: String) -> Self {
        Self {
            rule,
            details,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        }
    }
}

/// Position
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub symbol: Symbol,
    pub exchange_id: ExchangeId,
    pub size: Size,
    pub average_price: Option<Price>,
    pub unrealized_pnl: Option<rust_decimal::Decimal>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Price, Size};

    #[test]
    fn test_order_book_level() {
        let price = Price::new(rust_decimal::Decimal::new(10000, 2)); // 100.00
        let size = Size::new(rust_decimal::Decimal::new(1500, 2)); // 15.00

        let level = OrderBookLevel { price, size };
        assert_eq!(level.price, price);
        assert_eq!(level.size, size);
    }

    #[test]
    fn test_order_book_snapshot() {
        let symbol = Symbol::new("BTCUSDT");
        let exchange_id = "binance".to_string();
        let timestamp = 1638368000000; // Example timestamp

        let snapshot = OrderBookSnapshot {
            symbol: symbol.clone(),
            exchange_id: exchange_id.clone(),
            bids: vec![],
            asks: vec![],
            timestamp,
        };

        assert_eq!(snapshot.symbol, symbol);
        assert_eq!(snapshot.exchange_id, exchange_id);
        assert_eq!(snapshot.timestamp, timestamp);
    }

    #[test]
    fn test_new_order() {
        let symbol = Symbol::new("BTCUSDT");
        let exchange_id = "binance".to_string();
        let price = Price::new(rust_decimal::Decimal::new(10000, 2)); // 100.00
        let size = Size::new(rust_decimal::Decimal::new(100, 2)); // 1.00

        let order = NewOrder {
            symbol: symbol.clone(),
            exchange_id: exchange_id.clone(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::GoodTillCancelled,
            price: Some(price),
            size,
            client_order_id: Some("client-123".to_string()),
        };

        assert_eq!(order.symbol, symbol);
        assert_eq!(order.exchange_id, exchange_id);
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.price, Some(price));
    }

    #[test]
    fn test_market_event() {
        let symbol = Symbol::new("BTCUSDT");
        let exchange_id = "binance".to_string();
        let timestamp = 1638368000000;

        let trade = Trade {
            symbol: symbol.clone(),
            exchange_id: exchange_id.clone(),
            price: Price::new(rust_decimal::Decimal::new(10000, 2)), // 100.00
            size: Size::new(rust_decimal::Decimal::new(100, 2)),     // 1.00
            side: OrderSide::Buy,
            timestamp,
            trade_id: Some("trade-123".to_string()),
        };

        let event = MarketEvent::Trade(trade);
        match event {
            MarketEvent::Trade(t) => {
                assert_eq!(t.symbol, symbol);
                assert_eq!(t.exchange_id, exchange_id);
            }
            _ => panic!("Expected trade event"),
        }
    }

    #[test]
    fn test_signal() {
        let symbol = Symbol::new("BTCUSDT");
        let exchange_id = "binance".to_string();
        let price = Price::new(rust_decimal::Decimal::new(10000, 2)); // 100.00
        let size = Size::new(rust_decimal::Decimal::new(100, 2)); // 1.00

        let order = NewOrder {
            symbol: symbol.clone(),
            exchange_id: exchange_id.clone(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::GoodTillCancelled,
            price: Some(price),
            size,
            client_order_id: Some("client-123".to_string()),
        };

        let signal = Signal::PlaceOrder { order };
        match signal {
            Signal::PlaceOrder { order: o } => {
                assert_eq!(o.symbol, symbol);
                assert_eq!(o.exchange_id, exchange_id);
            }
            _ => panic!("Expected place order signal"),
        }
    }
}
