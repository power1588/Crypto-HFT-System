use crate::strategy::engine::{MarketState, Signal, Strategy};
use crate::types::{Price, Size};
use rust_decimal::Decimal;
use std::collections::HashMap;

/// Simple arbitrage strategy that looks for price differences between exchanges
pub struct SimpleArbitrageStrategy {
    /// Minimum spread to trigger arbitrage
    min_spread: Price,
    /// Minimum quantity to trade
    min_quantity: Size,
    /// Maximum position size
    max_position: Size,
    /// Current positions by exchange and symbol
    positions: HashMap<String, HashMap<String, Size>>,
}

impl SimpleArbitrageStrategy {
    /// Create a new simple arbitrage strategy
    pub fn new(min_spread: Price, min_quantity: Size, max_position: Size) -> Self {
        Self {
            min_spread,
            min_quantity,
            max_position,
            positions: HashMap::new(),
        }
    }

    /// Get current position for an exchange and symbol
    pub fn get_position(&self, exchange: &str, symbol: &str) -> Size {
        self.positions
            .get(exchange)
            .and_then(|by_symbol| by_symbol.get(symbol))
            .cloned()
            .unwrap_or(Size::new(Decimal::ZERO))
    }

    /// Update position after a trade
    pub fn update_position(&mut self, exchange: &str, symbol: &str, quantity_change: Size) {
        let exchange_positions = self
            .positions
            .entry(exchange.to_string())
            .or_insert_with(HashMap::new);

        let current_position = exchange_positions
            .entry(symbol.to_string())
            .or_insert(Size::new(Decimal::ZERO));

        *current_position = *current_position + quantity_change;

        // Ensure position doesn't exceed maximum
        if *current_position > self.max_position {
            *current_position = self.max_position;
        }
    }

    /// Check if we have enough balance to place a trade
    pub fn can_trade(&self, exchange: &str, symbol: &str, quantity: Size) -> bool {
        let current_position = self.get_position(exchange, symbol);
        let available = if current_position > Size::new(Decimal::ZERO) {
            // We have a long position, can sell
            current_position
        } else {
            // We have a short position, can buy
            self.max_position + current_position
        };

        available >= quantity
    }
}

impl Strategy for SimpleArbitrageStrategy {
    fn generate_signal(&mut self, market_state: &MarketState) -> Option<Signal> {
        // For this simple example, we'll just check if there's a bid/ask spread
        // In a real implementation, you would compare prices across exchanges

        let symbol = &market_state.symbol;

        // Get best bid and ask
        let (bid_price, bid_size) = market_state.best_bid()?;
        let (ask_price, ask_size) = market_state.best_ask()?;

        // Calculate spread
        let spread = ask_price - bid_price;

        // Check if spread is large enough
        if spread < self.min_spread {
            return None;
        }

        // Determine trade quantity (minimum of bid and ask sizes)
        let trade_quantity = if bid_size < ask_size {
            bid_size
        } else {
            ask_size
        };

        // Check if quantity meets minimum requirement
        if trade_quantity < self.min_quantity {
            return None;
        }

        // For this example, we'll assume we're buying at the bid and selling at the ask
        // In a real implementation, you would track which exchange has which price

        // Calculate expected profit (excluding fees)
        let expected_profit = spread.value() * trade_quantity.value();

        // Generate arbitrage signal
        Some(Signal::Arbitrage {
            buy_exchange: "ExchangeA".to_string(), // Would be determined by price comparison
            sell_exchange: "ExchangeB".to_string(), // Would be determined by price comparison
            symbol: symbol.clone(),
            buy_price: bid_price,
            sell_price: ask_price,
            quantity: trade_quantity,
            expected_profit: Price::new(expected_profit),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orderbook::{OrderBookLevel, OrderBookSnapshot};
    use crate::traits::MarketEvent;
    use crate::types::{Price, Size};

    fn create_market_state_with_spread(
        bid_price: &str,
        ask_price: &str,
        bid_size: &str,
        ask_size: &str,
    ) -> MarketState {
        let mut market_state = MarketState::new("BTCUSDT".to_string());

        let snapshot = OrderBookSnapshot::new(
            "BTCUSDT".to_string(),
            "binance".to_string(),
            vec![OrderBookLevel::new(
                Price::from_str(bid_price).unwrap(),
                Size::from_str(bid_size).unwrap(),
            )],
            vec![OrderBookLevel::new(
                Price::from_str(ask_price).unwrap(),
                Size::from_str(ask_size).unwrap(),
            )],
            123456789,
        );

        let event = MarketEvent::OrderBookSnapshot(snapshot);
        market_state.update(&event);

        market_state
    }

    #[test]
    fn test_arbitrage_signal_generation() {
        let mut strategy = SimpleArbitrageStrategy::new(
            Price::from_str("0.5").unwrap(), // 0.5 USDT minimum spread
            Size::from_str("0.1").unwrap(),  // 0.1 BTC minimum quantity
            Size::from_str("1.0").unwrap(),  // 1.0 BTC maximum position
        );

        // Create a Market State with a profitable spread
        let market_state = create_market_state_with_spread("100.0", "101.0", "1.0", "1.0");

        // Generate signal
        let signal = strategy.generate_signal(&market_state);

        // Check that a signal was generated
        assert!(signal.is_some());

        // Check signal details
        if let Some(Signal::Arbitrage {
            buy_exchange,
            sell_exchange,
            symbol,
            buy_price,
            sell_price,
            quantity,
            expected_profit,
        }) = signal
        {
            assert_eq!(buy_exchange, "ExchangeA");
            assert_eq!(sell_exchange, "ExchangeB");
            assert_eq!(symbol, "BTCUSDT");
            assert_eq!(buy_price, Price::from_str("100.0").unwrap());
            assert_eq!(sell_price, Price::from_str("101.0").unwrap());
            assert_eq!(quantity, Size::from_str("1.0").unwrap());
            assert_eq!(expected_profit, Price::from_str("1.0").unwrap()); // 1.0 * 1.0
        } else {
            panic!("Expected Arbitrage signal");
        }
    }

    #[test]
    fn test_no_signal_when_spread_too_small() {
        let mut strategy = SimpleArbitrageStrategy::new(
            Price::from_str("0.5").unwrap(), // 0.5 USDT minimum spread
            Size::from_str("0.1").unwrap(),  // 0.1 BTC minimum quantity
            Size::from_str("1.0").unwrap(),  // 1.0 BTC maximum position
        );

        // Create a Market State with a small spread
        let market_state = create_market_state_with_spread("100.0", "100.3", "1.0", "1.0");

        // Generate signal
        let signal = strategy.generate_signal(&market_state);

        // Check that no signal was generated
        assert!(signal.is_none());
    }

    #[test]
    fn test_no_signal_when_quantity_too_small() {
        let mut strategy = SimpleArbitrageStrategy::new(
            Price::from_str("0.5").unwrap(), // 0.5 USDT minimum spread
            Size::from_str("0.1").unwrap(),  // 0.1 BTC minimum quantity
            Size::from_str("1.0").unwrap(),  // 1.0 BTC maximum position
        );

        // Create a Market State with a profitable spread but small quantity
        let market_state = create_market_state_with_spread("100.0", "101.0", "0.05", "0.05");

        // Generate signal
        let signal = strategy.generate_signal(&market_state);

        // Check that no signal was generated
        assert!(signal.is_none());
    }

    #[test]
    fn test_position_tracking() {
        let mut strategy = SimpleArbitrageStrategy::new(
            Price::from_str("0.5").unwrap(),
            Size::from_str("0.1").unwrap(),
            Size::from_str("1.0").unwrap(),
        );

        // Initially, no position
        assert_eq!(
            strategy.get_position("ExchangeA", "BTCUSDT"),
            Size::new(Decimal::ZERO)
        );

        // Update position
        strategy.update_position("ExchangeA", "BTCUSDT", Size::from_str("0.5").unwrap());

        // Check position was updated
        assert_eq!(
            strategy.get_position("ExchangeA", "BTCUSDT"),
            Size::from_str("0.5").unwrap()
        );

        // Update position again
        strategy.update_position("ExchangeA", "BTCUSDT", Size::from_str("0.3").unwrap());

        // Check position was updated
        assert_eq!(
            strategy.get_position("ExchangeA", "BTCUSDT"),
            Size::from_str("0.8").unwrap()
        );

        // Try to exceed maximum position
        strategy.update_position("ExchangeA", "BTCUSDT", Size::from_str("0.5").unwrap());

        // Check position was capped at maximum
        assert_eq!(
            strategy.get_position("ExchangeA", "BTCUSDT"),
            Size::from_str("1.0").unwrap()
        );
    }

    #[test]
    fn test_can_trade() {
        let mut strategy = SimpleArbitrageStrategy::new(
            Price::from_str("0.5").unwrap(),
            Size::from_str("0.1").unwrap(),
            Size::from_str("1.0").unwrap(),
        );

        // Add a position
        strategy.update_position("ExchangeA", "BTCUSDT", Size::from_str("0.5").unwrap());

        // Can trade with available balance
        assert!(strategy.can_trade("ExchangeA", "BTCUSDT", Size::from_str("0.3").unwrap()));

        // Cannot trade with insufficient balance
        assert!(!strategy.can_trade("ExchangeA", "BTCUSDT", Size::from_str("0.6").unwrap()));

        // Add a short position (negative)
        strategy.update_position("ExchangeB", "BTCUSDT", Size::from_str("-0.5").unwrap());

        // Can trade with available balance (short position can buy)
        assert!(strategy.can_trade("ExchangeB", "BTCUSDT", Size::from_str("0.3").unwrap()));

        // Can trade up to maximum position
        assert!(strategy.can_trade("ExchangeB", "BTCUSDT", Size::from_str("1.5").unwrap()));
    }
}
