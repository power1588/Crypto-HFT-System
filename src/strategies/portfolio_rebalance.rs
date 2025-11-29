use crate::strategy::{MarketState, Signal, Strategy};
use crate::types::Size;
use std::collections::HashMap;

/// Data for rebalancing signal
/// Used for structured access to rebalance information outside of Signal::Custom
#[derive(Debug, Clone)]
pub struct RebalanceData {
    pub asset: String,
    pub current_allocation: Size,
    pub target_allocation: Size,
    pub deviation: Size,
}

impl RebalanceData {
    /// Convert RebalanceData to HashMap<String, String> for Signal::Custom
    pub fn to_hashmap(&self) -> HashMap<String, String> {
        let mut data = HashMap::new();
        data.insert("asset".to_string(), self.asset.clone());
        data.insert(
            "current_allocation".to_string(),
            self.current_allocation.to_string(),
        );
        data.insert(
            "target_allocation".to_string(),
            self.target_allocation.to_string(),
        );
        data.insert("deviation".to_string(), self.deviation.to_string());
        data
    }
}

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
    /// T045: Clone target_allocations before moving to avoid borrow-after-move
    pub fn new(target_allocations: HashMap<String, Size>, rebalancing_threshold: Size) -> Self {
        let current_allocations = target_allocations.clone();
        Self {
            target_allocations,
            current_allocations,
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
    /// T043: Takes no arguments - compares internal state only
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
    /// T042: Returns Signal::Custom with HashMap<String, String> data
    pub fn generate_signals(&mut self, _market_state: &MarketState) -> Vec<Signal> {
        let mut signals = Vec::new();

        for (asset, target_allocation) in &self.target_allocations {
            if let Some(current) = self.get_current_allocation(asset) {
                let deviation = if current > *target_allocation {
                    current - *target_allocation
                } else {
                    *target_allocation - current
                };

                if deviation > self.rebalancing_threshold {
                    // T042: Generate rebalancing signal with HashMap<String, String> data
                    let rebalance_data = RebalanceData {
                        asset: asset.clone(),
                        current_allocation: current,
                        target_allocation: *target_allocation,
                        deviation,
                    };

                    let signal = Signal::Custom {
                        name: "rebalance".to_string(),
                        data: rebalance_data.to_hashmap(),
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
        // T043: needs_rebalancing() takes no arguments
        if self.needs_rebalancing() {
            // Find the first asset that needs rebalancing and generate signal
            for (asset, target_allocation) in &self.target_allocations {
                if let Some(current) = self.get_current_allocation(asset) {
                    let deviation = if current > *target_allocation {
                        current - *target_allocation
                    } else {
                        *target_allocation - current
                    };

                    if deviation > self.rebalancing_threshold {
                        // T042, T044: Use HashMap<String, String> for data, include asset not symbol
                        let rebalance_data = RebalanceData {
                            asset: asset.clone(),
                            current_allocation: current,
                            target_allocation: *target_allocation,
                            deviation,
                        };

                        let mut data = rebalance_data.to_hashmap();
                        // Also include symbol from market_state for context
                        data.insert("symbol".to_string(), market_state.symbol.clone());

                        return Some(Signal::Custom {
                            name: "rebalance".to_string(),
                            data,
                        });
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Size;

    #[test]
    fn test_portfolio_rebalancing_strategy() {
        // T045: Test construction with clone-before-move fix
        let mut strategy = PortfolioRebalancingStrategy::new(
            HashMap::from([
                ("BTC".to_string(), Size::from_str("10.0").unwrap()),
                ("ETH".to_string(), Size::from_str("5.0").unwrap()),
            ]),
            Size::from_str("2.0").unwrap(), // 2.0 deviation threshold
        );

        // Set initial allocations
        strategy.set_target_allocation("BTC".to_string(), Size::from_str("8.0").unwrap());
        strategy.set_target_allocation("ETH".to_string(), Size::from_str("6.0").unwrap());

        // Create a market state
        let market_state = MarketState::new("BTCUSDT".to_string());

        // Initially, deviation between current (10.0, 5.0) and target (8.0, 6.0) = (2.0, 1.0)
        // BTC deviation of 2.0 equals threshold, so technically needs rebalancing
        // (depends on whether we use > or >= for comparison - using >)
        // current: BTC=10.0, ETH=5.0, target: BTC=8.0, ETH=6.0
        // BTC deviation = 2.0, threshold = 2.0, 2.0 > 2.0 is false
        // ETH deviation = 1.0, threshold = 2.0, 1.0 > 2.0 is false
        // T043: needs_rebalancing() takes no arguments
        assert!(!strategy.needs_rebalancing());

        // Update BTC target allocation to trigger rebalancing (increase deviation beyond threshold)
        strategy.set_target_allocation("BTC".to_string(), Size::from_str("12.0").unwrap());

        // Now deviation = |10.0 - 12.0| = 2.0, which is NOT > 2.0 threshold
        // Need larger deviation
        strategy.set_target_allocation("BTC".to_string(), Size::from_str("15.0").unwrap());

        // Now deviation = |10.0 - 15.0| = 5.0 > 2.0 threshold
        assert!(strategy.needs_rebalancing());

        // Generate rebalancing signal
        let signal = strategy.generate_signal(&market_state);
        assert!(signal.is_some());

        // T042: Check signal uses HashMap<String, String> data
        if let Some(Signal::Custom { name, data }) = signal {
            assert_eq!(name, "rebalance");
            // T042: data is HashMap<String, String>
            assert!(data.contains_key("asset"));
            assert!(data.contains_key("current_allocation"));
            assert!(data.contains_key("target_allocation"));
            assert!(data.contains_key("deviation"));
            assert!(data.contains_key("symbol"));
            assert_eq!(data.get("symbol").unwrap(), "BTCUSDT");
        } else {
            panic!("Expected rebalancing signal");
        }
    }

    #[test]
    fn test_needs_rebalancing_no_args() {
        // T043: Verify needs_rebalancing() takes no arguments
        let strategy = PortfolioRebalancingStrategy::new(
            HashMap::from([("BTC".to_string(), Size::from_str("10.0").unwrap())]),
            Size::from_str("2.0").unwrap(),
        );

        // This should compile - needs_rebalancing takes no args
        let _ = strategy.needs_rebalancing();
    }

    #[test]
    fn test_rebalance_data_to_hashmap() {
        // T042: Test RebalanceData.to_hashmap() conversion
        let data = RebalanceData {
            asset: "BTC".to_string(),
            current_allocation: Size::from_str("10.0").unwrap(),
            target_allocation: Size::from_str("8.0").unwrap(),
            deviation: Size::from_str("2.0").unwrap(),
        };

        let hashmap = data.to_hashmap();

        assert_eq!(hashmap.get("asset"), Some(&"BTC".to_string()));
        assert_eq!(hashmap.get("current_allocation"), Some(&"10.0".to_string()));
        assert_eq!(hashmap.get("target_allocation"), Some(&"8.0".to_string()));
        assert_eq!(hashmap.get("deviation"), Some(&"2.0".to_string()));
    }

    #[test]
    fn test_constructor_clones_allocations() {
        // T045: Test that constructor properly clones allocations
        let target = HashMap::from([("BTC".to_string(), Size::from_str("10.0").unwrap())]);

        let strategy = PortfolioRebalancingStrategy::new(target, Size::from_str("1.0").unwrap());

        // Both should have the same initial values
        assert_eq!(
            strategy.get_target_allocation("BTC"),
            Some(Size::from_str("10.0").unwrap())
        );
        assert_eq!(
            strategy.get_current_allocation("BTC"),
            Some(Size::from_str("10.0").unwrap())
        );
    }
}
