use crate::orderbook::OrderBook;
use crate::types::{Price, Size};
use rust_decimal::prelude::*;
use std::collections::VecDeque;

/// Moving average calculator for order book data
pub struct OrderBookMovingAverage {
    /// Window size for the moving average
    window_size: usize,
    /// History of mid prices
    price_history: VecDeque<Price>,
    /// Current average value
    current_average: Option<Price>,
}

impl OrderBookMovingAverage {
    /// Create a new moving average with the specified window size
    pub fn new(window_size: usize) -> Self {
        Self {
            window_size,
            price_history: VecDeque::with_capacity(window_size),
            current_average: None,
        }
    }

    /// Update the moving average with a new order book
    pub fn update(&mut self, order_book: &OrderBook) -> Option<Price> {
        // Calculate mid price
        let mid_price = match (order_book.best_bid(), order_book.best_ask()) {
            (Some((bid_price, _)), Some((ask_price, _))) => {
                // Price - Price returns Price, Price / Price returns Decimal
                let spread = ask_price - bid_price;
                let half_spread = Price::new(spread.value() / Decimal::new(2, 0));
                Some(bid_price + half_spread)
            }
            _ => None,
        };

        if let Some(price) = mid_price {
            // Add new price to history
            self.price_history.push_back(price);

            // Remove oldest price if window is full
            if self.price_history.len() > self.window_size {
                self.price_history.pop_front();
            }

            // Calculate new average
            if !self.price_history.is_empty() {
                let sum = self
                    .price_history
                    .iter()
                    .fold(Price::new(Decimal::ZERO), |acc, &p| acc + p);
                // Price / Decimal returns Price
                self.current_average =
                    Some(sum / Decimal::from_usize(self.price_history.len()).unwrap());
            }
        }

        self.current_average
    }

    /// Get the current moving average value
    pub fn current(&self) -> Option<Price> {
        self.current_average
    }

    /// Get the window size
    pub fn window_size(&self) -> usize {
        self.window_size
    }
}

/// Order book imbalance indicator
pub struct OrderBookImbalance {
    /// Number of levels to consider for imbalance calculation
    levels: usize,
}

impl OrderBookImbalance {
    /// Create a new order book imbalance indicator
    pub fn new(levels: usize) -> Self {
        Self { levels }
    }

    /// Calculate the order book imbalance
    /// Returns a value between -1.0 (all asks) and 1.0 (all bids)
    pub fn calculate(&self, order_book: &OrderBook) -> Option<f64> {
        if self.levels == 0 {
            return None;
        }

        // Get top bid levels
        let bids = order_book.top_bids(self.levels);
        // Get top ask levels
        let asks = order_book.top_asks(self.levels);

        if bids.is_empty() || asks.is_empty() {
            return None;
        }

        // Calculate total bid volume
        let total_bid_volume = bids
            .iter()
            .fold(Size::new(rust_decimal::Decimal::ZERO), |acc, (_, size)| {
                acc + *size
            });

        // Calculate total ask volume
        let total_ask_volume = asks
            .iter()
            .fold(Size::new(rust_decimal::Decimal::ZERO), |acc, (_, size)| {
                acc + *size
            });

        // Calculate total volume
        let total_volume = total_bid_volume + total_ask_volume;

        if total_volume.is_zero() {
            return Some(0.0);
        }

        // Calculate imbalance: (bid_volume - ask_volume) / total_volume
        let bid_volume_ratio = total_bid_volume.value() / total_volume.value();
        let ask_volume_ratio = total_ask_volume.value() / total_volume.value();

        Some(bid_volume_ratio.to_f64().unwrap_or(0.0) - ask_volume_ratio.to_f64().unwrap_or(0.0))
    }

    /// Get the number of levels considered
    pub fn levels(&self) -> usize {
        self.levels
    }
}

/// Order book spread indicator
pub struct OrderBookSpread {
    /// History of spread values
    spread_history: VecDeque<Price>,
    /// Window size for spread statistics
    window_size: usize,
    /// Current spread
    current_spread: Option<Price>,
    /// Minimum spread in the window
    min_spread: Option<Price>,
    /// Maximum spread in the window
    max_spread: Option<Price>,
    /// Average spread in the window
    avg_spread: Option<Price>,
}

impl OrderBookSpread {
    /// Create a new spread indicator with the specified window size
    pub fn new(window_size: usize) -> Self {
        Self {
            spread_history: VecDeque::with_capacity(window_size),
            window_size,
            current_spread: None,
            min_spread: None,
            max_spread: None,
            avg_spread: None,
        }
    }

    /// Update the spread indicator with a new order book
    pub fn update(&mut self, order_book: &OrderBook) -> Option<Price> {
        // Calculate current spread
        self.current_spread = order_book.spread();

        if let Some(spread) = self.current_spread {
            // Add new spread to history
            self.spread_history.push_back(spread);

            // Remove oldest spread if window is full
            if self.spread_history.len() > self.window_size {
                self.spread_history.pop_front();
            }

            // Calculate statistics
            if !self.spread_history.is_empty() {
                // Calculate min and max
                self.min_spread = self.spread_history.iter().min().copied();
                self.max_spread = self.spread_history.iter().max().copied();

                // Calculate average
                let sum = self
                    .spread_history
                    .iter()
                    .fold(Price::new(Decimal::ZERO), |acc, &p| acc + p);
                // Price / Decimal returns Price
                self.avg_spread =
                    Some(sum / Decimal::from_usize(self.spread_history.len()).unwrap());
            }
        }

        self.current_spread
    }

    /// Get the current spread
    pub fn current(&self) -> Option<Price> {
        self.current_spread
    }

    /// Get the minimum spread in the window
    pub fn min(&self) -> Option<Price> {
        self.min_spread
    }

    /// Get the maximum spread in the window
    pub fn max(&self) -> Option<Price> {
        self.max_spread
    }

    /// Get the average spread in the window
    pub fn average(&self) -> Option<Price> {
        self.avg_spread
    }

    /// Get the window size
    pub fn window_size(&self) -> usize {
        self.window_size
    }
}

/// Order book depth indicator
pub struct OrderBookDepth {
    /// Number of levels to consider for depth calculation
    levels: usize,
}

impl OrderBookDepth {
    /// Create a new order book depth indicator
    pub fn new(levels: usize) -> Self {
        Self { levels }
    }

    /// Calculate the bid depth (total volume at bid levels)
    pub fn bid_depth(&self, order_book: &OrderBook) -> Size {
        let bids = order_book.top_bids(self.levels);
        bids.iter()
            .fold(Size::new(rust_decimal::Decimal::ZERO), |acc, (_, size)| {
                acc + *size
            })
    }

    /// Calculate the ask depth (total volume at ask levels)
    pub fn ask_depth(&self, order_book: &OrderBook) -> Size {
        let asks = order_book.top_asks(self.levels);
        asks.iter()
            .fold(Size::new(rust_decimal::Decimal::ZERO), |acc, (_, size)| {
                acc + *size
            })
    }

    /// Calculate the total depth (bid + ask)
    pub fn total_depth(&self, order_book: &OrderBook) -> Size {
        self.bid_depth(order_book) + self.ask_depth(order_book)
    }

    /// Calculate the depth ratio (bid_depth / total_depth)
    pub fn depth_ratio(&self, order_book: &OrderBook) -> Option<f64> {
        let total = self.total_depth(order_book);
        if total.is_zero() {
            return Some(0.5); // Balanced when no depth
        }

        let bid = self.bid_depth(order_book);
        Some((bid.value() / total.value()).to_f64().unwrap_or(0.5))
    }

    /// Get the number of levels considered
    pub fn levels(&self) -> usize {
        self.levels
    }
}

/// Order book volatility indicator
pub struct OrderBookVolatility {
    /// History of mid prices
    price_history: VecDeque<Price>,
    /// Window size for volatility calculation
    window_size: usize,
    /// Current volatility
    current_volatility: Option<Price>,
}

impl OrderBookVolatility {
    /// Create a new volatility indicator with the specified window size
    pub fn new(window_size: usize) -> Self {
        Self {
            price_history: VecDeque::with_capacity(window_size),
            window_size,
            current_volatility: None,
        }
    }

    /// Update the volatility indicator with a new order book
    pub fn update(&mut self, order_book: &OrderBook) -> Option<Price> {
        // Calculate mid price
        let mid_price = match (order_book.best_bid(), order_book.best_ask()) {
            (Some((bid_price, _)), Some((ask_price, _))) => {
                // Price - Price returns Price, need to use .value() for division
                let spread = ask_price - bid_price;
                let half_spread = Price::new(spread.value() / Decimal::new(2, 0));
                Some(bid_price + half_spread)
            }
            _ => None,
        };

        if let Some(price) = mid_price {
            // Add new price to history
            self.price_history.push_back(price);

            // Remove oldest price if window is full
            if self.price_history.len() > self.window_size {
                self.price_history.pop_front();
            }

            // Calculate volatility if we have enough data
            if self.price_history.len() >= 2 {
                // Calculate mean
                let sum = self
                    .price_history
                    .iter()
                    .fold(Price::new(Decimal::ZERO), |acc, &p| acc + p);
                // Price / Decimal returns Price
                let mean = sum / Decimal::from_usize(self.price_history.len()).unwrap();

                // Calculate variance
                let variance = self.price_history.iter().fold(Decimal::ZERO, |acc, &p| {
                    let diff = p.value() - mean.value();
                    acc + diff * diff
                }) / Decimal::from_usize(self.price_history.len()).unwrap();

                // Volatility is the square root of variance (convert to f64 for sqrt)
                let volatility = variance.to_f64().map(|v| v.sqrt()).unwrap_or(0.0);
                self.current_volatility = Some(Price::new(
                    Decimal::from_f64(volatility).unwrap_or(Decimal::ZERO),
                ));
            }
        }

        self.current_volatility
    }

    /// Get the current volatility
    pub fn current(&self) -> Option<Price> {
        self.current_volatility
    }

    /// Get the window size
    pub fn window_size(&self) -> usize {
        self.window_size
    }
}

/// Combined order book indicators
pub struct OrderBookIndicators {
    /// Moving average of mid prices
    moving_average: OrderBookMovingAverage,
    /// Order book imbalance
    imbalance: OrderBookImbalance,
    /// Order book spread
    spread: OrderBookSpread,
    /// Order book depth
    depth: OrderBookDepth,
    /// Order book volatility
    volatility: OrderBookVolatility,
}

impl OrderBookIndicators {
    /// Create a new set of order book indicators
    pub fn new(
        ma_window: usize,
        imbalance_levels: usize,
        spread_window: usize,
        depth_levels: usize,
        volatility_window: usize,
    ) -> Self {
        Self {
            moving_average: OrderBookMovingAverage::new(ma_window),
            imbalance: OrderBookImbalance::new(imbalance_levels),
            spread: OrderBookSpread::new(spread_window),
            depth: OrderBookDepth::new(depth_levels),
            volatility: OrderBookVolatility::new(volatility_window),
        }
    }

    /// Update all indicators with a new order book
    pub fn update(&mut self, order_book: &OrderBook) {
        self.moving_average.update(order_book);
        self.spread.update(order_book);
        self.volatility.update(order_book);
    }

    /// Get the current moving average
    pub fn moving_average(&self) -> Option<Price> {
        self.moving_average.current()
    }

    /// Get the current order book imbalance
    pub fn imbalance(&self, order_book: &OrderBook) -> Option<f64> {
        self.imbalance.calculate(order_book)
    }

    /// Get the current spread
    pub fn spread(&self) -> Option<Price> {
        self.spread.current()
    }

    /// Get the minimum spread in the window
    pub fn min_spread(&self) -> Option<Price> {
        self.spread.min()
    }

    /// Get the maximum spread in the window
    pub fn max_spread(&self) -> Option<Price> {
        self.spread.max()
    }

    /// Get the average spread in the window
    pub fn avg_spread(&self) -> Option<Price> {
        self.spread.average()
    }

    /// Get the bid depth
    pub fn bid_depth(&self, order_book: &OrderBook) -> Size {
        self.depth.bid_depth(order_book)
    }

    /// Get the ask depth
    pub fn ask_depth(&self, order_book: &OrderBook) -> Size {
        self.depth.ask_depth(order_book)
    }

    /// Get the total depth
    pub fn total_depth(&self, order_book: &OrderBook) -> Size {
        self.depth.total_depth(order_book)
    }

    /// Get the depth ratio
    pub fn depth_ratio(&self, order_book: &OrderBook) -> Option<f64> {
        self.depth.depth_ratio(order_book)
    }

    /// Get the current volatility
    pub fn volatility(&self) -> Option<Price> {
        self.volatility.current()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::OrderBookLevel;
    use crate::orderbook::OrderBookSnapshot;
    use crate::types::Symbol;

    fn create_test_orderbook() -> OrderBook {
        let mut book = OrderBook::new("BTCUSDT".to_string());

        let snapshot = OrderBookSnapshot::new(
            Symbol::new("BTCUSDT"),
            "test_exchange".to_string(),
            vec![
                OrderBookLevel::new(
                    Price::from_str("100.00").unwrap(),
                    Size::from_str("10.0").unwrap(),
                ),
                OrderBookLevel::new(
                    Price::from_str("99.50").unwrap(),
                    Size::from_str("5.0").unwrap(),
                ),
                OrderBookLevel::new(
                    Price::from_str("99.00").unwrap(),
                    Size::from_str("8.0").unwrap(),
                ),
            ],
            vec![
                OrderBookLevel::new(
                    Price::from_str("101.00").unwrap(),
                    Size::from_str("10.0").unwrap(),
                ),
                OrderBookLevel::new(
                    Price::from_str("101.50").unwrap(),
                    Size::from_str("5.0").unwrap(),
                ),
                OrderBookLevel::new(
                    Price::from_str("102.00").unwrap(),
                    Size::from_str("8.0").unwrap(),
                ),
            ],
            123456789,
        );

        book.apply_snapshot(snapshot);
        book
    }

    #[test]
    fn test_moving_average() {
        let mut ma = OrderBookMovingAverage::new(3);
        let book = create_test_orderbook();

        // First update
        let avg1 = ma.update(&book);
        assert!(avg1.is_some());
        assert_eq!(avg1.unwrap(), Price::from_str("100.50").unwrap()); // Mid price

        // Second update
        let avg2 = ma.update(&book);
        assert!(avg2.is_some());

        // Third update
        let avg3 = ma.update(&book);
        assert!(avg3.is_some());

        // Fourth update (should replace oldest)
        let avg4 = ma.update(&book);
        assert!(avg4.is_some());
    }

    #[test]
    fn test_order_book_imbalance() {
        let imbalance = OrderBookImbalance::new(2);
        let book = create_test_orderbook();

        let imbalance_value = imbalance.calculate(&book);
        assert!(imbalance_value.is_some());

        // With equal volumes, imbalance should be close to 0
        assert!(imbalance_value.unwrap().abs() < 0.1);
    }

    #[test]
    fn test_order_book_spread() {
        let mut spread = OrderBookSpread::new(5);
        let book = create_test_orderbook();

        let current_spread = spread.update(&book);
        assert!(current_spread.is_some());
        assert_eq!(current_spread.unwrap(), Price::from_str("1.00").unwrap()); // 101.00 - 100.00

        assert_eq!(spread.current(), Some(Price::from_str("1.00").unwrap()));
        assert_eq!(spread.min(), Some(Price::from_str("1.00").unwrap()));
        assert_eq!(spread.max(), Some(Price::from_str("1.00").unwrap()));
        assert_eq!(spread.average(), Some(Price::from_str("1.00").unwrap()));
    }

    #[test]
    fn test_order_book_depth() {
        let depth = OrderBookDepth::new(2);
        let book = create_test_orderbook();

        let bid_depth = depth.bid_depth(&book);
        let ask_depth = depth.ask_depth(&book);
        let total_depth = depth.total_depth(&book);
        let depth_ratio = depth.depth_ratio(&book);

        assert_eq!(bid_depth, Size::from_str("15.0").unwrap()); // 10.0 + 5.0
        assert_eq!(ask_depth, Size::from_str("15.0").unwrap()); // 10.0 + 5.0
        assert_eq!(total_depth, Size::from_str("30.0").unwrap()); // 15.0 + 15.0
        assert_eq!(depth_ratio, Some(0.5)); // Equal bid and ask depth
    }

    #[test]
    fn test_order_book_volatility() {
        let mut volatility = OrderBookVolatility::new(3);
        let book = create_test_orderbook();

        // First update
        let vol1 = volatility.update(&book);
        assert!(vol1.is_none()); // Need at least 2 data points

        // Second update
        let vol2 = volatility.update(&book);
        assert!(vol2.is_some());
    }

    #[test]
    fn test_combined_indicators() {
        let mut indicators = OrderBookIndicators::new(5, 3, 5, 3, 5);
        let book = create_test_orderbook();

        indicators.update(&book);

        assert!(indicators.moving_average().is_some());
        assert!(indicators.imbalance(&book).is_some());
        assert!(indicators.spread().is_some());
        assert_eq!(indicators.bid_depth(&book), Size::from_str("23.0").unwrap()); // 10.0 + 5.0 + 8.0
        assert_eq!(indicators.ask_depth(&book), Size::from_str("23.0").unwrap()); // 10.0 + 5.0 + 8.0
        assert_eq!(
            indicators.total_depth(&book),
            Size::from_str("46.0").unwrap()
        ); // 23.0 + 23.0
        assert_eq!(indicators.depth_ratio(&book), Some(0.5)); // Equal bid and ask depth
    }
}
