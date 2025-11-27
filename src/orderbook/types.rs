use crate::types::{Price, Size};
use serde::{Deserialize, Serialize};

/// Represents a single level in the order book
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

/// Represents a delta update to the order book
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderBookDelta {
    pub symbol: String,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub timestamp: u64,
}

impl OrderBookDelta {
    pub fn new(symbol: String, bids: Vec<OrderBookLevel>, asks: Vec<OrderBookLevel>, timestamp: u64) -> Self {
        Self {
            symbol,
            bids,
            asks,
            timestamp,
        }
    }
}

/// Represents a full snapshot of the order book
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderBookSnapshot {
    pub symbol: String,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub timestamp: u64,
}

impl OrderBookSnapshot {
    pub fn new(symbol: String, bids: Vec<OrderBookLevel>, asks: Vec<OrderBookLevel>, timestamp: u64) -> Self {
        Self {
            symbol,
            bids,
            asks,
            timestamp,
        }
    }
}
