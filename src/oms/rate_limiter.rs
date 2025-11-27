use crate::traits::{OrderManager, OrderManager::Error};
use std::time::{Duration, Instant};
use std::collections::HashMap;

/// Rate limiter that controls order submission rate
pub struct RateLimiter {
    /// Maximum number of orders per time window
    max_orders_per_window: usize,
    /// Time window duration
    window_duration: Duration,
    /// Order history
    order_history: HashMap<String, Vec<Instant>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(max_orders_per_window: usize, window_duration: Duration) -> Self {
        Self {
            max_orders_per_window,
            window_duration,
            order_history: HashMap::new(),
        }
    }

    /// Check if an order can be placed
    pub fn can_place_order(&mut self, symbol: &str) -> Result<(), OrderManager::Error> {
        let now = Instant::now();
        let orders = self.order_history.entry(symbol.to_string()).or_insert_with(Vec::new());
        
        // Remove old orders outside the window
        let cutoff = now - self.window_duration;
        orders.retain(|&timestamp| timestamp > cutoff);
        
        // Check if we're at the limit
        if orders.len() >= self.max_orders_per_window {
            return Err(OrderManager::Error::Other(format!(
                "Rate limit exceeded for symbol {}: {}/{} (max: {})",
                symbol,
                orders.len(),
                self.max_orders_per_window
            )));
        }
        
        // Add the new order
        orders.push(now);
        Ok(())
    }

    /// Get the current rate for a symbol
    pub fn get_current_rate(&self, symbol: &str) -> f64 {
        let now = Instant::now();
        let orders = self.order_history.entry(symbol.to_string()).or_insert_with(Vec::new());
        
        // Count orders in the current window
        let cutoff = now - self.window_duration;
        let count_in_window = orders.iter().filter(|&timestamp| timestamp > cutoff).count();
        
        count_in_window as f64 / self.window_duration.as_secs_f64()
    }

    /// Get the remaining time until the next order can be placed
    pub fn time_until_next_order(&self, symbol: &str) -> Option<Duration> {
        let now = Instant::now();
        let orders = self.order_history.entry(symbol.to_string()).or_insert_with(Vec::new());
        
        // Count orders in the current window
        let cutoff = now - self.window_duration;
        let count_in_window = orders.iter().filter(|&timestamp| timestamp > cutoff).count();
        
        if count_in_window >= self.max_orders_per_window {
            // Find the oldest order in the window
            if let Some(oldest_timestamp) = orders.iter().filter(|&timestamp| timestamp > cutoff).min() {
                let time_since_oldest = now.duration_since(*oldest_timestamp);
                let time_until_next = self.window_duration - time_since_oldest;
                
                if time_until_next.is_zero() {
                    return Some(Duration::ZERO);
                } else {
                    return Some(time_until_next);
                }
            }
        }
        
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(5, Duration::from_secs(60));
        
        // Should be able to place orders initially
        assert!(limiter.can_place_order("BTCUSDT").is_ok());
        
        // Place 5 orders
        for i in 0..5 {
            assert!(limiter.can_place_order("BTCUSDT").is_ok());
            limiter.can_place_order("BTCUSDT").unwrap();
        }
        
        // Next order should be rejected
        assert!(limiter.can_place_order("BTCUSDT").is_err());
        
        // Check current rate
        let rate = limiter.get_current_rate("BTCUSDT");
        assert!(rate > 0.0);
        
        // Check time until next order
        let time_until_next = limiter.time_until_next_order("BTCUSDT").unwrap();
        assert!(time_until_next.is_some());
        
        // Wait for the window to pass
        std::thread::sleep(Duration::from_secs(61));
        
        // Should be able to place orders again
        assert!(limiter.can_place_order("BTCUSDT").is_ok());
    }
}
