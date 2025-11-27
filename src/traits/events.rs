use crate::orderbook::{OrderBookSnapshot, OrderBookDelta};
use crate::types::{Price, Size};
use serde::{Deserialize, Serialize};

/// Unique identifier for an order
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderId(String);

impl OrderId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for OrderId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

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
}

/// Order time in force
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    GTC, // Good Till Cancel
    IOC, // Immediate Or Cancel
    FOK, // Fill Or Kill
}

/// New order request
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewOrder {
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub time_in_force: TimeInForce,
    pub quantity: Size,
    pub price: Option<Price>, // None for market orders
    pub client_order_id: Option<String>,
}

impl NewOrder {
    pub fn new_market_buy(symbol: String, quantity: Size) -> Self {
        Self {
            symbol,
            side: OrderSide::Buy,
            order_type: OrderType::Market,
            time_in_force: TimeInForce::IOC,
            quantity,
            price: None,
            client_order_id: None,
        }
    }

    pub fn new_market_sell(symbol: String, quantity: Size) -> Self {
        Self {
            symbol,
            side: OrderSide::Sell,
            order_type: OrderType::Market,
            time_in_force: TimeInForce::IOC,
            quantity,
            price: None,
            client_order_id: None,
        }
    }

    pub fn new_limit_buy(
        symbol: String,
        quantity: Size,
        price: Price,
        time_in_force: TimeInForce,
    ) -> Self {
        Self {
            symbol,
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            time_in_force,
            quantity,
            price: Some(price),
            client_order_id: None,
        }
    }

    pub fn new_limit_sell(
        symbol: String,
        quantity: Size,
        price: Price,
        time_in_force: TimeInForce,
    ) -> Self {
        Self {
            symbol,
            side: OrderSide::Sell,
            order_type: OrderType::Limit,
            time_in_force,
            quantity,
            price: Some(price),
            client_order_id: None,
        }
    }

    pub fn with_client_order_id(mut self, client_order_id: String) -> Self {
        self.client_order_id = Some(client_order_id);
        self
    }
}

/// Market event types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarketEvent {
    OrderBookSnapshot(OrderBookSnapshot),
    OrderBookDelta(OrderBookDelta),
    Trade {
        symbol: String,
        price: Price,
        size: Size,
        timestamp: u64,
        is_buyer_maker: bool,
    },
}

/// Order execution status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderStatus {
    New,
    PartiallyFilled {
        filled_size: Size,
        remaining_size: Size,
    },
    Filled {
        filled_size: Size,
    },
    Canceled {
        remaining_size: Size,
    },
    Rejected {
        reason: String,
    },
}

/// Order execution report
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionReport {
    pub order_id: OrderId,
    pub client_order_id: Option<String>,
    pub symbol: String,
    pub status: OrderStatus,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub time_in_force: TimeInForce,
    pub quantity: Size,
    pub price: Option<Price>,
    pub timestamp: u64,
}
