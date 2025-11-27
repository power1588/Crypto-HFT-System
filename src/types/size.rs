use rust_decimal::Decimal;
use serde::{Deserialize, Serialize, Serializer, Deserializer};
use std::fmt;
use std::str::FromStr;

/// Size type using NewType pattern for type safety
/// Represents quantity/amount and is distinct from Price
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Size(pub Decimal);

impl Size {
    /// Create a new Size from a Decimal
    pub fn new(value: Decimal) -> Self {
        Self(value)
    }

    /// Get the underlying Decimal value
    pub fn value(&self) -> Decimal {
        self.0
    }

    /// Create a Size from a string
    pub fn from_str(s: &str) -> Result<Self, rust_decimal::Error> {
        let decimal = Decimal::from_str(s)?;
        Ok(Self(decimal))
    }

    /// Check if the size is zero
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Custom serialization to preserve decimal places
impl Serialize for Size {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as string to preserve precision
        serializer.serialize_str(&self.0.to_string())
    }
}

// Custom deserialization from string
impl<'de> Deserialize<'de> for Size {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let decimal = Decimal::from_str(&s).map_err(serde::de::Error::custom)?;
        Ok(Size(decimal))
    }
}

// Prevent arithmetic operations between Size and other types
impl std::ops::Add for Size {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl std::ops::Sub for Size {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

// Implement multiplication with scalar values
impl std::ops::Mul<Decimal> for Size {
    type Output = Self;

    fn mul(self, rhs: Decimal) -> Self {
        Self(self.0 * rhs)
    }
}

impl std::ops::Div<Decimal> for Size {
    type Output = Self;

    fn div(self, rhs: Decimal) -> Self {
        Self(self.0 / rhs)
    }
}

// Allow multiplication between Price and Size to calculate total value
impl std::ops::Mul<crate::types::Price> for Size {
    type Output = Decimal;

    fn mul(self, rhs: crate::types::Price) -> Decimal {
        self.0 * rhs.0
    }
}

impl std::ops::Mul<crate::types::Size> for crate::types::Price {
    type Output = Decimal;

    fn mul(self, rhs: crate::types::Size) -> Decimal {
        self.0 * rhs.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Price;
    use rust_decimal::Decimal;

    #[test]
    fn test_size_creation() {
        let size = Size::new(Decimal::new(1500, 2)); // 15.00
        assert_eq!(size.value(), Decimal::new(1500, 2));
    }

    #[test]
    fn test_size_from_str() {
        let size = Size::from_str("15.00").unwrap();
        assert_eq!(size.value(), Decimal::new(1500, 2));
    }

    #[test]
    fn test_size_arithmetic() {
        let size1 = Size::new(Decimal::new(1000, 2)); // 10.00
        let size2 = Size::new(Decimal::new(500, 2));  // 5.00
        
        let sum = size1 + size2;
        assert_eq!(sum.value(), Decimal::new(1500, 2)); // 15.00
        
        let diff = size1 - size2;
        assert_eq!(diff.value(), Decimal::new(500, 2)); // 5.00
    }

    #[test]
    fn test_size_multiplication() {
        let size = Size::new(Decimal::new(1000, 2)); // 10.00
        let multiplier = Decimal::new(2, 0); // 2
        
        let result = size * multiplier;
        assert_eq!(result.value(), Decimal::new(2000, 2)); // 20.00
    }

    #[test]
    fn test_price_size_multiplication() {
        let price = Price::new(Decimal::new(10000, 2)); // 100.00
        let size = Size::new(Decimal::new(1500, 2));    // 15.00
        
        let total_value = size * price;
        assert_eq!(total_value, Decimal::new(150000, 2)); // 1500.00
        
        let total_value_alt = price * size;
        assert_eq!(total_value_alt, Decimal::new(150000, 2)); // 1500.00
    }

    #[test]
    fn test_size_serialization() {
        let size = Size::new(Decimal::new(1500, 2)); // 15.00
        
        // Test JSON serialization
        let json = serde_json::to_string(&size).unwrap();
        assert_eq!(json, "\"15.00\"");
        
        // Test JSON deserialization
        let deserialized: Size = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, size);
    }

    #[test]
    fn test_size_is_zero() {
        let zero_size = Size::new(Decimal::ZERO);
        assert!(zero_size.is_zero());
        
        let non_zero_size = Size::new(Decimal::new(100, 2)); // 1.00
        assert!(!non_zero_size.is_zero());
    }
}
