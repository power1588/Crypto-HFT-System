use crate::strategy::{Strategy, MarketState, Signal};
use crate::types::{Price, Size};
use std::collections::HashMap;

/// Portfolio rebalancing strategy that maintains target allocations
pub struct PortfolioRebalancingStrategy {
    /// Target allocations for each asset
    target_allocations: HashMap<String, Size>,
    /// Current allocations
    current_allocations: HashMap<String, Size>,
    /// Rebalancing threshold
    rebalancing_threshold: Size,
}

impl PortfolioRebalancingStrategy {
    /// Create a new portfolio rebalancing strategy
    pub fn new(target_allocations: HashMap<String, Size>, rebalancing_threshold: Size) -> Self {
        Self {
            target_allocations,
            current_allocations: target_allocations.clone(),
            rebalancing_threshold,
        }
    }

    /// Set target allocation for an asset
    pub fn set_target_allocation(&mut self, asset: String, target_allocation: Size) {
        self.target_allocations.insert(asset, target_allocation);
    }

    /// Get target allocation for an asset
    pub fn get_target_allocation(&self, asset: &str) -> Option<Size> {
        self.target_allocations.get(asset).cloned()
    }

    /// Get current allocation for an asset
    pub fn get_current_allocation(&self, asset: &str) -> Option<Size> {
        self.current_allocations.get(asset).cloned()
    }

    /// Get rebalancing threshold
    pub fn get_rebalancing_threshold(&self) -> Size {
        self.rebalancing_threshold
    }

    /// Check if rebalancing is needed
    pub fn needs_rebalancing(&self) -> bool {
        for (asset, target_allocation) in &self.target_allocations {
            if let Some(current) = self.get_current_allocation(asset) {
                let deviation = if current > *target_allocation {
                    current - *target_allocation
                } else {
                    *target_allocation - current
                };
                
                if deviation > self.rebalancing_threshold {
                    return true;
                }
            }
        }
        false
    }

    /// Generate rebalancing signals
    pub fn generate_signals(&mut self, market_state: &MarketState) -> Vec<Signal> {
        let mut signals = Vec::new();
        
        for (asset, target_allocation) in &self.target_allocations {
            if let Some(current) = self.get_current_allocation(asset) {
                let deviation = if current > *target_allocation {
                    current - *target_allocation
                } else {
                    *target_allocation - current
                };
                
                if deviation > self.rebalancing_threshold {
                    // Generate rebalancing signal
                    let signal = Signal::Custom {
                        name: "rebalance".to_string(),
                        data: {
                            asset: asset.clone(),
                            current_allocation: current.clone(),
                            target_allocation: target_allocation.clone(),
                            deviation: deviation.clone(),
                        },
                    };
                    
                    signals.push(signal);
                }
            }
        }
        
        signals
    }
}

impl Strategy for PortfolioRebalancingStrategy {
    fn generate_signal(&mut self, market_state: &MarketState) -> Option<Signal> {
        // For this example, we'll just check if we need to rebalance
        if self.needs_rebalancing(market_state) {
            // Generate a simple rebalancing signal
            Some(Signal::Custom {
                name: "rebalance".to_string(),
                data: {
                    symbol: market_state.symbol.clone(),
                },
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Price, Size};

    #[test]
    fn test_portfolio_rebalancing_strategy() {
        let mut strategy = PortfolioRebalancingStrategy::new(
            HashMap::from([
                ("BTC".to_string(), Size::from_str("10.0").unwrap()),
                ("ETH".to_string(), Size::from_str("5.0").unwrap()),
            ]),
            Size::from_str("2.0").unwrap(), // 20% deviation threshold
        );
        
        // Set initial allocations
        strategy.set_target_allocation("BTC".to_string(), Size::from_str("8.0").unwrap());
        strategy.set_target_allocation("ETH".to_string(), Size::from_str("6.0").unwrap());
        
        // Create a market state
        let mut market_state = MarketState::new("BTCUSDT".to_string());
        
        // Initially, no rebalancing needed
        assert!(!strategy.needs_rebalancing(&market_state));
        
        // Update BTC allocation to trigger rebalancing
        strategy.set_target_allocation("BTC".to_string(), Size::from_str("12.0").unwrap());
        
        // Now rebalancing is needed
        assert!(strategy.needs_rebalancing(&market_state));
        
        // Generate rebalancing signal
        let signal = strategy.generate_signal(&market_state);
        assert!(signal.is_some());
        
        // Check signal details
        if let Some(Signal::Custom { name, data }) = signal {
            assert_eq!(name, "rebalance");
            assert_eq!(data.get("symbol").unwrap(), "BTCUSDT");
            assert_eq!(data.get("current_allocation").unwrap(), Size::from_str("12.0").unwrap());
            assert_eq!(data.get("target_allocation").unwrap(), Size::from_str("10.0").unwrap());
            assert_eq!(data.get("deviation").unwrap(), Size::from_str("2.0").unwrap());
        } else {
            panic!("Expected rebalancing signal");
        }
        
        // Update ETH allocation to trigger rebalancing
        strategy.set_target_allocation("ETH".to_string(), Size::from_str("8.0").unwrap());
        
        // Now rebalancing is needed for ETH too
        assert!(strategy.needs_rebalancing(&market_state));
        
        // Generate rebalancing signal for ETH
        let signal = strategy.generate_signal(&market_state);
        assert!(signal.is_some());
        
        // Check ETH signal details
        if let Some(Signal::Custom { name, data }) = signal {
            assert_eq!(name, "rebalance");
            assert_eq!(data.get("symbol").unwrap(), "ETHUSDT");
            assert_eq!(data.get("current_allocation").unwrap(), Size::from_str("8.0").unwrap());
            assert_eq!(data.get("target_allocation").unwrap(), Size::from_str("6.0").unwrap());
            assert_eq!(data.get("deviation").unwrap(), Size::from_str("2.0").unwrap());
        } else {
            panic!("Expected rebalancing signal");
        }
        
        // Update ETH allocation to not trigger rebalancing
        strategy.set_target_allocation("ETH".to_string(), Size::from_str("6.0").unwrap());
        
        // Now rebalancing is not needed for ETH
        assert!(!strategy.needs_rebalancing(&market_state));
        
        // Should not generate rebalancing signal for ETH
        let signal = strategy.generate_signal(&market_state);
        assert!(signal.is_none());
    }
}
