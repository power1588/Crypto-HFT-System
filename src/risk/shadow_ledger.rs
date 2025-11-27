use crate::types::{Price, Size};
use crate::traits::{ExecutionReport, OrderStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rust_decimal::Decimal;

/// Inventory tracking for trading assets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    /// Asset balances
    balances: HashMap<String, Balance>,
    /// Frozen balances (locked in orders)
    frozen: HashMap<String, Size>,
    /// Total position value
    total_value: HashMap<String, Size>,
}

/// Asset balance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    /// Asset symbol
    pub asset: String,
    /// Available balance
    pub available: Size,
    /// Locked balance
    pub locked: Size,
}

impl Balance {
    /// Create a new balance
    pub fn new(asset: String, available: Size, locked: Size) -> Self {
        Self {
            asset,
            available,
            locked,
        }
    }

    /// Get total balance
    pub fn total(&self) -> Size {
        self.available + self.locked
    }
}

impl Inventory {
    /// Create a new inventory
    pub fn new() -> Self {
        Self {
            balances: HashMap::new(),
            frozen: HashMap::new(),
            total_value: HashMap::new(),
        }
    }

    /// Get balance for an asset
    pub fn get_balance(&self, asset: &str) -> Option<&Balance> {
        self.balances.get(asset)
    }

    /// Get frozen balance for an asset
    pub fn get_frozen(&self, asset: &str) -> Size {
        self.frozen.get(asset).cloned().unwrap_or(Size::new(crate::rust_decimal::Decimal::ZERO))
    }

    /// Get total value for an asset
    pub fn get_total_value(&self, asset: &str) -> Size {
        self.total_value.get(asset).cloned().unwrap_or(Size::new(crate::rust_decimal::Decimal::ZERO))
    }

    /// Update balance
    pub fn update_balance(&mut self, asset: String, balance: Balance) {
        self.balances.insert(asset, balance);
    }

    /// Freeze balance for an order
    pub fn freeze_balance(&mut self, asset: &str, amount: Size) {
        let balance = self.balances
            .entry(asset.to_string())
            .or_insert_with(|| Balance::new(asset.to_string(), Size::new(crate::rust_decimal::Decimal::ZERO), Size::new(crate::rust_decimal::Decimal::ZERO)));
        
        // Check if enough balance is available
        if balance.available >= amount {
            balance.available -= amount;
            balance.locked += amount;
        } else {
            // Not enough balance, freeze all available
            self.frozen.insert(asset.to_string(), balance.available);
            balance.available = Size::new(crate::rust_decimal::Decimal::ZERO);
        }
    }

    /// Unfreeze balance (when order is filled or canceled)
    pub fn unfreeze_balance(&mut self, asset: &str, amount: Size) {
        let frozen = self.frozen.get(asset).cloned().unwrap_or(Size::new(crate::rust_decimal::Decimal::ZERO));
        
        // Return frozen balance to available
        let balance = self.balances
            .entry(asset.to_string())
            .or_insert_with(|| Balance::new(asset.to_string(), Size::new(crate::rust_decimal::Decimal::ZERO), Size::new(crate::rust_decimal::Decimal::ZERO)));
        
        balance.available += frozen;
        balance.locked -= frozen;
        
        // Remove from frozen map
        self.frozen.remove(asset);
    }

    /// Update total value for an asset
    pub fn update_total_value(&mut self, asset: &str, value: Size) {
        self.total_value.insert(asset.to_string(), value);
    }

    /// Process execution report
    pub fn process_execution(&mut self, report: &ExecutionReport) {
        match report.status {
            OrderStatus::New => {
                // Order was just placed, freeze the required balance
                if let Some(price) = report.price {
                    let order_value = report.quantity * price;
                    self.freeze_balance(&report.symbol, order_value);
                }
            }
            OrderStatus::Filled => {
                // Order was filled, unfreeze the balance
                if let Some(price) = report.price {
                    let order_value = report.quantity * price;
                    self.unfreeze_balance(&report.symbol, order_value);
                }
                
                // Update total value based on execution
                if let Some(price) = report.price {
                    let execution_value = report.quantity * price;
                    self.update_total_value(&report.symbol, execution_value);
                }
            }
            OrderStatus::Canceled => {
                // Order was canceled, unfreeze the balance
                if let Some(price) = report.price {
                    let order_value = report.quantity * price;
                    self.unfreeze_balance(&report.symbol, order_value);
                }
            }
            OrderStatus::PartiallyFilled { filled_size, .. } => {
                // Partial fill, unfreeze the filled portion
                if let Some(price) = report.price {
                    let filled_value = filled_size * price;
                    self.unfreeze_balance(&report.symbol, filled_value);
                }
            }
            _ => {
                // Other statuses, no action needed
            }
        }
    }

    /// Get total portfolio value
    pub fn get_total_portfolio_value(&self) -> Size {
        let mut total = Size::new(crate::rust_decimal::Decimal::ZERO);
        for value in self.total_value.values() {
            total = total + value;
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Price, Size};

    #[test]
    fn test_inventory_balance_management() {
        let mut inventory = Inventory::new();
        
        // Initially, no balance
        assert!(inventory.get_balance("BTC").is_none());
        assert_eq!(inventory.get_frozen("BTC"), Size::new(crate::rust_decimal::Decimal::ZERO));
        
        // Update balance
        inventory.update_balance(
            "BTC".to_string(),
            Balance::new("BTC".to_string(), Size::from_str("10.0").unwrap(), Size::new(crate::rust_decimal::Decimal::ZERO))
        );
        
        // Check balance was updated
        assert_eq!(inventory.get_balance("BTC").unwrap().available, Size::from_str("10.0").unwrap());
        assert_eq!(inventory.get_balance("BTC").unwrap().locked, Size::new(crate::rust_decimal::Decimal::ZERO));
        
        // Freeze some balance
        inventory.freeze_balance("BTC", Size::from_str("5.0").unwrap());
        
        // Check frozen balance
        assert_eq!(inventory.get_frozen("BTC"), Size::from_str("5.0").unwrap());
        assert_eq!(inventory.get_balance("BTC").unwrap().available, Size::from_str("5.0").unwrap());
        
        // Unfreeze balance
        inventory.unfreeze_balance("BTC", Size::from_str("2.0").unwrap());
        
        // Check unfreeze
        assert_eq!(inventory.get_frozen("BTC"), Size::from_str("3.0").unwrap());
        assert_eq!(inventory.get_balance("BTC").unwrap().available, Size::from_str("7.0").unwrap());
    }

    #[test]
    fn test_insufficient_balance() {
        let mut inventory = Inventory::new();
        
        // Set initial balance
        inventory.update_balance(
            "BTC".to_string(),
            Balance::new("BTC".to_string(), Size::from_str("10.0").unwrap(), Size::new(crate::rust_decimal::Decimal::ZERO))
        );
        
        // Try to freeze more than available
        inventory.freeze_balance("BTC", Size::from_str("15.0").unwrap());
        
        // Check that only available was frozen
        assert_eq!(inventory.get_frozen("BTC"), Size::from_str("10.0").unwrap());
        assert_eq!(inventory.get_balance("BTC").unwrap().available, Size::new(crate::rust_decimal::Decimal::ZERO));
    }

    #[test]
    fn test_total_value_tracking() {
        let mut inventory = Inventory::new();
        
        // Update total value
        inventory.update_total_value("BTC".to_string(), Size::from_str("1000.0").unwrap());
        
        // Check total value
        assert_eq!(inventory.get_total_value("BTC"), Some(Size::from_str("1000.0").unwrap()));
        
        // Update with execution
        let report = ExecutionReport {
            order_id: crate::traits::OrderId::new("test_order".to_string()),
            client_order_id: None,
            symbol: "BTCUSDT".to_string(),
            status: OrderStatus::Filled,
            side: crate::traits::OrderSide::Buy,
            order_type: crate::traits::OrderType::Market,
            time_in_force: crate::traits::TimeInForce::IOC,
            quantity: Size::from_str("1.0").unwrap(),
            price: Some(Price::from_str("50000.0").unwrap()),
            timestamp: 123456789,
        };
        
        inventory.process_execution(&report);
        
        // Check total value was updated
        assert_eq!(inventory.get_total_value("BTC"), Some(Size::from_str("50000.0").unwrap()));
    }
}
