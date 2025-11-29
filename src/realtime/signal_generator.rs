use crate::strategy::{MarketState, Signal, Strategy};
use crate::traits::{NewOrder, TimeInForce};
use crate::types::{Price, Size};
use log::{debug, warn};
use rust_decimal::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Signal generator configuration
#[derive(Debug, Clone)]
pub struct SignalGeneratorConfig {
    /// Default order size
    pub default_order_size: Size,
    /// Maximum order size
    pub max_order_size: Size,
    /// Minimum order size
    pub min_order_size: Size,
    /// Default time in force
    pub default_time_in_force: TimeInForce,
    /// Order size scaling factor
    pub order_size_scaling: f64,
    /// Maximum number of orders per signal
    pub max_orders_per_signal: usize,
    /// Enable order size adjustment based on volatility
    pub enable_volatility_adjustment: bool,
    /// Volatility threshold for order size adjustment
    pub volatility_threshold: Price,
    /// Order size adjustment factor for high volatility
    pub high_volatility_factor: f64,
}

impl Default for SignalGeneratorConfig {
    fn default() -> Self {
        Self {
            default_order_size: Size::from_str("0.1").unwrap(),
            max_order_size: Size::from_str("1.0").unwrap(),
            min_order_size: Size::from_str("0.01").unwrap(),
            default_time_in_force: TimeInForce::GoodTillCancelled,
            order_size_scaling: 1.0,
            max_orders_per_signal: 5,
            enable_volatility_adjustment: true,
            volatility_threshold: Price::from_str("0.5").unwrap(),
            high_volatility_factor: 0.5,
        }
    }
}

/// Signal generator that converts strategy signals to orders
pub struct SignalGenerator<S>
where
    S: Strategy + Send + Sync,
{
    /// Configuration
    config: SignalGeneratorConfig,
    /// Strategy
    strategy: Arc<RwLock<S>>,
    /// Market states by symbol
    market_states: Arc<RwLock<HashMap<String, MarketState>>>,
    /// Last signal time by symbol
    last_signal_times: Arc<RwLock<HashMap<String, std::time::Instant>>>,
    /// Signal cooldown by symbol
    signal_cooldowns: Arc<RwLock<HashMap<String, std::time::Duration>>>,
    /// Current volatility by symbol
    volatilities: Arc<RwLock<HashMap<String, Price>>>,
}

impl<S> SignalGenerator<S>
where
    S: Strategy + Send + Sync + 'static,
{
    /// Create a new signal generator
    pub fn new(config: SignalGeneratorConfig, strategy: S) -> Self {
        Self {
            config,
            strategy: Arc::new(RwLock::new(strategy)),
            market_states: Arc::new(RwLock::new(HashMap::new())),
            last_signal_times: Arc::new(RwLock::new(HashMap::new())),
            signal_cooldowns: Arc::new(RwLock::new(HashMap::new())),
            volatilities: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Update market state for a symbol
    pub async fn update_market_state(&self, symbol: &str, market_state: MarketState) {
        // Update volatility if enabled (do this before moving market_state)
        if self.config.enable_volatility_adjustment {
            self.update_volatility(symbol, &market_state).await;
        }

        let mut market_states = self.market_states.write().await;
        market_states.insert(symbol.to_string(), market_state);
    }

    /// Update volatility for a symbol
    async fn update_volatility(&self, symbol: &str, market_state: &MarketState) {
        // Simple volatility calculation based on price changes
        // In a real implementation, you'd use a more sophisticated method

        let mut volatilities = self.volatilities.write().await;

        // Get current mid price
        let current_mid_price = match (market_state.best_bid(), market_state.best_ask()) {
            (Some((bid_price, _)), Some((ask_price, _))) => {
                // Price - Price returns Price, Price / Price returns Decimal
                let spread = ask_price - bid_price;
                Some(bid_price + Price::new(spread.value() / rust_decimal::Decimal::new(2, 0)))
            }
            _ => None,
        };

        if let Some(mid_price) = current_mid_price {
            // Get previous volatility
            let prev_volatility = volatilities
                .get(symbol)
                .cloned()
                .unwrap_or(Price::new(rust_decimal::Decimal::ZERO));

            // Simple volatility calculation: absolute price change
            // In a real implementation, you'd use standard deviation or other methods
            let price_change = if let Some(prev_state) = self.get_market_state(symbol).await {
                match (prev_state.best_bid(), prev_state.best_ask()) {
                    (Some((prev_bid_price, _)), Some((prev_ask_price, _))) => {
                        let prev_spread = prev_ask_price - prev_bid_price;
                        let prev_mid_price = prev_bid_price
                            + Price::new(prev_spread.value() / rust_decimal::Decimal::new(2, 0));
                        (mid_price - prev_mid_price).abs()
                    }
                    _ => Price::new(rust_decimal::Decimal::ZERO),
                }
            } else {
                Price::new(rust_decimal::Decimal::ZERO)
            };

            // Update volatility with exponential moving average
            let alpha = Decimal::from_f64(0.1).unwrap_or(Decimal::new(1, 1)); // 0.1
            let one_minus_alpha = Decimal::ONE - alpha;

            // Convert to Decimal for calculation, then back to Price
            let prev_vol_value = prev_volatility.value();
            let change_value = price_change.value();
            let new_vol_value = prev_vol_value * one_minus_alpha + change_value * alpha;
            let new_volatility = Price::new(new_vol_value);

            volatilities.insert(symbol.to_string(), new_volatility);
        }
    }

    /// Get market state for a symbol
    async fn get_market_state(&self, symbol: &str) -> Option<MarketState> {
        let market_states = self.market_states.read().await;
        market_states.get(symbol).cloned()
    }

    /// Get current volatility for a symbol
    #[allow(dead_code)]
    async fn get_volatility(&self, symbol: &str) -> Price {
        let volatilities = self.volatilities.read().await;
        volatilities
            .get(symbol)
            .cloned()
            .unwrap_or(Price::new(rust_decimal::Decimal::ZERO))
    }

    /// Set signal cooldown for a symbol
    pub async fn set_signal_cooldown(&self, symbol: &str, cooldown: std::time::Duration) {
        let mut cooldowns = self.signal_cooldowns.write().await;
        cooldowns.insert(symbol.to_string(), cooldown);
    }

    /// Check if a signal should be generated for a symbol
    async fn should_generate_signal(&self, symbol: &str) -> bool {
        // Check cooldown
        let cooldowns = self.signal_cooldowns.read().await;
        if let Some(cooldown) = cooldowns.get(symbol) {
            let last_times = self.last_signal_times.read().await;
            if let Some(last_time) = last_times.get(symbol) {
                if last_time.elapsed() < *cooldown {
                    return false;
                }
            }
        }

        true
    }

    /// Record that a signal was generated for a symbol
    async fn record_signal_generated(&self, symbol: &str) {
        let mut last_times = self.last_signal_times.write().await;
        last_times.insert(symbol.to_string(), std::time::Instant::now());
    }

    /// Generate signals from strategy
    pub async fn generate_signals(&self) -> Vec<Signal> {
        let mut signals = Vec::new();

        // Get market states
        let market_states = self.market_states.read().await;

        // Generate signals for each symbol
        for (symbol, market_state) in market_states.iter() {
            // Check if we should generate a signal
            if !self.should_generate_signal(symbol).await {
                continue;
            }

            // Generate signal from strategy
            let signal = {
                let mut strategy = self.strategy.write().await;
                strategy.generate_signal(market_state)
            };

            if let Some(signal) = signal {
                debug!("Generated signal for {}: {:?}", symbol, signal);
                signals.push(signal);
                self.record_signal_generated(symbol).await;
            }
        }

        signals
    }

    /// Convert a signal to one or more orders
    pub fn signal_to_orders(&self, signal: &Signal) -> Vec<NewOrder> {
        match signal {
            Signal::PlaceOrder { order } => vec![order.clone()],
            Signal::CancelOrder {
                order_id,
                symbol,
                exchange_id,
            } => {
                // Convert cancel signal to order
                // In a real implementation, you'd need to track the original order
                // For now, we'll return an empty vector
                warn!(
                    "Cancel signal not implemented: {} {} {}",
                    order_id.as_str(),
                    symbol,
                    exchange_id
                );
                Vec::new()
            }
            Signal::CancelAllOrders {
                symbol,
                exchange_id,
            } => {
                // Convert cancel all signal to orders
                // In a real implementation, you'd need to track all open orders
                // For now, we'll return an empty vector
                warn!(
                    "Cancel all signal not implemented: {} {}",
                    symbol, exchange_id
                );
                Vec::new()
            }
            Signal::UpdateOrder {
                order_id,
                price,
                size,
                ..
            } => {
                // Convert update signal to order
                // In a real implementation, you'd need to track the original order
                // For now, we'll return an empty vector
                warn!(
                    "Update signal not implemented: {} {:?} {:?}",
                    order_id.as_str(),
                    price,
                    size
                );
                Vec::new()
            }
            Signal::Arbitrage {
                buy_exchange: _,
                sell_exchange: _,
                symbol,
                buy_price,
                sell_price,
                quantity,
                ..
            } => {
                // Convert arbitrage signal to two orders
                let buy_order = NewOrder::new_limit_buy(
                    symbol.clone(),
                    *quantity,
                    *buy_price,
                    self.config.default_time_in_force,
                );

                let sell_order = NewOrder::new_limit_sell(
                    symbol.clone(),
                    *quantity,
                    *sell_price,
                    self.config.default_time_in_force,
                );

                vec![buy_order, sell_order]
            }
            Signal::Custom { name, data } => {
                // Handle custom signals
                debug!("Custom signal: {} {:?}", name, data);

                // In a real implementation, you'd have custom logic for different signal types
                // For now, we'll return an empty vector
                Vec::new()
            }
        }
    }

    /// Adjust order size based on volatility
    #[allow(dead_code)]
    async fn adjust_order_size(&self, symbol: &str, base_size: Size) -> Size {
        if !self.config.enable_volatility_adjustment {
            return base_size;
        }

        let volatility = self.get_volatility(symbol).await;

        if volatility > self.config.volatility_threshold {
            // High volatility: reduce order size
            let factor =
                Decimal::from_f64(self.config.high_volatility_factor).unwrap_or(Decimal::ONE);
            let adjusted_value = base_size.value() * factor;
            let adjusted_size = Size::new(adjusted_value);

            // Ensure within bounds
            let min_size = self.config.min_order_size;
            let max_size = self.config.max_order_size;

            if adjusted_size < min_size {
                return min_size;
            } else if adjusted_size > max_size {
                return max_size;
            } else {
                return adjusted_size;
            }
        }

        // Normal volatility: use base size with scaling
        let scale_factor =
            Decimal::from_f64(self.config.order_size_scaling).unwrap_or(Decimal::ONE);
        let scaled_value = base_size.value() * scale_factor;
        let scaled_size = Size::new(scaled_value);

        // Ensure within bounds
        let min_size = self.config.min_order_size;
        let max_size = self.config.max_order_size;

        if scaled_size < min_size {
            min_size
        } else if scaled_size > max_size {
            max_size
        } else {
            scaled_size
        }
    }

    /// Convert a signal to a single order (for signals that represent one order)
    pub fn signal_to_order(&self, signal: &Signal) -> Option<NewOrder> {
        let orders = self.signal_to_orders(signal);

        // Return the first order if available
        orders.into_iter().next()
    }
}

/// Signal generator implementation for testing
#[allow(dead_code)]
pub struct SignalGeneratorImpl {
    /// Configuration
    config: SignalGeneratorConfig,
}

impl SignalGeneratorImpl {
    /// Create a new signal generator implementation
    pub fn new() -> Self {
        Self {
            config: SignalGeneratorConfig::default(),
        }
    }
}

impl SignalGeneratorImpl {
    #[allow(dead_code)]
    fn signal_to_order(&self, signal: &Signal) -> Option<NewOrder> {
        match signal {
            Signal::PlaceOrder { order } => Some(order.clone()),
            Signal::Arbitrage {
                symbol,
                buy_price,
                quantity,
                ..
            } => {
                // For arbitrage, just return the buy order
                Some(NewOrder::new_limit_buy(
                    symbol.clone(),
                    *quantity,
                    *buy_price,
                    self.config.default_time_in_force,
                ))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::engine::MarketState;
    use crate::strategy::simple_arbitrage::SimpleArbitrageStrategy;
    use crate::types::{Price, Size};
    use crate::OrderSide;

    #[test]
    fn test_signal_generator_config_default() {
        let config = SignalGeneratorConfig::default();

        assert_eq!(config.default_order_size, Size::from_str("0.1").unwrap());
        assert_eq!(config.max_order_size, Size::from_str("1.0").unwrap());
        assert_eq!(config.min_order_size, Size::from_str("0.01").unwrap());
        assert_eq!(config.default_time_in_force, TimeInForce::GoodTillCancelled);
        assert_eq!(config.order_size_scaling, 1.0);
        assert_eq!(config.max_orders_per_signal, 5);
        assert!(config.enable_volatility_adjustment);
        assert_eq!(config.volatility_threshold, Price::from_str("0.5").unwrap());
        assert_eq!(config.high_volatility_factor, 0.5);
    }

    #[tokio::test]
    async fn test_signal_generator_creation() {
        let config = SignalGeneratorConfig::default();
        let strategy = SimpleArbitrageStrategy::new(
            Price::from_str("0.5").unwrap(),
            Size::from_str("0.1").unwrap(),
            Size::from_str("1.0").unwrap(),
        );

        let generator = SignalGenerator::new(config, strategy);

        // Initially no market states
        let market_state = generator.get_market_state("BTCUSDT").await;
        assert!(market_state.is_none());

        // Initially no volatility
        let volatility = generator.get_volatility("BTCUSDT").await;
        assert_eq!(volatility, Price::new(rust_decimal::Decimal::ZERO));
    }

    #[tokio::test]
    async fn test_signal_generator_update_market_state() {
        let config = SignalGeneratorConfig::default();
        let strategy = SimpleArbitrageStrategy::new(
            Price::from_str("0.5").unwrap(),
            Size::from_str("0.1").unwrap(),
            Size::from_str("1.0").unwrap(),
        );

        let generator = SignalGenerator::new(config, strategy);

        // Update market state
        let market_state = MarketState::new("BTCUSDT".to_string());
        generator.update_market_state("BTCUSDT", market_state).await;

        // Verify market state was updated
        let retrieved_state = generator.get_market_state("BTCUSDT").await;
        assert!(retrieved_state.is_some());
        assert_eq!(retrieved_state.unwrap().symbol, "BTCUSDT");
    }

    #[tokio::test]
    async fn test_signal_generator_cooldown() {
        let config = SignalGeneratorConfig::default();
        let strategy = SimpleArbitrageStrategy::new(
            Price::from_str("0.5").unwrap(),
            Size::from_str("0.1").unwrap(),
            Size::from_str("1.0").unwrap(),
        );

        let generator = SignalGenerator::new(config, strategy);

        // Set cooldown
        generator
            .set_signal_cooldown("BTCUSDT", std::time::Duration::from_millis(100))
            .await;

        // Initially should generate signal
        assert!(generator.should_generate_signal("BTCUSDT").await);

        // Record signal generation
        generator.record_signal_generated("BTCUSDT").await;

        // Should not generate signal during cooldown
        assert!(!generator.should_generate_signal("BTCUSDT").await);

        // Wait for cooldown to expire
        tokio::time::sleep(std::time::Duration::from_millis(110)).await;

        // Should generate signal again
        assert!(generator.should_generate_signal("BTCUSDT").await);
    }

    #[tokio::test]
    async fn test_signal_generator_volatility() {
        let mut config = SignalGeneratorConfig::default();
        config.enable_volatility_adjustment = true;
        config.volatility_threshold = Price::from_str("0.1").unwrap();
        config.high_volatility_factor = 0.5;

        let strategy = SimpleArbitrageStrategy::new(
            Price::from_str("0.5").unwrap(),
            Size::from_str("0.1").unwrap(),
            Size::from_str("1.0").unwrap(),
        );

        let generator = SignalGenerator::new(config, strategy);

        // Update market state with high volatility
        let market_state = MarketState::new("BTCUSDT".to_string());
        generator.update_market_state("BTCUSDT", market_state).await;

        // Simulate high volatility
        let mut volatilities = generator.volatilities.write().await;
        volatilities.insert("BTCUSDT".to_string(), Price::from_str("1.0").unwrap());

        // Test order size adjustment
        let base_size = Size::from_str("0.5").unwrap();
        let adjusted_size = generator.adjust_order_size("BTCUSDT", base_size).await;

        // Should be reduced due to high volatility
        assert!(adjusted_size < base_size);
        assert_eq!(adjusted_size, Size::from_str("0.25").unwrap()); // 0.5 * 0.5
    }

    #[tokio::test]
    async fn test_signal_generator_signal_to_orders() {
        let config = SignalGeneratorConfig::default();
        let strategy = SimpleArbitrageStrategy::new(
            Price::from_str("0.5").unwrap(),
            Size::from_str("0.1").unwrap(),
            Size::from_str("1.0").unwrap(),
        );

        let generator = SignalGenerator::new(config, strategy);

        // Test place order signal
        let order = NewOrder::new_limit_buy(
            "BTCUSDT".to_string(),
            Size::from_str("0.1").unwrap(),
            Price::from_str("50000.0").unwrap(),
            TimeInForce::GoodTillCancelled,
        );

        let signal = Signal::PlaceOrder {
            order: order.clone(),
        };
        let orders = generator.signal_to_orders(&signal);
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0], order);

        // Test arbitrage signal
        let arbitrage_signal = Signal::Arbitrage {
            buy_exchange: "binance".to_string(),
            sell_exchange: "coinbase".to_string(),
            symbol: "BTCUSDT".to_string(),
            buy_price: Price::from_str("50000.0").unwrap(),
            sell_price: Price::from_str("50100.0").unwrap(),
            quantity: Size::from_str("0.1").unwrap(),
            expected_profit: Price::from_str("10.0").unwrap(),
        };

        let orders = generator.signal_to_orders(&arbitrage_signal);
        assert_eq!(orders.len(), 2); // Buy and sell orders

        // Verify buy order
        assert_eq!(orders[0].symbol.as_str(), "BTCUSDT");
        assert_eq!(orders[0].side, OrderSide::Buy);
        assert_eq!(orders[0].size, Size::from_str("0.1").unwrap());
        assert_eq!(orders[0].price, Some(Price::from_str("50000.0").unwrap()));

        // Verify sell order
        assert_eq!(orders[1].symbol.as_str(), "BTCUSDT");
        assert_eq!(orders[1].side, OrderSide::Sell);
        assert_eq!(orders[1].size, Size::from_str("0.1").unwrap());
        assert_eq!(orders[1].price, Some(Price::from_str("50100.0").unwrap()));
    }

    #[test]
    fn test_signal_generator_impl() {
        let generator = SignalGeneratorImpl::new();

        // Test place order signal
        let order = NewOrder::new_limit_buy(
            "BTCUSDT".to_string(),
            Size::from_str("0.1").unwrap(),
            Price::from_str("50000.0").unwrap(),
            TimeInForce::GoodTillCancelled,
        );

        let signal = Signal::PlaceOrder {
            order: order.clone(),
        };
        let result_order = generator.signal_to_order(&signal);
        assert!(result_order.is_some());
        assert_eq!(result_order.unwrap(), order);

        // Test arbitrage signal
        let arbitrage_signal = Signal::Arbitrage {
            buy_exchange: "binance".to_string(),
            sell_exchange: "coinbase".to_string(),
            symbol: "BTCUSDT".to_string(),
            buy_price: Price::from_str("50000.0").unwrap(),
            sell_price: Price::from_str("50100.0").unwrap(),
            quantity: Size::from_str("0.1").unwrap(),
            expected_profit: Price::from_str("10.0").unwrap(),
        };

        let result_order = generator.signal_to_order(&arbitrage_signal);
        assert!(result_order.is_some());

        // Should return the buy order
        let order = result_order.unwrap();
        assert_eq!(order.symbol.as_str(), "BTCUSDT");
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.size, Size::from_str("0.1").unwrap());
        assert_eq!(order.price, Some(Price::from_str("50000.0").unwrap()));
    }
}
