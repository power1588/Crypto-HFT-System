use crypto_hft::strategies::prediction::LinearRegressionPredictor;
use crypto_hft::types::{Price, Size};
use crypto_hft::core::events::{Trade, OrderSide};
use crypto_hft::types::Symbol;

#[test]
fn test_predictor_creation() {
    let predictor = LinearRegressionPredictor::new(100, 10);
    assert_eq!(predictor.data_point_count(), 0);
    assert!(!predictor.is_ready());
    assert_eq!(predictor.coefficients(), None);
}

#[test]
fn test_predictor_update() {
    let mut predictor = LinearRegressionPredictor::new(100, 5);
    
    // Add data points
    for i in 0..10 {
        let timestamp = 1000 + i * 1000;
        let price_value = 100.0 + (i as f64 * 0.1);
        let price = Price::new(crate::rust_decimal::Decimal::from_f64(price_value).unwrap());
        predictor.update(timestamp, price);
    }
    
    assert_eq!(predictor.data_point_count(), 10);
    assert!(predictor.is_ready());
    assert!(predictor.coefficients().is_some());
}

#[test]
fn test_predictor_update_from_trade() {
    let mut predictor = LinearRegressionPredictor::new(100, 5);
    
    let trade = Trade {
        symbol: Symbol::new("BTCUSDT"),
        exchange_id: "binance".to_string(),
        price: Price::from_str("100.00").unwrap(),
        size: Size::from_str("1.0").unwrap(),
        side: OrderSide::Buy,
        timestamp: 1000,
        trade_id: Some("trade1".to_string()),
    };
    
    predictor.update_from_trade(&trade);
    assert_eq!(predictor.data_point_count(), 1);
}

#[test]
fn test_predictor_insufficient_data() {
    let mut predictor = LinearRegressionPredictor::new(100, 10);
    
    // Add fewer than min_data_points
    for i in 0..5 {
        let timestamp = 1000 + i * 1000;
        let price = Price::from_str(&format!("100.{}", i)).unwrap();
        predictor.update(timestamp, price);
    }
    
    assert!(!predictor.is_ready());
    assert_eq!(predictor.coefficients(), None);
    assert_eq!(predictor.predict(5000), None);
}

#[test]
fn test_predictor_prediction() {
    let mut predictor = LinearRegressionPredictor::new(100, 5);
    
    // Create a linear trend: price increases by 0.1 per second
    for i in 0..10 {
        let timestamp = 1000 + i * 1000;
        let price_value = 100.0 + (i as f64 * 0.1);
        let price = Price::new(crate::rust_decimal::Decimal::from_f64(price_value).unwrap());
        predictor.update(timestamp, price);
    }
    
    assert!(predictor.is_ready());
    
    // Predict 5 seconds into the future
    let predicted = predictor.predict_after_seconds(5);
    assert!(predicted.is_some());
    
        let predicted_value = predicted.unwrap().value().to_f64().unwrap();
        assert!(predicted_value > 100.0);
}

#[test]
fn test_predictor_r_squared() {
    let mut predictor = LinearRegressionPredictor::new(100, 5);
    
    // Create a perfect linear trend
    for i in 0..10 {
        let timestamp = 1000 + i * 1000;
        let price_value = 100.0 + (i as f64 * 0.1);
        let price = Price::new(crate::rust_decimal::Decimal::from_f64(price_value).unwrap());
        predictor.update(timestamp, price);
    }
    
    let r_squared = predictor.r_squared();
    assert!(r_squared.is_some());
    // For a perfect linear trend, R² should be very close to 1.0
    assert!(r_squared.unwrap() > 0.9);
}

#[test]
fn test_predictor_clear() {
    let mut predictor = LinearRegressionPredictor::new(100, 5);
    
    // Add some data
    for i in 0..10 {
        let timestamp = 1000 + i * 1000;
        let price = Price::from_str(&format!("100.{}", i)).unwrap();
        predictor.update(timestamp, price);
    }
    
    assert!(predictor.is_ready());
    
    // Clear
    predictor.clear();
    
    assert!(!predictor.is_ready());
    assert_eq!(predictor.data_point_count(), 0);
    assert_eq!(predictor.coefficients(), None);
}

#[test]
fn test_predictor_max_history() {
    let mut predictor = LinearRegressionPredictor::new(5, 3);
    
    // Add more than max_history_size data points
    for i in 0..10 {
        let timestamp = 1000 + i * 1000;
        let price = Price::from_str(&format!("100.{}", i)).unwrap();
        predictor.update(timestamp, price);
    }
    
    // Should only keep the last 5 data points
    assert_eq!(predictor.data_point_count(), 5);
}

#[test]
fn test_predictor_with_noisy_data() {
    let mut predictor = LinearRegressionPredictor::new(100, 5);
    
    // Create data with some noise but overall upward trend
    for i in 0..20 {
        let timestamp = 1000 + i * 1000;
        let base_price = 100.0 + (i as f64 * 0.1);
        let noise = (i % 3) as f64 * 0.01 - 0.01; // Small noise
        let price_value = base_price + noise;
        let price = Price::new(crate::rust_decimal::Decimal::from_f64(price_value).unwrap());
        predictor.update(timestamp, price);
    }
    
    assert!(predictor.is_ready());
    
    // R² should still be reasonable for a trend with noise
    let r_squared = predictor.r_squared();
    assert!(r_squared.is_some());
    assert!(r_squared.unwrap() > 0.5);
}

#[test]
fn test_predictor_last_update() {
    let mut predictor = LinearRegressionPredictor::new(100, 5);
    
    assert_eq!(predictor.last_update(), None);
    
    let timestamp = 1000;
    let price = Price::from_str("100.00").unwrap();
    predictor.update(timestamp, price);
    
    assert_eq!(predictor.last_update(), Some(timestamp));
}

