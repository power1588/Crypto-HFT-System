use crypto_hft::types::{Price, Size};
use std::str::FromStr;

#[test]
fn test_price_zero() {
    assert!(Price::from_str("0").is_err());
}

#[test]
fn test_price_negative() {
    assert!(Price::from_str("-1.0").is_err());
}

#[test]
fn test_price_very_small() {
    let price = Price::from_str("0.00000001").unwrap();
    assert_eq!(price.to_string(), "0.00000001");
}

#[test]
fn test_price_very_large() {
    let price = Price::from_str("999999999.99").unwrap();
    assert_eq!(price.to_string(), "999999999.99");
}

#[test]
fn test_price_invalid_format() {
    assert!(Price::from_str("abc").is_err());
    assert!(Price::from_str("").is_err());
    assert!(Price::from_str("1.2.3").is_err());
}

#[test]
fn test_size_zero() {
    assert!(Size::from_str("0").is_err());
}

#[test]
fn test_size_negative() {
    assert!(Size::from_str("-1.0").is_err());
}

#[test]
fn test_size_very_small() {
    let size = Size::from_str("0.00000001").unwrap();
    assert_eq!(size.to_string(), "0.00000001");
}

#[test]
fn test_size_very_large() {
    let size = Size::from_str("999999999.99").unwrap();
    assert_eq!(size.to_string(), "999999999.99");
}

#[test]
fn test_size_invalid_format() {
    assert!(Size::from_str("abc").is_err());
    assert!(Size::from_str("").is_err());
    assert!(Size::from_str("1.2.3").is_err());
}

#[test]
fn test_price_precision() {
    let p1 = Price::from_str("100.123456789").unwrap();
    let p2 = Price::from_str("100.123456789").unwrap();
    assert_eq!(p1, p2);
}

#[test]
fn test_size_precision() {
    let s1 = Size::from_str("100.123456789").unwrap();
    let s2 = Size::from_str("100.123456789").unwrap();
    assert_eq!(s1, s2);
}

