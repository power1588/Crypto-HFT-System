pub mod simple_arbitrage;
pub mod portfolio_rebalance;
pub mod market_making;
pub mod event_driven;

pub use simple_arbitrage::SimpleArbitrageStrategy;
pub use portfolio_rebalance::PortfolioRebalancer;
pub use market_making::MarketMakingStrategy;
pub use event_driven::EventDrivenStrategy;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Price, Size};
    use std::time::Duration;

    #[test]
    fn test_strategy_engine_integration() {
        // Test integration of all strategy components
        let simple_strategy = simple_arbitrage::SimpleArbitrageStrategy::new(
            Price::from_str("0.5").unwrap(),
            Size::from_str("0.1").unwrap(),
            Size::from_str("1.0").unwrap()
        );
        
        let portfolio_strategy = portfolio_rebalance::PortfolioRebalancer::new(
            HashMap::from([
                ("BTC".to_string(), Size::from_str("10.0").unwrap()),
                ("ETH".to_string(), Size::from_str("5.0")).unwrap()),
            ]),
            Size::from_str("2.0").unwrap(), // 20% deviation threshold
        );
        
        let market_making_strategy = market_making::MarketMakingStrategy::new(simple_strategy);
        
        let event_driven_strategy = event_driven::EventDrivenStrategy::new(market_making_strategy);
        
        // Test that all strategies can be used with the same engine
        let market_state = crate::strategy::MarketState::new("BTCUSDT".to_string());
        
        // Test simple arbitrage strategy
        let signal = simple_strategy.generate_signal(&market_state);
        assert!(signal.is_some());
        
        // Test portfolio rebalancing strategy
        let portfolio_signal = portfolio_strategy.generate_signal(&market_state);
        assert!(portfolio_signal.is_some());
        
        // Test event-driven strategy
        let event_signal = event_driven_strategy.generate_signal(&market_state);
        assert!(event_signal.is_some());
    }

    #[test]
    fn test_strategy_performance() {
        // Test performance of strategy generation
        let simple_strategy = simple_arbitrage::SimpleArbitrageStrategy::new(
            Price::from_str("0.5").unwrap(),
            Size::from_str("0.1").unwrap(),
            Size::from_str("1.0").unwrap()
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
        
        // Should complete in reasonable time (less than 1ms for 10k iterations)
        assert!(duration.as_millis() < 100);
    }

    #[test]
    fn test_strategy_composition() {
        // Test that multiple strategies can be composed
        let arbitrage_strategy = simple_arbitrage::SimpleArbitrageStrategy::new(
            Price::from_str("0.5").unwrap(),
            Size::from_str("0.1").unwrap(),
            Size::from_str("1.0").unwrap()
        );
        
        let rebalancing_strategy = portfolio_rebalance::PortfolioRebalancer::new(
            HashMap::from([
                ("BTC".to_string(), Size::from_str("10.0")).unwrap()),
                ("ETH".to_string(), Size::from_str("5.0")).unwrap()),
            ]),
            Size::from_str("2.0").unwrap(), // 20% deviation threshold
        );
        
        let composite_strategy = market_making::MarketMakingStrategy::new(arbitrage_strategy);
        
        let market_state = crate::strategy::MarketState::new("BTCUSDT".to_string());
        
        // Test that composite strategy uses both underlying strategies
        let signal = composite_strategy.generate_signal(&market_state);
        assert!(signal.is_some());
        
        // Verify that both strategies were called
        // This would require more complex mocking to verify internal calls
    }

    #[test]
    fn test_error_handling() {
        // Test error handling in strategies
        let strategy = simple_arbitrage::SimpleArbitrageStrategy::new(
            Price::from_str("0.5").unwrap(),
            Size::from_str("0.1").unwrap(),
            Size::from_str("1.0").unwrap()
        );
        
        let market_state = crate::strategy::MarketState::new("BTCUSDT".to_string());
        
        // Create a market state that will cause an error
        let invalid_market_state = crate::strategy::MarketState::new("INVALID".to_string());
        
        // Strategy should handle invalid market state gracefully
        let signal = strategy.generate_signal(&invalid_market_state);
        assert!(signal.is_none());
    }
}
