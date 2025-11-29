use crate::core::events::Trade;
use crate::indicators::trade_flow_indicators::{TradeFlowIndicator, TradeFlowMomentum};
use crate::strategies::prediction::LinearRegressionPredictor;
use crate::strategy::{MarketState, Signal, Strategy};
use crate::traits::{NewOrder, OrderSide, TimeInForce};
use crate::types::{Price, Size};
use rust_decimal::prelude::*;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Market making strategy that places bid and ask orders around the current market price
/// Can optionally use price prediction to improve order placement
pub struct MarketMakingStrategy {
    /// Target spread to maintain between bid and ask orders
    target_spread: Price,
    /// Base order size for each level
    base_order_size: Size,
    /// Maximum position size (total exposure)
    max_position_size: Size,
    /// Maximum number of order levels to place on each side
    max_order_levels: usize,
    /// Time to wait before refreshing orders
    order_refresh_time: Duration,
    /// Current positions by symbol
    positions: HashMap<String, Size>,
    /// Last order placement time for each symbol
    last_order_time: HashMap<String, Instant>,
    /// Current active orders by symbol
    active_orders: HashMap<String, Vec<NewOrder>>,
    /// Price predictors by symbol (optional)
    predictors: HashMap<String, LinearRegressionPredictor>,
    /// Trade flow indicators by symbol (optional)
    trade_flow_indicators: HashMap<String, TradeFlowIndicator>,
    /// Trade flow momentum indicators by symbol (optional)
    trade_flow_momentum: HashMap<String, TradeFlowMomentum>,
    /// Enable price prediction (default: false)
    enable_prediction: bool,
    /// Prediction horizon in seconds (how far ahead to predict)
    prediction_horizon_seconds: u64,
    /// Weight for prediction adjustment (0.0 to 1.0)
    prediction_weight: f64,
}

impl MarketMakingStrategy {
    /// Create a new market making strategy without prediction
    pub fn new(
        target_spread: Price,
        base_order_size: Size,
        max_position_size: Size,
        max_order_levels: usize,
        order_refresh_time: Duration,
    ) -> Self {
        Self {
            target_spread,
            base_order_size,
            max_position_size,
            max_order_levels,
            order_refresh_time,
            positions: HashMap::new(),
            last_order_time: HashMap::new(),
            active_orders: HashMap::new(),
            predictors: HashMap::new(),
            trade_flow_indicators: HashMap::new(),
            trade_flow_momentum: HashMap::new(),
            enable_prediction: false,
            prediction_horizon_seconds: 60,
            prediction_weight: 0.3,
        }
    }

    /// Create a new market making strategy with prediction enabled
    pub fn with_prediction(
        target_spread: Price,
        base_order_size: Size,
        max_position_size: Size,
        max_order_levels: usize,
        order_refresh_time: Duration,
        prediction_horizon_seconds: u64,
        prediction_weight: f64,
        _predictor_max_history: usize,
        _predictor_min_data_points: usize,
        _trade_flow_max_trades: usize,
        _trade_flow_time_window_ms: u64,
    ) -> Self {
        Self {
            target_spread,
            base_order_size,
            max_position_size,
            max_order_levels,
            order_refresh_time,
            positions: HashMap::new(),
            last_order_time: HashMap::new(),
            active_orders: HashMap::new(),
            predictors: HashMap::new(),
            trade_flow_indicators: HashMap::new(),
            trade_flow_momentum: HashMap::new(),
            enable_prediction: true,
            prediction_horizon_seconds,
            prediction_weight: prediction_weight.max(0.0).min(1.0),
        }
    }

    /// Enable or disable prediction
    pub fn set_prediction_enabled(&mut self, enabled: bool) {
        self.enable_prediction = enabled;
    }

    /// Update prediction with a new trade
    pub fn update_prediction(&mut self, trade: &Trade) {
        if !self.enable_prediction {
            return;
        }

        let symbol = trade.symbol.value().to_string();

        // Update predictor
        let predictor = self
            .predictors
            .entry(symbol.clone())
            .or_insert_with(|| LinearRegressionPredictor::new(100, 10));
        predictor.update_from_trade(trade);

        // Update trade flow indicator
        let flow_indicator = self
            .trade_flow_indicators
            .entry(symbol.clone())
            .or_insert_with(|| TradeFlowIndicator::new(100, 60000));
        flow_indicator.add_trade(trade.clone());

        // Update trade flow momentum
        let momentum = self
            .trade_flow_momentum
            .entry(symbol.clone())
            .or_insert_with(|| TradeFlowMomentum::new(100, 60000, 10));
        momentum.add_trade(trade.clone());
    }

    /// Get predicted price adjustment factor
    /// Returns a factor to adjust mid price based on prediction
    fn get_prediction_adjustment(&self, symbol: &str, current_mid_price: Price) -> Option<f64> {
        if !self.enable_prediction {
            return None;
        }

        let predictor = self.predictors.get(symbol)?;
        if !predictor.is_ready() {
            return None;
        }

        // Get predicted price
        let predicted_price = predictor.predict_after_seconds(self.prediction_horizon_seconds)?;

        // Calculate price change percentage
        let current_value = current_mid_price.value().to_f64()?;
        let predicted_value = predicted_price.value().to_f64()?;

        if current_value.abs() < 1e-10 {
            return None;
        }

        // Calculate adjustment factor: (predicted - current) / current
        let price_change_ratio = (predicted_value - current_value) / current_value;

        // Apply prediction weight
        Some(price_change_ratio * self.prediction_weight)
    }

    /// Get the target spread
    pub fn target_spread(&self) -> Price {
        self.target_spread
    }

    /// Get the base order size
    pub fn base_order_size(&self) -> Size {
        self.base_order_size
    }

    /// Get the maximum position size
    pub fn max_position_size(&self) -> Size {
        self.max_position_size
    }

    /// Get the maximum number of order levels
    pub fn max_order_levels(&self) -> usize {
        self.max_order_levels
    }

    /// Get the order refresh time
    pub fn order_refresh_time(&self) -> Duration {
        self.order_refresh_time
    }

    /// Get current position for a symbol
    pub fn get_position(&self, symbol: &str) -> Size {
        self.positions
            .get(symbol)
            .cloned()
            .unwrap_or(Size::new(rust_decimal::Decimal::ZERO))
    }

    /// Update position after a trade
    pub fn update_position(&mut self, symbol: &str, quantity_change: Size) {
        let current_position = self.get_position(symbol);
        let new_position = current_position + quantity_change;
        self.positions.insert(symbol.to_string(), new_position);
    }

    /// Check if we can place an order given current position
    /// T038: Position size comparison with negative max_position for short selling
    pub fn can_place_order(&self, symbol: &str, side: OrderSide, quantity: Size) -> bool {
        let current_position = self.get_position(symbol);

        match side {
            OrderSide::Buy => {
                // For buy orders, check if adding to position would exceed max
                let new_position = current_position + quantity;
                new_position <= self.max_position_size
            }
            OrderSide::Sell => {
                // For sell orders, check if we have enough position to sell
                // Allow short selling up to max_position_size
                let new_position = current_position - quantity;
                new_position >= -self.max_position_size
            }
        }
    }

    /// Calculate inventory skew adjustment factor
    /// Returns a value between 0.5 and 2.0 to adjust order sizes
    fn calculate_inventory_skew(&self, symbol: &str) -> f64 {
        let current_position = self.get_position(symbol);

        // Calculate position ratio (-1.0 to 1.0)
        let position_ratio = if self.max_position_size.is_zero() {
            0.0
        } else {
            let ratio = current_position.value() / self.max_position_size.value();
            // Clamp to [-1.0, 1.0]
            if ratio > rust_decimal::Decimal::ONE {
                1.0
            } else if ratio < -rust_decimal::Decimal::ONE {
                -1.0
            } else {
                ratio.to_f64().unwrap_or(0.0)
            }
        };

        // Convert position ratio to skew adjustment
        // Positive position (long) -> reduce buy orders, increase sell orders
        // Negative position (short) -> increase buy orders, reduce sell orders
        1.0 - position_ratio * 0.5
    }

    /// Calculate order prices based on current market
    fn calculate_order_prices(
        &self,
        best_bid: Price,
        best_ask: Price,
        inventory_skew: f64,
        symbol: &str,
    ) -> (Vec<Price>, Vec<Price>) {
        let mut mid_price = best_bid + (best_ask - best_bid) / Decimal::new(2, 0);

        // Apply prediction adjustment if enabled
        if let Some(adjustment_factor) = self.get_prediction_adjustment(symbol, mid_price) {
            let adjustment =
                mid_price.value() * Decimal::from_f64(adjustment_factor).unwrap_or(Decimal::ZERO);
            mid_price = mid_price + Price::new(adjustment);
        }

        // Calculate bid prices (below mid price)
        let mut bid_prices = Vec::new();
        for i in 0..self.max_order_levels {
            // Adjust spread based on inventory skew
            let spread_adjustment = if inventory_skew > 1.0 {
                // We're short, tighten bid prices (move closer to mid)
                self.target_spread * Decimal::from_f64(2.0 - inventory_skew).unwrap_or(Decimal::ONE)
            } else {
                // We're long, widen bid prices (move further from mid)
                self.target_spread * Decimal::from_f64(inventory_skew).unwrap_or(Decimal::ONE)
            };

            let price_offset =
                spread_adjustment * Decimal::from_usize(i + 1).unwrap_or(Decimal::ONE);
            let bid_price = mid_price - price_offset;
            bid_prices.push(bid_price);
        }

        // Calculate ask prices (above mid price)
        let mut ask_prices = Vec::new();
        for i in 0..self.max_order_levels {
            // Adjust spread based on inventory skew
            let spread_adjustment = if inventory_skew < 1.0 {
                // We're long, tighten ask prices (move closer to mid)
                self.target_spread * Decimal::from_f64(inventory_skew).unwrap_or(Decimal::ONE)
            } else {
                // We're short, widen ask prices (move further from mid)
                self.target_spread * Decimal::from_f64(2.0 - inventory_skew).unwrap_or(Decimal::ONE)
            };

            let price_offset =
                spread_adjustment * Decimal::from_usize(i + 1).unwrap_or(Decimal::ONE);
            let ask_price = mid_price + price_offset;
            ask_prices.push(ask_price);
        }

        (bid_prices, ask_prices)
    }

    /// Calculate order sizes based on inventory skew
    fn calculate_order_sizes(&self, inventory_skew: f64) -> (Vec<Size>, Vec<Size>) {
        let mut bid_sizes = Vec::new();
        let mut ask_sizes = Vec::new();

        for i in 0..self.max_order_levels {
            // Adjust size based on inventory skew and level
            let level_multiplier = 1.0 - (i as f64 * 0.1); // Decrease size for outer levels

            // For bid orders, adjust based on inventory skew
            let bid_size_multiplier = if inventory_skew < 1.0 {
                // We're long, reduce bid sizes
                inventory_skew * level_multiplier
            } else {
                // We're short or balanced, use normal or increased bid sizes
                level_multiplier
            };

            // For ask orders, adjust based on inventory skew
            let ask_size_multiplier = if inventory_skew > 1.0 {
                // We're short, reduce ask sizes
                (2.0 - inventory_skew) * level_multiplier
            } else {
                // We're long or balanced, use normal or increased ask sizes
                level_multiplier
            };

            let bid_size = self.base_order_size
                * Decimal::from_f64(bid_size_multiplier).unwrap_or(Decimal::ONE);
            let ask_size = self.base_order_size
                * Decimal::from_f64(ask_size_multiplier).unwrap_or(Decimal::ONE);

            bid_sizes.push(bid_size);
            ask_sizes.push(ask_size);
        }

        (bid_sizes, ask_sizes)
    }

    /// Check if enough time has passed since last order placement
    fn should_refresh_orders(&self, symbol: &str) -> bool {
        if let Some(last_time) = self.last_order_time.get(symbol) {
            last_time.elapsed() >= self.order_refresh_time
        } else {
            true // No previous order, should place
        }
    }

    /// Update the last order placement time
    fn update_last_order_time(&mut self, symbol: &str) {
        self.last_order_time
            .insert(symbol.to_string(), Instant::now());
    }

    /// Cancel all active orders for a symbol
    fn cancel_all_orders(&mut self, symbol: &str) -> Vec<Signal> {
        let mut signals = Vec::new();

        if self.active_orders.contains_key(symbol) {
            // Create a single cancel all orders signal
            signals.push(Signal::CancelAllOrders {
                symbol: symbol.to_string(),
                exchange_id: "binance".to_string(), // Default to binance
            });
        }

        // Clear active orders
        self.active_orders.remove(symbol);

        signals
    }
}

impl Strategy for MarketMakingStrategy {
    fn generate_signal(&mut self, market_state: &MarketState) -> Option<Signal> {
        let symbol = &market_state.symbol;

        // Check if we should refresh orders
        if !self.should_refresh_orders(symbol) {
            return None;
        }

        // Get best bid and ask
        let (best_bid_price, _best_bid_size) = market_state.best_bid()?;
        let (best_ask_price, _best_ask_size) = market_state.best_ask()?;

        // Calculate current spread
        let current_spread = best_ask_price - best_bid_price;

        // If spread is too small, don't place orders
        if current_spread < self.target_spread {
            return None;
        }

        // Calculate inventory skew
        let inventory_skew = self.calculate_inventory_skew(symbol);

        // Calculate order prices and sizes
        let (bid_prices, ask_prices) =
            self.calculate_order_prices(best_bid_price, best_ask_price, inventory_skew, symbol);

        let (bid_sizes, ask_sizes) = self.calculate_order_sizes(inventory_skew);

        // Cancel existing orders
        let cancel_signals = self.cancel_all_orders(symbol);

        // Create new orders
        let mut new_orders = Vec::new();
        let mut active_orders = Vec::new();

        // Add bid orders
        for (i, (price, size)) in bid_prices.iter().zip(bid_sizes.iter()).enumerate() {
            if self.can_place_order(symbol, OrderSide::Buy, *size) {
                let order = NewOrder::new_limit_buy(
                    symbol.clone(),
                    *size,
                    *price,
                    TimeInForce::GoodTillCancelled,
                )
                .with_client_order_id(format!("mm_bid_{}_{}", symbol, i));

                new_orders.push(Signal::PlaceOrder {
                    order: order.clone(),
                });
                active_orders.push(order);
            }
        }

        // Add ask orders
        for (i, (price, size)) in ask_prices.iter().zip(ask_sizes.iter()).enumerate() {
            if self.can_place_order(symbol, OrderSide::Sell, *size) {
                let order = NewOrder::new_limit_sell(
                    symbol.clone(),
                    *size,
                    *price,
                    TimeInForce::GoodTillCancelled,
                )
                .with_client_order_id(format!("mm_ask_{}_{}", symbol, i));

                new_orders.push(Signal::PlaceOrder {
                    order: order.clone(),
                });
                active_orders.push(order);
            }
        }

        // Update active orders
        self.active_orders.insert(symbol.to_string(), active_orders);

        // Update last order time
        self.update_last_order_time(symbol);

        // Return the first signal (in a real implementation, we'd return all signals)
        // For simplicity, we'll just return the first new order or cancel signal
        if let Some(signal) = cancel_signals
            .into_iter()
            .chain(new_orders.into_iter())
            .next()
        {
            Some(signal)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orderbook::{OrderBookLevel, OrderBookSnapshot};
    use crate::traits::MarketEvent;

    #[test]
    fn test_market_making_strategy_creation() {
        let strategy = MarketMakingStrategy::new(
            Price::from_str("0.5").unwrap(),
            Size::from_str("0.1").unwrap(),
            Size::from_str("1.0").unwrap(),
            5,
            Duration::from_millis(100),
        );

        assert_eq!(strategy.target_spread(), Price::from_str("0.5").unwrap());
        assert_eq!(strategy.base_order_size(), Size::from_str("0.1").unwrap());
        assert_eq!(strategy.max_position_size(), Size::from_str("1.0").unwrap());
        assert_eq!(strategy.max_order_levels(), 5);
        assert_eq!(strategy.order_refresh_time(), Duration::from_millis(100));
    }

    #[test]
    fn test_position_tracking() {
        let mut strategy = MarketMakingStrategy::new(
            Price::from_str("0.5").unwrap(),
            Size::from_str("0.1").unwrap(),
            Size::from_str("1.0").unwrap(),
            5,
            Duration::from_millis(100),
        );

        // Initial position should be zero
        assert_eq!(
            strategy.get_position("BTCUSDT"),
            Size::new(rust_decimal::Decimal::ZERO)
        );

        // Update position
        strategy.update_position("BTCUSDT", Size::from_str("0.5").unwrap());
        assert_eq!(
            strategy.get_position("BTCUSDT"),
            Size::from_str("0.5").unwrap()
        );

        // Update position again
        strategy.update_position("BTCUSDT", Size::from_str("-0.2").unwrap());
        assert_eq!(
            strategy.get_position("BTCUSDT"),
            Size::from_str("0.3").unwrap()
        );
    }

    #[test]
    fn test_inventory_skew_calculation() {
        let mut strategy = MarketMakingStrategy::new(
            Price::from_str("0.5").unwrap(),
            Size::from_str("0.1").unwrap(),
            Size::from_str("1.0").unwrap(),
            5,
            Duration::from_millis(100),
        );

        // No position should result in neutral skew
        let skew1 = strategy.calculate_inventory_skew("BTCUSDT");
        assert_eq!(skew1, 1.0);

        // Long position should result in skew < 1.0
        strategy.update_position("BTCUSDT", Size::from_str("0.5").unwrap());
        let skew2 = strategy.calculate_inventory_skew("BTCUSDT");
        assert!(skew2 < 1.0);

        // Short position should result in skew > 1.0
        strategy.update_position("BTCUSDT", Size::from_str("-1.5").unwrap());
        let skew3 = strategy.calculate_inventory_skew("BTCUSDT");
        assert!(skew3 > 1.0);
    }

    #[test]
    fn test_order_placement_limits() {
        let strategy = MarketMakingStrategy::new(
            Price::from_str("0.5").unwrap(),
            Size::from_str("0.5").unwrap(),
            Size::from_str("1.0").unwrap(),
            5,
            Duration::from_millis(100),
        );

        // Should be able to place buy order when under limit
        assert!(strategy.can_place_order(
            "BTCUSDT",
            OrderSide::Buy,
            Size::from_str("0.5").unwrap()
        ));

        // Should not be able to place buy order that exceeds limit
        assert!(!strategy.can_place_order(
            "BTCUSDT",
            OrderSide::Buy,
            Size::from_str("1.5").unwrap()
        ));

        // Should be able to place sell order when under limit
        assert!(strategy.can_place_order(
            "BTCUSDT",
            OrderSide::Sell,
            Size::from_str("0.5").unwrap()
        ));

        // Should not be able to place sell order that exceeds limit
        assert!(!strategy.can_place_order(
            "BTCUSDT",
            OrderSide::Sell,
            Size::from_str("1.5").unwrap()
        ));
    }

    #[test]
    fn test_signal_generation() {
        let mut strategy = MarketMakingStrategy::new(
            Price::from_str("0.5").unwrap(),
            Size::from_str("0.1").unwrap(),
            Size::from_str("1.0").unwrap(),
            2,
            Duration::from_millis(100),
        );

        // Create market state with a spread
        let mut market_state = MarketState::new("BTCUSDT".to_string());

        let snapshot = OrderBookSnapshot::new(
            "BTCUSDT".to_string(),
            "binance".to_string(),
            vec![OrderBookLevel::new(
                Price::from_str("100.00").unwrap(),
                Size::from_str("10.0").unwrap(),
            )],
            vec![OrderBookLevel::new(
                Price::from_str("101.00").unwrap(),
                Size::from_str("10.0").unwrap(),
            )],
            123456789,
        );

        let event = MarketEvent::OrderBookSnapshot(snapshot);
        market_state.update(&event);

        // Should generate a signal when there's a spread
        let signal = strategy.generate_signal(&market_state);
        assert!(signal.is_some());
    }

    #[test]
    fn test_order_refresh_time() {
        let mut strategy = MarketMakingStrategy::new(
            Price::from_str("0.5").unwrap(),
            Size::from_str("0.1").unwrap(),
            Size::from_str("1.0").unwrap(),
            2,
            Duration::from_millis(100), // Short refresh time for testing
        );

        // Create market state with a spread
        let mut market_state = MarketState::new("BTCUSDT".to_string());

        let snapshot = OrderBookSnapshot::new(
            "BTCUSDT".to_string(),
            "binance".to_string(),
            vec![OrderBookLevel::new(
                Price::from_str("100.00").unwrap(),
                Size::from_str("10.0").unwrap(),
            )],
            vec![OrderBookLevel::new(
                Price::from_str("101.00").unwrap(),
                Size::from_str("10.0").unwrap(),
            )],
            123456789,
        );

        let event = MarketEvent::OrderBookSnapshot(snapshot);
        market_state.update(&event);

        // First signal should be generated
        let signal1 = strategy.generate_signal(&market_state);
        assert!(signal1.is_some());

        // Immediate second call should not generate a signal due to cooldown
        let signal2 = strategy.generate_signal(&market_state);
        assert!(signal2.is_none());
    }
}
