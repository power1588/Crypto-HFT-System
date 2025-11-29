use crate::strategy::{MarketState, Signal, Strategy};
use crate::traits::MarketEvent;
use std::collections::HashMap;

/// Event-driven strategy that responds to market events
pub struct EventDrivenStrategy<S> {
    /// The underlying strategy implementation
    strategy: S,
    /// Market states for all symbols
    market_states: HashMap<String, MarketState>,
    /// Last signal generation time for each symbol
    last_signal_time: HashMap<String, std::time::Instant>,
}

impl<S> EventDrivenStrategy<S>
where
    S: Strategy,
{
    /// Create a new event-driven strategy
    pub fn new(strategy: S) -> Self {
        Self {
            strategy,
            market_states: HashMap::new(),
            last_signal_time: HashMap::new(),
        }
    }

    /// Process a market event and potentially generate a signal
    pub fn process_event(&mut self, event: MarketEvent) -> Option<Signal> {
        // Update market state
        let symbol = match &event {
            MarketEvent::OrderBookSnapshot(ref snapshot) => &snapshot.symbol,
            MarketEvent::OrderBookDelta(ref delta) => &delta.symbol,
            MarketEvent::Trade(ref trade) => &trade.symbol,
        };

        let symbol_str = symbol.value().to_string();
        let market_state = self
            .market_states
            .entry(symbol_str.clone())
            .or_insert_with(|| MarketState::new(symbol_str.clone()));

        market_state.update(&event);

        // Check if we should generate a signal
        let now = std::time::Instant::now();
        let last_signal = self.last_signal_time.get(&symbol_str);

        // Apply debounce/cooldown (1 second for this example)
        if let Some(last_time) = last_signal {
            if now.duration_since(*last_time) < std::time::Duration::from_secs(1) {
                return None;
            }
        }

        // Generate signal using the strategy
        if let Some(signal) = self.strategy.generate_signal(market_state) {
            self.last_signal_time.insert(symbol_str, now);
            return Some(signal);
        }

        None
    }

    /// Get market state for a symbol
    pub fn get_market_state(&self, symbol: &str) -> Option<&MarketState> {
        self.market_states.get(symbol)
    }

    /// Get all market states
    pub fn get_all_market_states(&self) -> &HashMap<String, MarketState> {
        &self.market_states
    }
}

/// Trait for event-driven strategies
pub trait EventDrivenStrategyTrait {
    /// The underlying strategy implementation
    type Strategy;

    /// Process a market event and potentially generate a signal
    fn process_event(&mut self, event: MarketEvent) -> Option<Self::Strategy>;

    /// Get market state for a symbol
    fn get_market_state(&self, symbol: &str) -> Option<&MarketState>;

    /// Get all market states
    fn get_all_market_states(&self) -> &HashMap<String, MarketState>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Price, Size};
    use std::time::Duration;

    struct MockStrategy {
        should_generate_signal: bool,
        signal_count: usize,
    }

    impl Strategy for MockStrategy {
        fn generate_signal(&mut self, _market_state: &MarketState) -> Option<Signal> {
            if self.should_generate_signal {
                self.signal_count += 1;
                Some(Signal::Arbitrage {
                    buy_exchange: "ExchangeA".to_string(),
                    sell_exchange: "ExchangeB".to_string(),
                    symbol: "BTCUSDT".to_string(),
                    buy_price: Price::from_str("100.00").unwrap(),
                    sell_price: Price::from_str("101.00").unwrap(),
                    quantity: Size::from_str("1.0").unwrap(),
                    expected_profit: Price::from_str("1.00").unwrap(),
                })
            } else {
                None
            }
        }
    }

    #[test]
    fn test_event_driven_strategy() {
        let mut strategy = EventDrivenStrategy::new(MockStrategy {
            should_generate_signal: true,
            signal_count: 0,
        });

        // Create a market state
        let mut market_state = MarketState::new("BTCUSDT".to_string());

        // Create a mock order book snapshot
        let snapshot = crate::orderbook::OrderBookSnapshot::new(
            "BTCUSDT".to_string(),
            "binance".to_string(),
            vec![crate::orderbook::OrderBookLevel::new(
                Price::from_str("100.00").unwrap(),
                Size::from_str("10.0").unwrap(),
            )],
            vec![crate::orderbook::OrderBookLevel::new(
                Price::from_str("101.00").unwrap(),
                Size::from_str("10.0").unwrap(),
            )],
            123456789,
        );

        let event = crate::traits::MarketEvent::OrderBookSnapshot(snapshot);
        market_state.update(&event);

        // Process first event - should generate signal
        let signal1 = strategy.process_event(event.clone());
        assert!(signal1.is_some());
        assert_eq!(strategy.strategy.signal_count, 1);

        // Process second event immediately - should not generate signal due to cooldown
        let signal2 = strategy.process_event(event.clone());
        assert!(signal2.is_none());
        assert_eq!(strategy.strategy.signal_count, 1);

        // Wait for cooldown to expire
        std::thread::sleep(Duration::from_secs(1));

        // Process third event - should generate signal again
        let signal3 = strategy.process_event(event.clone());
        assert!(signal3.is_some());
        assert_eq!(strategy.strategy.signal_count, 2);

        // Test market state access
        let btc_state = strategy.get_market_state("BTCUSDT");
        assert!(btc_state.is_some());
        assert_eq!(btc_state.unwrap().symbol, "BTCUSDT");

        // Test all market states
        let all_states = strategy.get_all_market_states();
        assert_eq!(all_states.len(), 1);
        assert_eq!(all_states.get("BTCUSDT").unwrap().symbol, "BTCUSDT");
    }
}
