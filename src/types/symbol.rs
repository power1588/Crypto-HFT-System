use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Index;

/// Symbol type representing a trading pair (e.g., "BTCUSDT")
/// Uses NewType pattern for type safety
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Symbol(String);

impl Symbol {
    /// Create a new Symbol from a string
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Get the underlying string value
    pub fn value(&self) -> &str {
        &self.0
    }

    /// Get the underlying string as &str (alias for value())
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the length of the symbol string
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if symbol is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Check if symbol is valid (basic validation)
    pub fn is_valid(&self) -> bool {
        !self.0.is_empty() && self.0.len() >= 3 && self.0.len() <= 20
    }
}

/// Implement Index trait to support slicing operations like &symbol[0..3]
impl Index<std::ops::Range<usize>> for Symbol {
    type Output = str;

    fn index(&self, index: std::ops::Range<usize>) -> &Self::Output {
        &self.0[index]
    }
}

/// Implement Index trait for RangeFrom (e.g., &symbol[3..])
impl Index<std::ops::RangeFrom<usize>> for Symbol {
    type Output = str;

    fn index(&self, index: std::ops::RangeFrom<usize>) -> &Self::Output {
        &self.0[index]
    }
}

/// Implement Index trait for RangeTo (e.g., &symbol[..3])
impl Index<std::ops::RangeTo<usize>> for Symbol {
    type Output = str;

    fn index(&self, index: std::ops::RangeTo<usize>) -> &Self::Output {
        &self.0[index]
    }
}

/// Implement Index trait for RangeFull (e.g., &symbol[..])
impl Index<std::ops::RangeFull> for Symbol {
    type Output = str;

    fn index(&self, index: std::ops::RangeFull) -> &Self::Output {
        &self.0[index]
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for Symbol {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for Symbol {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<Symbol> for String {
    fn from(s: Symbol) -> String {
        s.0
    }
}

impl AsRef<str> for Symbol {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<str> for Symbol {
    fn borrow(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_creation() {
        let symbol = Symbol::new("BTCUSDT");
        assert_eq!(symbol.value(), "BTCUSDT");
    }

    #[test]
    fn test_symbol_from_str() {
        let symbol = Symbol::from("ETHUSDT");
        assert_eq!(symbol.value(), "ETHUSDT");
    }

    #[test]
    fn test_symbol_validation() {
        let valid_symbol = Symbol::new("BTCUSDT");
        assert!(valid_symbol.is_valid());

        let invalid_symbol = Symbol::new("");
        assert!(!invalid_symbol.is_valid());

        // Symbol must be > 20 chars to be invalid (is_valid allows len <= 20)
        let too_long_symbol = Symbol::new("VERYLONGSYMBOLNAMEEXCEEDS20");
        assert!(!too_long_symbol.is_valid());
    }

    #[test]
    fn test_symbol_display() {
        let symbol = Symbol::new("BTCUSDT");
        assert_eq!(format!("{}", symbol), "BTCUSDT");
    }

    #[test]
    fn test_symbol_serialization() {
        let symbol = Symbol::new("BTCUSDT");

        // Test JSON serialization
        let json = serde_json::to_string(&symbol).unwrap();
        assert_eq!(json, "\"BTCUSDT\"");

        // Test JSON deserialization
        let deserialized: Symbol = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, symbol);
    }

    #[test]
    fn test_symbol_len() {
        let symbol = Symbol::new("BTCUSDT");
        assert_eq!(symbol.len(), 7);

        let empty = Symbol::new("");
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_symbol_as_str() {
        let symbol = Symbol::new("ETHUSDT");
        assert_eq!(symbol.as_str(), "ETHUSDT");
        assert_eq!(symbol.as_str(), symbol.value());
    }

    #[test]
    fn test_symbol_indexing() {
        let symbol = Symbol::new("BTCUSDT");

        // Test various slicing operations
        assert_eq!(&symbol[0..3], "BTC");
        assert_eq!(&symbol[3..7], "USDT");
        assert_eq!(&symbol[..3], "BTC");
        assert_eq!(&symbol[3..], "USDT");
        assert_eq!(&symbol[..], "BTCUSDT");
    }

    #[test]
    fn test_symbol_slice_for_asset_extraction() {
        // This pattern is used in risk/rules.rs
        let symbol = Symbol::new("BTCUSDT");
        let len = symbol.len();

        if len >= 4 {
            let base_asset = &symbol[..len - 4]; // Remove "USDT" suffix
            assert_eq!(base_asset, "BTC");
        }
    }
}
