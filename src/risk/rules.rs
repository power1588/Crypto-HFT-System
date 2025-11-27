use crate::traits::{NewOrder, OrderId, ExecutionReport, OrderStatus};
use crate::types::{Price, Size};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Risk violation types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskViolation {
    /// Order size exceeds maximum limit
    ExceedsMaxSize {
        symbol: String,
        order_size: Size,
        max_size: Size,
    },
    /// Order value exceeds maximum limit
    ExceedsMaxValue {
        symbol: String,
        order_value: Size,
        max_value: Size,
    },
    /// Insufficient balance
    InsufficientBalance {
        asset: String,
        required: Size,
        available: Size,
    },
    /// Position size exceeds limit
    ExceedsPositionLimit {
        symbol: String,
        position_size: Size,
        max_position: Size,
    },
    /// Order rate limit exceeded
    RateLimitExceeded {
        symbol: String,
        retry_after: Option<u64>, // Timestamp when retry is allowed
    },
    /// Custom risk violation
    Custom {
        name: String,
        details: HashMap<String, String>,
    },
}

/// Risk rule trait
pub trait RiskRule {
    /// Check if an order violates this rule
    fn check_order(&self, order: &NewOrder) -> Option<RiskViolation>;
    
    /// Check if an execution report violates this rule
    fn check_execution(&self, report: &ExecutionReport) -> Option<RiskViolation>;
    
    /// Get rule name
    fn name(&self) -> &str;
}

/// Maximum order size rule
#[derive(Debug, Clone)]
pub struct MaxOrderSizeRule {
    max_sizes: HashMap<String, Size>,
}

impl MaxOrderSizeRule {
    /// Create a new maximum order size rule
    pub fn new() -> Self {
        Self {
            max_sizes: HashMap::new(),
        }
    }

    /// Set maximum size for a symbol
    pub fn set_max_size(&mut self, symbol: String, max_size: Size) {
        self.max_sizes.insert(symbol, max_size);
    }

    /// Get maximum size for a symbol
    pub fn get_max_size(&self, symbol: &str) -> Option<Size> {
        self.max_sizes.get(symbol).cloned()
    }
}

impl RiskRule for MaxOrderSizeRule {
    fn check_order(&self, order: &NewOrder) -> Option<RiskViolation> {
        if let Some(max_size) = self.get_max_size(&order.symbol) {
            if order.quantity > max_size {
                return Some(RiskViolation::ExceedsMaxSize {
                    symbol: order.symbol.clone(),
                    order_size: order.quantity,
                    max_size,
                });
            }
        }
        None
    }

    fn check_execution(&self, _report: &ExecutionReport) -> Option<RiskViolation> {
        // This rule only checks orders before execution
        None
    }

    fn name(&self) -> &str {
        "MaxOrderSize"
    }
}

/// Maximum order value rule
#[derive(Debug, Clone)]
pub struct MaxOrderValueRule {
    max_values: HashMap<String, Size>,
}

impl MaxOrderValueRule {
    /// Create a new maximum order value rule
    pub fn new() -> Self {
        Self {
            max_values: HashMap::new(),
        }
    }

    /// Set maximum value for a symbol
    pub fn set_max_value(&mut self, symbol: String, max_value: Size) {
        self.max_values.insert(symbol, max_value);
    }

    /// Get maximum value for a symbol
    pub fn get_max_value(&self, symbol: &str) -> Option<Size> {
        self.max_values.get(symbol).cloned()
    }
}

impl RiskRule for MaxOrderValueRule {
    fn check_order(&self, order: &NewOrder) -> Option<RiskViolation> {
        if let (Some(quantity), Some(price)) = (order.quantity, order.price) {
            if let Some(max_value) = self.get_max_value(&order.symbol) {
                let order_value = quantity * price;
                if order_value > max_value {
                    return Some(RiskViolation::ExceedsMaxValue {
                        symbol: order.symbol.clone(),
                        order_value,
                        max_value,
                    });
                }
            }
        }
        None
    }

    fn check_execution(&self, _report: &ExecutionReport) -> Option<RiskViolation> {
        // This rule only checks orders before execution
        None
    }

    fn name(&self) -> &str {
        "MaxOrderValue"
    }
}

/// Risk engine that applies multiple risk rules
#[derive(Debug)]
pub struct RiskEngine {
    rules: Vec<Box<dyn RiskRule>>,
}

impl RiskEngine {
    /// Create a new risk engine
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
        }
    }

    /// Add a risk rule
    pub fn add_rule(&mut self, rule: Box<dyn RiskRule>) {
        self.rules.push(rule);
    }

    /// Check if an order violates any risk rules
    pub fn check_order(&self, order: &NewOrder) -> Option<RiskViolation> {
        for rule in &self.rules {
            if let Some(violation) = rule.check_order(order) {
                return Some(violation);
            }
        }
        None
    }

    /// Check if an execution report violates any risk rules
    pub fn check_execution(&self, report: &ExecutionReport) -> Option<RiskViolation> {
        for rule in &self.rules {
            if let Some(violation) = rule.check_execution(report) {
                return Some(violation);
            }
        }
        None
    }

    /// Get all rule names
    pub fn get_rule_names(&self) -> Vec<&str> {
        self.rules.iter().map(|rule| rule.name()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Price, Size};

    #[test]
    fn test_max_order_size_rule() {
        let mut rule = MaxOrderSizeRule::new();
        rule.set_max_size("BTCUSDT".to_string(), Size::from_str("1.0").unwrap());
        
        // Test order within limit
        let valid_order = NewOrder::new_market_buy(
            "BTCUSDT".to_string(),
            Size::from_str("0.5").unwrap()
        );
        assert!(rule.check_order(&valid_order).is_none());
        
        // Test order exceeding limit
        let invalid_order = NewOrder::new_market_buy(
            "BTCUSDT".to_string(),
            Size::from_str("1.5").unwrap()
        );
        
        if let Some(RiskViolation::ExceedsMaxSize { symbol, order_size, max_size }) = rule.check_order(&invalid_order) {
            assert_eq!(symbol, "BTCUSDT");
            assert_eq!(order_size, Size::from_str("1.5").unwrap());
            assert_eq!(max_size, Size::from_str("1.0").unwrap());
        } else {
            panic!("Expected ExceedsMaxSize violation");
        }
    }

    #[test]
    fn test_max_order_value_rule() {
        let mut rule = MaxOrderValueRule::new();
        rule.set_max_value("BTCUSDT".to_string(), Size::from_str("10000.0").unwrap());
        
        // Test order within limit
        let valid_order = NewOrder::new_limit_buy(
            "BTCUSDT".to_string(),
            Size::from_str("0.1").unwrap(),
            Some(Price::from_str("100000.0").unwrap()),
            crate::traits::TimeInForce::GTC,
        );
        assert!(rule.check_order(&valid_order).is_none());
        
        // Test order exceeding limit
        let invalid_order = NewOrder::new_limit_buy(
            "BTCUSDT".to_string(),
            Size::from_str("0.2").unwrap(),
            Some(Price::from_str("100000.0").unwrap()),
            crate::traits::TimeInForce::GTC,
        );
        
        if let Some(RiskViolation::ExceedsMaxValue { symbol, order_value, max_value }) = rule.check_order(&invalid_order) {
            assert_eq!(symbol, "BTCUSDT");
            assert_eq!(order_value, Size::from_str("20000.0").unwrap());
            assert_eq!(max_value, Size::from_str("10000.0").unwrap());
        } else {
            panic!("Expected ExceedsMaxValue violation");
        }
    }

    #[test]
    fn test_risk_engine() {
        let mut engine = RiskEngine::new();
        
        // Add rules
        let size_rule = MaxOrderSizeRule::new();
        size_rule.set_max_size("BTCUSDT".to_string(), Size::from_str("1.0").unwrap());
        
        let value_rule = MaxOrderValueRule::new();
        value_rule.set_max_value("BTCUSDT".to_string(), Size::from_str("10000.0").unwrap());
        
        engine.add_rule(Box::new(size_rule));
        engine.add_rule(Box::new(value_rule));
        
        // Test order within all limits
        let valid_order = NewOrder::new_market_buy(
            "BTCUSDT".to_string(),
            Size::from_str("0.5").unwrap()
        );
        assert!(engine.check_order(&valid_order).is_none());
        
        // Test order exceeding size limit
        let large_order = NewOrder::new_market_buy(
            "BTCUSDT".to_string(),
            Size::from_str("1.5").unwrap()
        );
        assert!(engine.check_order(&large_order).is_some());
        
        // Test order exceeding value limit
        let valuable_order = NewOrder::new_limit_buy(
            "BTCUSDT".to_string(),
            Size::from_str("0.2").unwrap(),
            Some(Price::from_str("100000.0").unwrap()),
            crate::traits::TimeInForce::GTC,
        );
        assert!(engine.check_order(&valuable_order).is_some());
        
        // Check rule names
        let rule_names = engine.get_rule_names();
        assert_eq!(rule_names.len(), 2);
        assert!(rule_names.contains(&"MaxOrderSize"));
        assert!(rule_names.contains(&"MaxOrderValue"));
    }
}
