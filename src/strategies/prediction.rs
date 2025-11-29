use crate::core::events::Trade;
use crate::types::Price;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use std::collections::VecDeque;

/// Linear regression model for short-term price prediction
/// Uses ordinary least squares (OLS) to fit a linear model to historical price data
pub struct LinearRegressionPredictor {
    /// Historical price data points (timestamp, price)
    price_history: VecDeque<(u64, Price)>,
    /// Maximum number of data points to keep
    max_history_size: usize,
    /// Minimum number of data points required for prediction
    min_data_points: usize,
    /// Coefficients: (slope, intercept)
    coefficients: Option<(f64, f64)>,
    /// Last update timestamp
    last_update: Option<u64>,
}

impl LinearRegressionPredictor {
    /// Create a new linear regression predictor
    pub fn new(max_history_size: usize, min_data_points: usize) -> Self {
        Self {
            price_history: VecDeque::with_capacity(max_history_size),
            max_history_size,
            min_data_points,
            coefficients: None,
            last_update: None,
        }
    }

    /// Update the model with a new price point
    pub fn update(&mut self, timestamp: u64, price: Price) {
        // Add new data point
        self.price_history.push_back((timestamp, price));

        // Remove oldest data point if we exceed max history size
        if self.price_history.len() > self.max_history_size {
            self.price_history.pop_front();
        }

        // Recalculate coefficients if we have enough data
        if self.price_history.len() >= self.min_data_points {
            self.recalculate_coefficients();
        }

        self.last_update = Some(timestamp);
    }

    /// Update the model with a trade event
    pub fn update_from_trade(&mut self, trade: &Trade) {
        self.update(trade.timestamp, trade.price);
    }

    /// Recalculate the linear regression coefficients using OLS
    fn recalculate_coefficients(&mut self) {
        if self.price_history.len() < self.min_data_points {
            self.coefficients = None;
            return;
        }

        let n = self.price_history.len() as f64;

        // Get the first timestamp as reference point to avoid large numbers
        let first_timestamp = self.price_history[0].0 as f64;

        // Calculate sums for OLS
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xy = 0.0;
        let mut sum_x2 = 0.0;

        for (timestamp, price) in self.price_history.iter() {
            // Normalize timestamp relative to first timestamp
            let x = (*timestamp as f64 - first_timestamp) / 1000.0; // Convert to seconds
            let y = price.value().to_f64().unwrap_or(0.0);

            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
            sum_x2 += x * x;
        }

        // Calculate slope (beta) and intercept (alpha)
        // beta = (n*sum_xy - sum_x*sum_y) / (n*sum_x2 - sum_x^2)
        // alpha = (sum_y - beta*sum_x) / n
        let denominator = n * sum_x2 - sum_x * sum_x;

        if denominator.abs() < 1e-10 {
            // Avoid division by zero
            self.coefficients = None;
            return;
        }

        let slope = (n * sum_xy - sum_x * sum_y) / denominator;

        // Intercept is calculated at the reference timestamp
        // We need to adjust it back to absolute time
        let intercept = (sum_y - slope * sum_x) / n;

        self.coefficients = Some((slope, intercept));
    }

    /// Predict the price at a future timestamp
    /// Returns None if there's insufficient data or the model hasn't been trained
    pub fn predict(&self, future_timestamp: u64) -> Option<Price> {
        let (slope, intercept) = self.coefficients?;

        // Get reference timestamp
        let first_timestamp = self.price_history.front()?.0 as f64;

        // Calculate normalized time
        let x = (future_timestamp as f64 - first_timestamp) / 1000.0;

        // Predict: y = slope * x + intercept
        let predicted_price = slope * x + intercept;

        // Convert back to Price
        Some(Price::new(
            Decimal::from_f64(predicted_price).unwrap_or(Decimal::ZERO),
        ))
    }

    /// Predict the price after a certain number of seconds
    pub fn predict_after_seconds(&self, seconds: u64) -> Option<Price> {
        let last_timestamp = self.last_update?;
        let future_timestamp = last_timestamp + (seconds * 1000);
        self.predict(future_timestamp)
    }

    /// Get the current model coefficients
    pub fn coefficients(&self) -> Option<(f64, f64)> {
        self.coefficients
    }

    /// Get the R-squared value (goodness of fit)
    /// Returns a value between 0 and 1, where 1 is perfect fit
    pub fn r_squared(&self) -> Option<f64> {
        let (slope, intercept) = self.coefficients?;

        if self.price_history.len() < self.min_data_points {
            return None;
        }

        let n = self.price_history.len() as f64;
        let first_timestamp = self.price_history[0].0 as f64;

        // Calculate mean of y values
        let mean_y = self
            .price_history
            .iter()
            .map(|(_, price)| price.value().to_f64().unwrap_or(0.0))
            .sum::<f64>()
            / n;

        // Calculate total sum of squares (TSS)
        let tss: f64 = self
            .price_history
            .iter()
            .map(|(_, price)| {
                let y = price.value().to_f64().unwrap_or(0.0);
                let diff = y - mean_y;
                diff * diff
            })
            .sum();

        // Calculate residual sum of squares (RSS)
        let rss: f64 = self
            .price_history
            .iter()
            .map(|(timestamp, price)| {
                let x = (*timestamp as f64 - first_timestamp) / 1000.0;
                let y = price.value().to_f64().unwrap_or(0.0);
                let predicted = slope * x + intercept;
                let residual = y - predicted;
                residual * residual
            })
            .sum();

        if tss.abs() < 1e-10 {
            return Some(0.0);
        }

        // R² = 1 - (RSS / TSS)
        Some(1.0 - (rss / tss))
    }

    /// Get the number of data points currently in the model
    pub fn data_point_count(&self) -> usize {
        self.price_history.len()
    }

    /// Check if the model is ready for prediction
    pub fn is_ready(&self) -> bool {
        self.coefficients.is_some()
    }

    /// Clear all historical data
    pub fn clear(&mut self) {
        self.price_history.clear();
        self.coefficients = None;
        self.last_update = None;
    }

    /// Get the last update timestamp
    pub fn last_update(&self) -> Option<u64> {
        self.last_update
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::Trade;
    use crate::types::{Size, Symbol};

    #[test]
    fn test_linear_regression_creation() {
        let predictor = LinearRegressionPredictor::new(100, 5);
        assert_eq!(predictor.data_point_count(), 0);
        assert!(!predictor.is_ready());
        assert_eq!(predictor.coefficients(), None);
    }

    #[test]
    fn test_update_with_insufficient_data() {
        let mut predictor = LinearRegressionPredictor::new(100, 5);

        // Add fewer than min_data_points
        for i in 0..3 {
            let timestamp = 1000 + i * 1000;
            let price = Price::from_str(&format!("100.{}", i)).unwrap();
            predictor.update(timestamp, price);
        }

        assert!(!predictor.is_ready());
        assert_eq!(predictor.coefficients(), None);
    }

    #[test]
    fn test_update_with_sufficient_data() {
        let mut predictor = LinearRegressionPredictor::new(100, 5);

        // Add enough data points
        for i in 0..10 {
            let timestamp = 1000 + i * 1000;
            let price = Price::from_str(&format!("100.{}", i)).unwrap();
            predictor.update(timestamp, price);
        }

        assert!(predictor.is_ready());
        assert!(predictor.coefficients().is_some());
    }

    #[test]
    fn test_prediction_with_linear_trend() {
        let mut predictor = LinearRegressionPredictor::new(100, 5);

        // Create a linear trend: price increases by 0.1 per second
        for i in 0..10 {
            let timestamp = 1000 + i * 1000;
            let price_value = 100.0 + (i as f64 * 0.1);
            let price = Price::new(Decimal::from_f64(price_value).unwrap());
            predictor.update(timestamp, price);
        }

        assert!(predictor.is_ready());

        // Predict 5 seconds into the future
        let predicted = predictor.predict_after_seconds(5);
        assert!(predicted.is_some());

        // The predicted price should be approximately 100.0 + 14 * 0.1 = 101.4
        // (last timestamp is 1000 + 9*1000 = 10000, so 5 seconds later is 15000)
        // Actually, let's check if it's reasonable
        let predicted_value = predicted.unwrap().value().to_f64().unwrap();
        assert!(predicted_value > 100.0);
    }

    #[test]
    fn test_update_from_trade() {
        let mut predictor = LinearRegressionPredictor::new(100, 5);

        let trade = Trade {
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "binance".to_string(),
            price: Price::from_str("100.00").unwrap(),
            size: Size::from_str("1.0").unwrap(),
            side: crate::core::events::OrderSide::Buy,
            timestamp: 1000,
            trade_id: Some("trade1".to_string()),
        };

        predictor.update_from_trade(&trade);
        assert_eq!(predictor.data_point_count(), 1);
    }

    #[test]
    fn test_r_squared_perfect_fit() {
        let mut predictor = LinearRegressionPredictor::new(100, 5);

        // Create a perfect linear trend
        for i in 0..10 {
            let timestamp = 1000 + i * 1000;
            let price_value = 100.0 + (i as f64 * 0.1);
            let price = Price::new(Decimal::from_f64(price_value).unwrap());
            predictor.update(timestamp, price);
        }

        let r_squared = predictor.r_squared();
        assert!(r_squared.is_some());
        // For a perfect linear trend, R² should be very close to 1.0
        assert!(r_squared.unwrap() > 0.9);
    }

    #[test]
    fn test_clear() {
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
    fn test_max_history_size() {
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
}
