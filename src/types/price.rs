use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// Price type using NewType pattern for type safety
/// Prevents accidental mixing with other numeric types like Size
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Price(pub Decimal);

impl Price {
    /// Create a new Price from a Decimal
    pub fn new(value: Decimal) -> Self {
        Self(value)
    }

    /// Get the underlying Decimal value
    pub fn value(&self) -> Decimal {
        self.0
    }

    /// Create a Price from a string
    pub fn from_str(s: &str) -> Result<Self, rust_decimal::Error> {
        let decimal = Decimal::from_str(s)?;
        Ok(Self(decimal))
    }
}

impl fmt::Display for Price {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Custom serialization to preserve decimal places
impl Serialize for Price {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as string to preserve precision
        serializer.serialize_str(&self.0.to_string())
    }
}

// Custom deserialization from string
impl<'de> Deserialize<'de> for Price {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let decimal = Decimal::from_str(&s).map_err(serde::de::Error::custom)?;
        Ok(Price(decimal))
    }
}

// Prevent arithmetic operations between Price and other types
impl std::ops::Add for Price {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl std::ops::Sub for Price {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

// Implement multiplication with scalar values
impl std::ops::Mul<Decimal> for Price {
    type Output = Self;

    fn mul(self, rhs: Decimal) -> Self {
        Self(self.0 * rhs)
    }
}

impl std::ops::Div<Decimal> for Price {
    type Output = Self;

    fn div(self, rhs: Decimal) -> Self {
        Self(self.0 / rhs)
    }
}

// Implement Price / Price -> Decimal (T034: for ratio calculations)
impl std::ops::Div<Price> for Price {
    type Output = Decimal;

    fn div(self, rhs: Price) -> Decimal {
        self.0 / rhs.0
    }
}

// Implement Neg trait for Price (needed for calculations)
impl std::ops::Neg for Price {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Price {
    /// Get the absolute value of the price
    pub fn abs(&self) -> Self {
        Self(self.0.abs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn test_price_creation() {
        let price = Price::new(Decimal::new(10050, 2)); // 100.50
        assert_eq!(price.value(), Decimal::new(10050, 2));
    }

    #[test]
    fn test_price_from_str() {
        let price = Price::from_str("100.50").unwrap();
        assert_eq!(price.value(), Decimal::new(10050, 2));
    }

    #[test]
    fn test_price_arithmetic() {
        let price1 = Price::new(Decimal::new(10050, 2)); // 100.50
        let price2 = Price::new(Decimal::new(50, 2)); // 0.50

        let sum = price1 + price2;
        assert_eq!(sum.value(), Decimal::new(10100, 2)); // 101.00

        let diff = price1 - price2;
        assert_eq!(diff.value(), Decimal::new(10000, 2)); // 100.00
    }

    #[test]
    fn test_price_multiplication() {
        let price = Price::new(Decimal::new(100, 0)); // 100
        let multiplier = Decimal::new(150, 2); // 1.5

        let result = price * multiplier;
        assert_eq!(result.value(), Decimal::new(150, 0)); // 150
    }

    #[test]
    fn test_price_serialization() {
        let price = Price::new(Decimal::new(10050, 2)); // 100.50

        // Test JSON serialization
        let json = serde_json::to_string(&price).unwrap();
        assert_eq!(json, "\"100.50\"");

        // Test JSON deserialization
        let deserialized: Price = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, price);
    }
}
