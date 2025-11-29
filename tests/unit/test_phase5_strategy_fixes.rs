/// Phase 5 TDD Tests: Strategy Module Fixes
/// 
/// These tests verify that all strategy module fixes have been correctly implemented
/// according to the task breakdown in specs/001-market-making/tasks.md
/// 
/// Run with: cargo test test_phase5 --test test_phase5_strategy_fixes
/// 
/// Test Coverage:
/// - T038: Position size comparison with negative max_position
/// - T042: Signal::Custom data field type (HashMap<String, String>)
/// - T043: needs_rebalancing() call signature (no args)
/// - T044: RebalanceData struct fields match usage
/// - T045: target_allocations clone before move

use crypto_hft::types::{Price, Size};
use crypto_hft::strategies::portfolio_rebalance::{PortfolioRebalancingStrategy, RebalanceData};
use crypto_hft::strategy::engine::{Strategy, MarketState, Signal};
use rust_decimal::Decimal;
use std::collections::HashMap;

#[cfg(test)]
mod portfolio_rebalance_tests {
    use super::*;

    /// T045: Test that PortfolioRebalancingStrategy can be constructed
    /// This verifies the clone-before-move fix for target_allocations
    #[test]
    fn test_portfolio_rebalancing_construction() {
        let mut target_allocations = HashMap::new();
        target_allocations.insert("BTC".to_string(), Size::new(Decimal::new(5, 1))); // 0.5 = 50%
        target_allocations.insert("ETH".to_string(), Size::new(Decimal::new(3, 1))); // 0.3 = 30%
        target_allocations.insert("USDT".to_string(), Size::new(Decimal::new(2, 1))); // 0.2 = 20%

        let threshold = Size::new(Decimal::new(5, 2)); // 0.05 = 5% deviation threshold
        
        // T045: This should not panic - clone happens before move
        let strategy = PortfolioRebalancingStrategy::new(target_allocations, threshold);
        
        // Verify target allocations are set correctly
        assert_eq!(
            strategy.get_target_allocation("BTC"),
            Some(Size::new(Decimal::new(5, 1)))
        );
        assert_eq!(
            strategy.get_target_allocation("ETH"),
            Some(Size::new(Decimal::new(3, 1)))
        );
    }

    /// T043: Test that needs_rebalancing() takes no arguments
    #[test]
    fn test_needs_rebalancing_no_args() {
        let mut target_allocations = HashMap::new();
        target_allocations.insert("BTC".to_string(), Size::new(Decimal::new(5, 1)));
        
        let threshold = Size::new(Decimal::new(5, 2));
        let strategy = PortfolioRebalancingStrategy::new(target_allocations, threshold);
        
        // T043: needs_rebalancing() should be callable without arguments
        // Since current_allocations equals target_allocations initially, should return false
        let needs_rebalance = strategy.needs_rebalancing();
        assert!(!needs_rebalance);
    }

    /// T044: Test that RebalanceData has correct fields
    #[test]
    fn test_rebalance_data_fields() {
        // T044: RebalanceData should have: asset, current_allocation, target_allocation, deviation
        let data = RebalanceData {
            asset: "BTC".to_string(),
            current_allocation: Size::new(Decimal::new(6, 1)), // 0.6
            target_allocation: Size::new(Decimal::new(5, 1)),   // 0.5
            deviation: Size::new(Decimal::new(1, 1)),           // 0.1
        };
        
        assert_eq!(data.asset, "BTC");
        assert_eq!(data.current_allocation.value(), Decimal::new(6, 1));
        assert_eq!(data.target_allocation.value(), Decimal::new(5, 1));
        assert_eq!(data.deviation.value(), Decimal::new(1, 1));
    }

    /// T042: Test that Signal::Custom uses HashMap<String, String> for data
    #[test]
    fn test_signal_custom_data_type() {
        // T042: Signal::Custom.data should be HashMap<String, String>
        let mut data = HashMap::new();
        data.insert("asset".to_string(), "BTC".to_string());
        data.insert("current_allocation".to_string(), "0.6".to_string());
        data.insert("target_allocation".to_string(), "0.5".to_string());
        data.insert("deviation".to_string(), "0.1".to_string());
        
        let signal = Signal::Custom {
            name: "rebalance".to_string(),
            data,
        };
        
        if let Signal::Custom { name, data } = signal {
            assert_eq!(name, "rebalance");
            assert_eq!(data.get("asset"), Some(&"BTC".to_string()));
            assert_eq!(data.get("deviation"), Some(&"0.1".to_string()));
        } else {
            panic!("Expected Signal::Custom");
        }
    }

    /// Test generate_signals method returns correct signals
    #[test]
    fn test_generate_signals() {
        let mut target_allocations = HashMap::new();
        target_allocations.insert("BTC".to_string(), Size::new(Decimal::new(5, 1))); // 0.5 target
        
        let threshold = Size::new(Decimal::new(5, 2)); // 0.05 threshold
        let mut strategy = PortfolioRebalancingStrategy::new(target_allocations, threshold);
        
        // Manually trigger rebalancing by changing target allocation
        // This creates a deviation since current_allocations was initialized from original target
        strategy.set_target_allocation("BTC".to_string(), Size::new(Decimal::new(7, 1))); // 0.7 new target
        
        // Now there's a deviation: current is 0.5, target is 0.7, deviation = 0.2 > 0.05 threshold
        let market_state = MarketState::new("BTCUSDT".to_string());
        let signals = strategy.generate_signals(&market_state);
        
        // Should generate a rebalancing signal
        assert!(!signals.is_empty());
        
        // Check the signal is a Custom signal with correct name
        if let Some(Signal::Custom { name, data }) = signals.first() {
            assert_eq!(name, "rebalance");
            // Data should contain asset information
            assert!(data.contains_key("asset"));
        } else {
            panic!("Expected Signal::Custom for rebalance");
        }
    }

    /// Test Strategy trait implementation - generate_signal method
    #[test]
    fn test_strategy_generate_signal() {
        let mut target_allocations = HashMap::new();
        target_allocations.insert("BTC".to_string(), Size::new(Decimal::new(5, 1)));
        
        let threshold = Size::new(Decimal::new(5, 2));
        let mut strategy = PortfolioRebalancingStrategy::new(target_allocations, threshold);
        
        // Create a deviation
        strategy.set_target_allocation("BTC".to_string(), Size::new(Decimal::new(7, 1)));
        
        let market_state = MarketState::new("BTCUSDT".to_string());
        
        // T042, T043: generate_signal should work correctly now
        let signal = strategy.generate_signal(&market_state);
        assert!(signal.is_some());
        
        if let Some(Signal::Custom { name, data }) = signal {
            assert_eq!(name, "rebalance");
            // T042: data is HashMap<String, String>
            assert!(data.contains_key("asset") || data.contains_key("symbol"));
        } else {
            panic!("Expected Signal::Custom for rebalance");
        }
    }
}

#[cfg(test)]
mod market_making_position_tests {
    use super::*;
    use crypto_hft::strategies::market_making::MarketMakingStrategy;
    use crypto_hft::OrderSide;
    use std::time::Duration;

    /// T038: Test position size comparison with negative max_position
    #[test]
    fn test_sell_order_position_limit_negative() {
        let strategy = MarketMakingStrategy::new(
            Price::new(Decimal::new(50, 2)),  // 0.50 target spread
            Size::new(Decimal::new(10, 2)),   // 0.10 base order size
            Size::new(Decimal::new(100, 2)),  // 1.00 max position
            3,                                 // 3 order levels
            Duration::from_millis(100),       // 100ms refresh time
        );
        
        // Should be able to sell up to max_position (go short by 1.00)
        // This tests: new_position >= -max_position_size
        assert!(strategy.can_place_order("BTCUSDT", OrderSide::Sell, Size::new(Decimal::new(50, 2)))); // -0.50
        assert!(strategy.can_place_order("BTCUSDT", OrderSide::Sell, Size::new(Decimal::new(100, 2)))); // -1.00
        
        // Should NOT be able to sell more than max_position
        assert!(!strategy.can_place_order("BTCUSDT", OrderSide::Sell, Size::new(Decimal::new(150, 2)))); // -1.50 exceeds limit
    }

    /// T038: Test position comparisons with Size negation
    #[test]
    fn test_position_comparison_with_negated_size() {
        let max_position = Size::new(Decimal::new(100, 2)); // 1.00
        let neg_max = -max_position;
        
        // Verify negation works correctly
        assert_eq!(neg_max.value(), Decimal::new(-100, 2)); // -1.00
        
        // Test various position comparisons
        let position_minus_50 = Size::new(Decimal::new(-50, 2)); // -0.50
        let position_minus_100 = Size::new(Decimal::new(-100, 2)); // -1.00
        let position_minus_150 = Size::new(Decimal::new(-150, 2)); // -1.50
        
        // -0.50 >= -1.00 should be true (not at limit)
        assert!(position_minus_50 >= neg_max);
        
        // -1.00 >= -1.00 should be true (exactly at limit)
        assert!(position_minus_100 >= neg_max);
        
        // -1.50 >= -1.00 should be false (exceeds limit)
        assert!(!(position_minus_150 >= neg_max));
    }

    /// Test market making strategy with existing position
    #[test]
    fn test_sell_with_existing_long_position() {
        let mut strategy = MarketMakingStrategy::new(
            Price::new(Decimal::new(50, 2)),
            Size::new(Decimal::new(10, 2)),
            Size::new(Decimal::new(100, 2)), // max 1.00
            3,
            Duration::from_millis(100),
        );
        
        // Set a long position of 0.50
        strategy.update_position("BTCUSDT", Size::new(Decimal::new(50, 2)));
        
        // With +0.50 position, can sell up to 1.50 (going from +0.50 to -1.00)
        // new_position = 0.50 - 1.50 = -1.00, which is exactly at -max_position
        assert!(strategy.can_place_order("BTCUSDT", OrderSide::Sell, Size::new(Decimal::new(150, 2))));
        
        // Cannot sell 1.60 (would go to -1.10, below -max)
        assert!(!strategy.can_place_order("BTCUSDT", OrderSide::Sell, Size::new(Decimal::new(160, 2))));
    }

    /// Test buy order limits
    #[test]
    fn test_buy_order_position_limit() {
        let strategy = MarketMakingStrategy::new(
            Price::new(Decimal::new(50, 2)),
            Size::new(Decimal::new(10, 2)),
            Size::new(Decimal::new(100, 2)), // max 1.00
            3,
            Duration::from_millis(100),
        );
        
        // Should be able to buy up to max_position
        assert!(strategy.can_place_order("BTCUSDT", OrderSide::Buy, Size::new(Decimal::new(100, 2)))); // +1.00
        
        // Should NOT be able to buy more than max_position
        assert!(!strategy.can_place_order("BTCUSDT", OrderSide::Buy, Size::new(Decimal::new(150, 2)))); // +1.50 exceeds limit
    }
}

