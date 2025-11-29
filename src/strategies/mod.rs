pub mod arbitrage;
pub mod event_driven;
pub mod market_making;
pub mod portfolio_rebalance;
pub mod prediction;
pub mod simple_arbitrage;

pub use arbitrage::ArbitrageStrategy;
pub use event_driven::EventDrivenStrategy;
pub use market_making::MarketMakingStrategy;
pub use portfolio_rebalance::PortfolioRebalancingStrategy as PortfolioRebalancer;
pub use prediction::LinearRegressionPredictor;
pub use simple_arbitrage::SimpleArbitrageStrategyImpl as SimpleArbitrageStrategy;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::Strategy;
    use crate::types::{Price, Size};
    use std::collections::HashMap;
    use std::time::Duration;

    #[test]
    fn test_strategy_engine_integration() {
        // Test integration of all strategy components
        let mut simple_strategy = simple_arbitrage::SimpleArbitrageStrategyImpl::new(
            Price::from_str("0.5").unwrap(),
            Size::from_str("0.1").unwrap(),
            Size::from_str("1.0").unwrap(),
        );

        let mut portfolio_strategy = portfolio_rebalance::PortfolioRebalancingStrategy::new(
            HashMap::from([
                ("BTC".to_string(), Size::from_str("10.0").unwrap()),
                ("ETH".to_string(), Size::from_str("5.0").unwrap()),
            ]),
            Size::from_str("2.0").unwrap(), // 20% deviation threshold
        );

        let mut market_making_strategy = market_making::MarketMakingStrategy::new(
            Price::from_str("0.5").unwrap(),
            Size::from_str("0.1").unwrap(),
            Size::from_str("1.0").unwrap(),
            5,
            Duration::from_millis(100),
        );

        // Test that all strategies can be used with the same engine
        let market_state = crate::strategy::MarketState::new("BTCUSDT".to_string());

        // Test simple arbitrage strategy
        let signal = simple_strategy.generate_signal(&market_state);
        // Signal may be None if market state has no data
        let _ = signal;

        // Test portfolio rebalancing strategy
        let portfolio_signal = portfolio_strategy.generate_signal(&market_state);
        let _ = portfolio_signal;

        // Test market making strategy
        let mm_signal = market_making_strategy.generate_signal(&market_state);
        let _ = mm_signal;
    }

    #[test]
    fn test_strategy_performance() {
        // Test performance of strategy generation
        let mut simple_strategy = simple_arbitrage::SimpleArbitrageStrategyImpl::new(
            Price::from_str("0.5").unwrap(),
            Size::from_str("0.1").unwrap(),
            Size::from_str("1.0").unwrap(),
        );

        let market_state = crate::strategy::MarketState::new("BTCUSDT".to_string());

        // Warm up
        for _ in 0..1000 {
            let _ = simple_strategy.generate_signal(&market_state);
        }

        // Measure performance
        let start = std::time::Instant::now();
        for _ in 0..10000 {
            let _ = simple_strategy.generate_signal(&market_state);
        }
        let duration = start.elapsed();

        // Should complete in reasonable time (less than 100ms for 10k iterations)
        assert!(duration.as_millis() < 100);
    }

    #[test]
    fn test_strategy_composition() {
        // Test that multiple strategies can be composed
        let mut market_making_strategy = market_making::MarketMakingStrategy::new(
            Price::from_str("0.5").unwrap(),
            Size::from_str("0.1").unwrap(),
            Size::from_str("1.0").unwrap(),
            5,
            Duration::from_millis(100),
        );

        let _rebalancing_strategy = portfolio_rebalance::PortfolioRebalancingStrategy::new(
            HashMap::from([
                ("BTC".to_string(), Size::from_str("10.0").unwrap()),
                ("ETH".to_string(), Size::from_str("5.0").unwrap()),
            ]),
            Size::from_str("2.0").unwrap(), // 20% deviation threshold
        );

        let market_state = crate::strategy::MarketState::new("BTCUSDT".to_string());

        // Test that composite strategy can generate signals
        let signal = market_making_strategy.generate_signal(&market_state);
        // Signal may be None if market state has no data
        let _ = signal;
    }

    #[test]
    fn test_error_handling() {
        // Test error handling in strategies
        let mut strategy = simple_arbitrage::SimpleArbitrageStrategyImpl::new(
            Price::from_str("0.5").unwrap(),
            Size::from_str("0.1").unwrap(),
            Size::from_str("1.0").unwrap(),
        );

        let _market_state = crate::strategy::MarketState::new("BTCUSDT".to_string());

        // Create a market state that will cause an error
        let invalid_market_state = crate::strategy::MarketState::new("INVALID".to_string());

        // Strategy should handle invalid market state gracefully
        let signal = strategy.generate_signal(&invalid_market_state);
        assert!(signal.is_none());
    }
}
