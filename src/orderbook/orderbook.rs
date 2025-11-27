use crate::types::{Price, Size};
use crate::orderbook::types::{OrderBookLevel, OrderBookDelta, OrderBookSnapshot};
use std::collections::BTreeMap;
use smallvec::SmallVec;

/// OrderBook implementation using BTreeMap for efficient price level management
/// Bids are stored in descending order (highest price first)
/// Asks are stored in ascending order (lowest price first)
#[derive(Debug, Clone)]
pub struct OrderBook {
    symbol: String,
    bids: BTreeMap<Price, Size>, // Descending order (reverse comparator)
    asks: BTreeMap<Price, Size>, // Ascending order
    last_update: u64,
}

impl OrderBook {
    /// Create a new empty OrderBook for the given symbol
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            last_update: 0,
        }
    }

    /// Get the symbol of this OrderBook
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// Get the best bid (highest price) if available
    pub fn best_bid(&self) -> Option<(Price, Size)> {
        self.bids.iter().next_back().map(|(price, size)| (*price, *size))
    }

    /// Get the best ask (lowest price) if available
    pub fn best_ask(&self) -> Option<(Price, Size)> {
        self.asks.iter().next().map(|(price, size)| (*price, *size))
    }

    /// Get the spread (best_ask - best_bid) if both sides are available
    pub fn spread(&self) -> Option<Price> {
        if let (Some((bid_price, _)), Some((ask_price, _))) = (self.best_bid(), self.best_ask()) {
            Some(ask_price - bid_price)
        } else {
            None
        }
    }

    /// Get the top N bid levels
    /// Uses SmallVec for stack allocation when N is small (common case in HFT)
    pub fn top_bids(&self, n: usize) -> SmallVec<[(Price, Size); 20]> {
        let mut result = SmallVec::new();
        
        for (price, size) in self.bids.iter().rev().take(n) {
            result.push((*price, *size));
        }
        
        result
    }

    /// Get the top N ask levels
    /// Uses SmallVec for stack allocation when N is small (common case in HFT)
    pub fn top_asks(&self, n: usize) -> SmallVec<[(Price, Size); 20]> {
        let mut result = SmallVec::new();
        
        for (price, size) in self.asks.iter().take(n) {
            result.push((*price, *size));
        }
        
        result
    }

    /// Apply a full snapshot to the order book
    /// This replaces the entire order book with the snapshot data
    pub fn apply_snapshot(&mut self, snapshot: OrderBookSnapshot) {
        // Clear existing data
        self.bids.clear();
        self.asks.clear();

        // Apply bids
        for level in snapshot.bids {
            if !level.size.is_zero() {
                self.bids.insert(level.price, level.size);
            }
        }

        // Apply asks
        for level in snapshot.asks {
            if !level.size.is_zero() {
                self.asks.insert(level.price, level.size);
            }
        }

        self.last_update = snapshot.timestamp;
    }

    /// Apply a delta update to the order book
    /// This updates specific price levels
    pub fn apply_delta(&mut self, delta: OrderBookDelta) {
        // Update bids
        for level in delta.bids {
            if level.size.is_zero() {
                // Remove the price level if size is zero
                self.bids.remove(&level.price);
            } else {
                // Update or insert the price level
                self.bids.insert(level.price, level.size);
            }
        }

        // Update asks
        for level in delta.asks {
            if level.size.is_zero() {
                // Remove the price level if size is zero
                self.asks.remove(&level.price);
            } else {
                // Update or insert the price level
                self.asks.insert(level.price, level.size);
            }
        }

        self.last_update = delta.timestamp;
    }

    /// Get the last update timestamp
    pub fn last_update(&self) -> u64 {
        self.last_update
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Price, Size};

    fn create_price(value: &str) -> Price {
        Price::from_str(value).unwrap()
    }

    fn create_size(value: &str) -> Size {
        Size::from_str(value).unwrap()
    }

    #[test]
    fn test_empty_orderbook() {
        let book = OrderBook::new("BTCUSDT".to_string());
        assert_eq!(book.symbol(), "BTCUSDT");
        assert!(book.best_bid().is_none());
        assert!(book.best_ask().is_none());
        assert!(book.spread().is_none());
    }

    #[test]
    fn test_apply_snapshot() {
        let mut book = OrderBook::new("BTCUSDT".to_string());
        
        let snapshot = OrderBookSnapshot::new(
            "BTCUSDT".to_string(),
            vec![
                OrderBookLevel::new(create_price("100.00"), create_size("10.0")),
                OrderBookLevel::new(create_price("99.50"), create_size("5.0")),
            ],
            vec![
                OrderBookLevel::new(create_price("100.50"), create_size("8.0")),
                OrderBookLevel::new(create_price("101.00"), create_size("12.0")),
            ],
            123456789,
        );

        book.apply_snapshot(snapshot);

        // Check best bid and ask
        assert_eq!(book.best_bid(), Some((create_price("100.00"), create_size("10.0"))));
        assert_eq!(book.best_ask(), Some((create_price("100.50"), create_size("8.0"))));
        assert_eq!(book.spread(), Some(create_price("0.50")));
        assert_eq!(book.last_update(), 123456789);

        // Check top levels
        let top_bids = book.top_bids(2);
        assert_eq!(top_bids.len(), 2);
        assert_eq!(top_bids[0], (create_price("100.00"), create_size("10.0")));
        assert_eq!(top_bids[1], (create_price("99.50"), create_size("5.0")));

        let top_asks = book.top_asks(2);
        assert_eq!(top_asks.len(), 2);
        assert_eq!(top_asks[0], (create_price("100.50"), create_size("8.0")));
        assert_eq!(top_asks[1], (create_price("101.00"), create_size("12.0")));
    }

    #[test]
    fn test_apply_delta() {
        let mut book = OrderBook::new("BTCUSDT".to_string());
        
        // First apply a snapshot
        let snapshot = OrderBookSnapshot::new(
            "BTCUSDT".to_string(),
            vec![OrderBookLevel::new(create_price("100.00"), create_size("10.0"))],
            vec![OrderBookLevel::new(create_price("101.00"), create_size("8.0"))],
            123456789,
        );
        book.apply_snapshot(snapshot);

        // Apply delta updates
        let delta = OrderBookDelta::new(
            "BTCUSDT".to_string(),
            vec![
                // Update existing bid
                OrderBookLevel::new(create_price("100.00"), create_size("15.0")),
                // Add new bid
                OrderBookLevel::new(create_price("100.50"), create_size("5.0")),
                // Remove bid (size = 0)
                OrderBookLevel::new(create_price("99.50"), create_size("0.0")),
            ],
            vec![
                // Update existing ask
                OrderBookLevel::new(create_price("101.00"), create_size("10.0")),
                // Remove ask (size = 0)
                OrderBookLevel::new(create_price("102.00"), create_size("0.0")),
            ],
            123456790,
        );
        book.apply_delta(delta);

        // Check updates
        assert_eq!(book.best_bid(), Some((create_price("100.50"), create_size("5.0"))));
        assert_eq!(book.best_ask(), Some((create_price("101.00"), create_size("10.0"))));
        assert_eq!(book.last_update(), 123456790);

        // Check that the removed levels are gone
        let top_bids = book.top_bids(10);
        assert!(!top_bids.iter().any(|(price, _)| *price == create_price("99.50")));

        let top_asks = book.top_asks(10);
        assert!(!top_asks.iter().any(|(price, _)| *price == create_price("102.00")));
    }

    #[test]
    fn test_smallvec_optimization() {
        let mut book = OrderBook::new("BTCUSDT".to_string());
        
        // Create a snapshot with many levels
        let mut bids = Vec::new();
        let mut asks = Vec::new();
        
        for i in 0..30 {
            bids.push(OrderBookLevel::new(
                create_price(&format!("{}.{}", 100 - i, 50 - i)),
                create_size(&format!("{}", i + 1))
            ));
            
            asks.push(OrderBookLevel::new(
                create_price(&format!("{}.{}", 101 + i, 50 + i)),
                create_size(&format!("{}", i + 1))
            ));
        }
        
        let snapshot = OrderBookSnapshot::new(
            "BTCUSDT".to_string(),
            bids,
            asks,
            123456789,
        );
        
        book.apply_snapshot(snapshot);
        
        // Test that top_bids and top_asks return SmallVec
        let top_bids = book.top_bids(5);
        let top_asks = book.top_asks(5);
        
        assert_eq!(top_bids.len(), 5);
        assert_eq!(top_asks.len(), 5);
        
        // Verify the order is correct
        assert!(top_bids[0].0 > top_bids[1].0); // Bids should be descending
        assert!(top_asks[0].0 < top_asks[1].0); // Asks should be ascending
    }
}
