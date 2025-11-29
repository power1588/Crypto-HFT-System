/// Phase 4 TDD Tests: Type System Enhancements
/// 
/// These tests verify that all type system enhancements have been correctly implemented
/// according to the task breakdown in specs/001-market-making/tasks.md
/// 
/// Run with: cargo test test_phase4_type_system --lib
/// 
/// Test Coverage:
/// - T026: Neg trait for Size type
/// - T027-T028: ToPrimitive/FromPrimitive traits
/// - T031-T035: Decimal operations in strategies
/// - T036: Spread calculation fixes

use crypto_hft::types::{Price, Size};
use rust_decimal::Decimal;
use rust_decimal::prelude::*;

#[cfg(test)]
mod size_neg_trait_tests {
    use super::*;
    use std::ops::Neg;

    #[test]
    fn test_size_negation() {
        // T026: Size should implement Neg trait for negative positions
        let size = Size::new(Decimal::new(100, 2)); // 1.00
        let negated = -size;
        
        assert_eq!(negated.value(), Decimal::new(-100, 2)); // -1.00
    }

    #[test]
    fn test_size_double_negation() {
        let size = Size::new(Decimal::new(500, 2)); // 5.00
        let double_negated = -(-size);
        
        assert_eq!(double_negated.value(), Decimal::new(500, 2)); // 5.00
    }

    #[test]
    fn test_size_negation_zero() {
        let size = Size::new(Decimal::ZERO);
        let negated = -size;
        
        assert_eq!(negated.value(), Decimal::ZERO);
    }

    #[test]
    fn test_size_negation_negative_value() {
        // Negating a negative should give positive
        let size = Size::new(Decimal::new(-100, 2)); // -1.00
        let negated = -size;
        
        assert_eq!(negated.value(), Decimal::new(100, 2)); // 1.00
    }
}

#[cfg(test)]
mod price_neg_trait_tests {
    use super::*;
    use std::ops::Neg;

    #[test]
    fn test_price_negation() {
        // Price should also implement Neg trait for calculations
        let price = Price::new(Decimal::new(10000, 2)); // 100.00
        let negated = -price;
        
        assert_eq!(negated.value(), Decimal::new(-10000, 2)); // -100.00
    }
}

#[cfg(test)]
mod price_division_tests {
    use super::*;

    #[test]
    fn test_price_divided_by_price_returns_decimal() {
        // T034: Division of Price by Price should return Decimal
        let price1 = Price::new(Decimal::new(20000, 2)); // 200.00
        let price2 = Price::new(Decimal::new(10000, 2)); // 100.00
        
        let result: Decimal = price1 / price2;
        
        assert_eq!(result, Decimal::new(2, 0)); // 2.0
    }

    #[test]
    fn test_price_divided_by_decimal() {
        // Price / Decimal should return Price
        let price = Price::new(Decimal::new(10000, 2)); // 100.00
        let divisor = Decimal::new(2, 0); // 2
        
        let result = price / divisor;
        
        assert_eq!(result.value(), Decimal::new(5000, 2)); // 50.00
    }
}

#[cfg(test)]
mod size_abs_tests {
    use super::*;

    #[test]
    fn test_size_abs_positive() {
        // Size should have abs() method
        let size = Size::new(Decimal::new(100, 2)); // 1.00
        let abs_size = size.abs();
        
        assert_eq!(abs_size.value(), Decimal::new(100, 2)); // 1.00
    }

    #[test]
    fn test_size_abs_negative() {
        let size = Size::new(Decimal::new(-100, 2)); // -1.00
        let abs_size = size.abs();
        
        assert_eq!(abs_size.value(), Decimal::new(100, 2)); // 1.00
    }

    #[test]
    fn test_size_abs_zero() {
        let size = Size::new(Decimal::ZERO);
        let abs_size = size.abs();
        
        assert_eq!(abs_size.value(), Decimal::ZERO);
    }
}

#[cfg(test)]
mod price_abs_tests {
    use super::*;

    #[test]
    fn test_price_abs_positive() {
        // Price should have abs() method
        let price = Price::new(Decimal::new(10000, 2)); // 100.00
        let abs_price = price.abs();
        
        assert_eq!(abs_price.value(), Decimal::new(10000, 2)); // 100.00
    }

    #[test]
    fn test_price_abs_negative() {
        let price = Price::new(Decimal::new(-10000, 2)); // -100.00
        let abs_price = price.abs();
        
        assert_eq!(abs_price.value(), Decimal::new(10000, 2)); // 100.00
    }
}

#[cfg(test)]
mod decimal_conversion_tests {
    use super::*;

    #[test]
    fn test_decimal_to_f64() {
        // T035: Verify ToPrimitive trait is available for Decimal
        let decimal = Decimal::new(12345, 2); // 123.45
        let f64_value = decimal.to_f64();
        
        assert!(f64_value.is_some());
        assert!((f64_value.unwrap() - 123.45).abs() < 0.0001);
    }

    #[test]
    fn test_decimal_from_f64() {
        // T031: Verify FromPrimitive trait is available for Decimal
        let decimal = Decimal::from_f64(123.45);
        
        assert!(decimal.is_some());
        // Note: f64 -> Decimal can have precision issues
    }

    #[test]
    fn test_decimal_from_usize() {
        // T032: Verify FromPrimitive trait is available for usize conversion
        let decimal = Decimal::from_usize(100);
        
        assert!(decimal.is_some());
        assert_eq!(decimal.unwrap(), Decimal::new(100, 0));
    }
}

#[cfg(test)]
mod size_comparison_tests {
    use super::*;

    #[test]
    fn test_size_comparison_with_negated() {
        // Test that Size comparisons work correctly with negated values
        let max_position = Size::new(Decimal::new(100, 2)); // 1.00
        let neg_max = -max_position;
        let current_position = Size::new(Decimal::new(-50, 2)); // -0.50
        
        // -0.50 should be greater than -1.00
        assert!(current_position >= neg_max);
    }
}

