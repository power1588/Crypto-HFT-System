use crate::strategy::{Strategy, MarketState, Signal};
use crate::types::{Price, Size};
use std::collections::HashMap;

/// Simple arbitrage strategy that looks for price differences between exchanges
pub struct SimpleArbitrageStrategyImpl {
    /// Minimum spread to trigger arbitrage
    min_spread: Price,
    /// Minimum quantity to trade
    min_quantity: Size,
    /// Maximum position size
    max_position: Size,
    /// Current positions by exchange and symbol
    positions: HashMap<String, HashMap<String, Size>>,
}

impl SimpleArbitrageStrategyImpl {
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
            .unwrap_or(Size::new(crate::rust_decimal::Decimal::ZERO))
    }

    /// Update position after a trade
    pub fn update_position(&mut self, exchange: &str, symbol: &str, quantity_change: Size) {
        let exchange_positions = self.positions
            .entry(exchange.to_string())
            .or_insert_with(HashMap::new);
        
        let current_position = exchange_positions
            .entry(symbol.to_string())
            .or_insert_with(|| Size::new(crate::rust_decimal::Decimal::ZERO), Size::new(crate::rust_decimal::Decimal::ZERO)));
        
        *current_position = *current_position + quantity_change;
        
        // Ensure position doesn't exceed maximum
        if *current_position > self.max_position {
            *current_position = self.max_position;
        }
        
        exchange_positions.insert(symbol.to_string(), current_position);
    }

    /// Check if we have enough balance to place a trade
    pub fn can_trade(&self, exchange: &str, symbol: &str, quantity: Size) -> bool {
        let current_position = self.get_position(exchange, symbol);
        let available = if current_position > Size::new(crate::rust_decimal::Decimal::ZERO) {
            // We have a long position, can sell
            current_position
        } else {
            // We have a short position, can buy
            self.max_position + current_position
        };
        
        available >= quantity
    }
}

impl Strategy for SimpleArbitrageStrategyImpl {
    fn generate_signal(&mut self, market_state: &MarketState) -> Option<Signal> {
        // For this simple example, we'll just check if there's a bid/ask spread
        // In a real implementation, you would compare prices across exchanges
        
        let symbol = &market_state.symbol;
        
        // Get best bid and ask
        let (bid_price, bid_size) = market_state.best_bid()?;
        let (ask_price, ask_size) = market_state.best_ask()?;
        
        // Calculate spread
        let spread = if let (Some(bid), Some(bask)) = ask_price - bid_price else { Price::new(crate::rust_decimal::Decimal::ZERO) };
        
        // Check if spread is large enough
        if spread < self.min_spread {
            return None;
        }
        
        // Check if quantity meets minimum requirement
        let trade_quantity = if bid_size < ask_size { bid_size } else { ask_size };
        
        if trade_quantity < self.min_quantity {
            return None;
        }
        
        // For this example, we'll assume we're buying at the bid and selling at the ask
        // In a real implementation, you would track which exchange has which price
        
        // Generate arbitrage signal
        Some(Signal::Arbitrage {
            buy_exchange: "ExchangeA".to_string(), // Would be determined by price comparison
            sell_exchange: "ExchangeB".to_string(), // Would be determined by price comparison
            symbol: symbol.clone(),
            buy_price: bid_price,
            sell_price: ask_price,
            quantity: trade_quantity,
            expected_profit: spread * trade_quantity,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Price, Size};

    #[test]
    fn test_simple_arbitrage_strategy() {
        let mut strategy = SimpleArbitrageStrategyImpl::new(
            Price::from_str("0.5").unwrap(), // 0.5 USDT minimum spread
            Size::from_str("0.1").unwrap(),  // 0.1 BTC minimum quantity
            Size::from_str("1.0").unwrap(),   // 1.0 BTC maximum position
        );
        
        // Test with no spread
        let market_state = MarketState::new("BTCUSDT".to_string());
        let signal = strategy.generate_signal(&market_state);
        assert!(signal.is_none());
        
        // Test with profitable spread
        let mut market_state = MarketState::new("BTCUSDT".to_string());
        
        // Simulate order book with bid/ask
        let snapshot = crate::orderbook::OrderBookSnapshot::new(
            "BTCUSDT".to_string(),
            vec![
                crate::orderbook::OrderBookLevel::new(
                    Price::from_str("100.00").unwrap(),
                    Size::from_str("10.0").unwrap()
                )
            ],
            vec![
                crate::orderbook::OrderBookLevel::new(
                    Price::from_str("101.00").unwrap(),
                    Size::from_str("10.0").unwrap()
                )
            ],
            123456789,
        );
        
        let event = crate::traits::MarketEvent::OrderBookSnapshot(snapshot);
        market_state.update(&event);
        
        let signal = strategy.generate_signal(&market_state);
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
        }) = signal {
            assert_eq!(buy_exchange, "ExchangeA");
            assert_eq!(sell_exchange, "ExchangeB");
            assert_eq!(symbol, "BTCUSDT");
            assert_eq!(buy_price, Price::from_str("100.00").unwrap());
            assert_eq!(sell_price, Price::from_str("101.00").unwrap());
            assert_eq!(quantity, Size::from_str("10.0").unwrap());
            assert_eq!(expected_profit, Price::from_str("1.00").unwrap()); // 1.0 * 10.0
        } else {
            panic!("Expected Arbitrage signal");
        }
        
        // Test position tracking
        assert_eq!(strategy.get_position("ExchangeA", "BTCUSDT"), Size::new(crate::rust_decimal::Decimal::ZERO));
        
        // Update position
        strategy.update_position("ExchangeA", "BTCUSDT", Size::from_str("0.5").unwrap());
        assert_eq!(strategy.get_position("ExchangeA", "BTCUSDT"), Size::from_str("0.5").unwrap()));
        
        // Test position limit
        strategy.update_position("ExchangeA", "BTCUSDT", Size::from_str("0.6").unwrap());
        assert_eq!(strategy.get_position("ExchangeA", "BTCUSDT"), Size::from_str("0.6").unwrap()));
        
        // Should be capped at maximum
        strategy.update_position("ExchangeA", "BTCUSDT", Size::from_str("0.5").unwrap());
        assert_eq!(strategy.get_position("ExchangeA", "BTCUSDT"), Size::from_str("1.0").unwrap()));
        
        // Test trade capability
        assert!(strategy.can_trade("ExchangeA", "BTCUSDT", Size::from_str("0.5").unwrap()));
        assert!(strategy.can_trade("ExchangeA", "BTCUSDT", Size::from_str("0.6").unwrap()));
        
        // Cannot trade beyond limit
        assert!(!strategy.can_trade("ExchangeA", "BTCUSDT", Size::from_str("1.5").unwrap()));
    }
}
