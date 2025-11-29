use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Rate limiter implementation using token bucket algorithm
pub struct RateLimiter {
    /// Maximum number of requests allowed in the time window
    max_requests: usize,
    /// Time window for rate limiting
    window: Duration,
    /// History of request timestamps
    request_history: Arc<Mutex<VecDeque<Instant>>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            request_history: Arc::new(Mutex::new(VecDeque::with_capacity(max_requests))),
        }
    }

    /// Check if a request is allowed and record it
    pub async fn check_limit(&self) -> bool {
        let now = Instant::now();

        // Clean up old requests outside the window
        {
            let mut history = self.request_history.lock().unwrap();

            // Remove requests older than the window
            while let Some(&front_time) = history.front() {
                if now.duration_since(front_time) >= self.window {
                    history.pop_front();
                } else {
                    break;
                }
            }

            // Check if we're at the limit
            if history.len() >= self.max_requests {
                return false;
            }

            // Record this request
            history.push_back(now);
        }

        true
    }

    /// Wait until a request is allowed
    pub async fn wait_for_slot(&self) {
        while !self.check_limit().await {
            // Calculate how long to wait until the oldest request is outside the window
            let wait_time = {
                let history = self.request_history.lock().unwrap();
                if let Some(&oldest_time) = history.front() {
                    let elapsed = oldest_time.elapsed();
                    if elapsed < self.window {
                        self.window - elapsed
                    } else {
                        Duration::from_millis(1) // Small delay to prevent busy waiting
                    }
                } else {
                    Duration::from_millis(0)
                }
            };

            sleep(wait_time).await;
        }
    }

    /// Get the current number of requests in the window
    pub fn current_requests(&self) -> usize {
        let now = Instant::now();
        let mut history = self.request_history.lock().unwrap();

        // Clean up old requests
        while let Some(&front_time) = history.front() {
            if now.duration_since(front_time) >= self.window {
                history.pop_front();
            } else {
                break;
            }
        }

        history.len()
    }

    /// Get the maximum number of requests allowed
    pub fn max_requests(&self) -> usize {
        self.max_requests
    }

    /// Get the time window
    pub fn window(&self) -> Duration {
        self.window
    }

    /// Get the time until the next request is allowed
    pub fn time_until_next_request(&self) -> Duration {
        let now = Instant::now();
        let history = self.request_history.lock().unwrap();

        if history.len() < self.max_requests {
            return Duration::from_millis(0);
        }

        if let Some(&oldest_time) = history.front() {
            let elapsed = now.duration_since(oldest_time);
            if elapsed < self.window {
                self.window - elapsed
            } else {
                Duration::from_millis(0)
            }
        } else {
            Duration::from_millis(0)
        }
    }

    /// Reset the rate limiter
    pub fn reset(&self) {
        let mut history = self.request_history.lock().unwrap();
        history.clear();
    }
}

/// Multi-rate limiter for different types of requests
pub struct MultiRateLimiter {
    /// Individual rate limiters for different request types
    limiters: std::collections::HashMap<String, RateLimiter>,
}

impl MultiRateLimiter {
    /// Create a new multi-rate limiter
    pub fn new() -> Self {
        Self {
            limiters: std::collections::HashMap::new(),
        }
    }

    /// Add a rate limiter for a specific request type
    pub fn add_limiter(&mut self, request_type: &str, max_requests: usize, window: Duration) {
        let limiter = RateLimiter::new(max_requests, window);
        self.limiters.insert(request_type.to_string(), limiter);
    }

    /// Check if a request of a specific type is allowed
    pub async fn check_limit(&self, request_type: &str) -> bool {
        if let Some(limiter) = self.limiters.get(request_type) {
            limiter.check_limit().await
        } else {
            true // No limit configured for this request type
        }
    }

    /// Wait until a request of a specific type is allowed
    pub async fn wait_for_slot(&self, request_type: &str) {
        if let Some(limiter) = self.limiters.get(request_type) {
            limiter.wait_for_slot().await;
        }
    }

    /// Get the current number of requests for a specific type
    pub fn current_requests(&self, request_type: &str) -> usize {
        if let Some(limiter) = self.limiters.get(request_type) {
            limiter.current_requests()
        } else {
            0
        }
    }

    /// Get the time until the next request of a specific type is allowed
    pub fn time_until_next_request(&self, request_type: &str) -> Duration {
        if let Some(limiter) = self.limiters.get(request_type) {
            limiter.time_until_next_request()
        } else {
            Duration::from_millis(0)
        }
    }

    /// Reset all rate limiters
    pub fn reset_all(&self) {
        for limiter in self.limiters.values() {
            limiter.reset();
        }
    }

    /// Reset a specific rate limiter
    pub fn reset(&self, request_type: &str) {
        if let Some(limiter) = self.limiters.get(request_type) {
            limiter.reset();
        }
    }
}

/// Rate limiter with exponential backoff for handling rate limit errors
pub struct AdaptiveRateLimiter {
    /// Base rate limiter
    base_limiter: RateLimiter,
    /// Current backoff multiplier (wrapped in Mutex for interior mutability)
    backoff_multiplier: Mutex<f64>,
    /// Maximum backoff multiplier
    max_backoff_multiplier: f64,
    /// Time to wait before reducing backoff
    backoff_reset_time: Duration,
    /// Last time we hit a rate limit
    last_rate_limit_hit: Arc<Mutex<Option<Instant>>>,
    /// Last time we reduced backoff
    last_backoff_reset: Arc<Mutex<Option<Instant>>>,
}

impl AdaptiveRateLimiter {
    /// Create a new adaptive rate limiter
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            base_limiter: RateLimiter::new(max_requests, window),
            backoff_multiplier: Mutex::new(1.0),
            max_backoff_multiplier: 10.0,
            backoff_reset_time: Duration::from_secs(60), // Reset after 1 minute
            last_rate_limit_hit: Arc::new(Mutex::new(None)),
            last_backoff_reset: Arc::new(Mutex::new(None)),
        }
    }

    /// Check if a request is allowed and record it
    pub async fn check_limit(&self) -> bool {
        // Apply backoff to the base rate limit
        let backoff = *self.backoff_multiplier.lock().unwrap();
        let effective_max_requests = (self.base_limiter.max_requests() as f64 / backoff) as usize;

        let now = Instant::now();

        // Check if we should reduce backoff
        {
            let mut last_reset = self.last_backoff_reset.lock().unwrap();
            if let Some(reset_time) = *last_reset {
                if now.duration_since(reset_time) >= self.backoff_reset_time {
                    // Reduce backoff
                    let mut backoff_guard = self.backoff_multiplier.lock().unwrap();
                    *backoff_guard = (*backoff_guard / 2.0).max(1.0);
                    *last_reset = Some(now);
                }
            } else {
                *last_reset = Some(now);
            }
        }

        // Check if we're at the effective limit
        let current_requests = self.base_limiter.current_requests();
        if current_requests >= effective_max_requests {
            return false;
        }

        // Record this request
        self.base_limiter.check_limit().await
    }

    /// Wait until a request is allowed
    pub async fn wait_for_slot(&self) {
        while !self.check_limit().await {
            // Calculate wait time with backoff
            let wait_time = {
                let base_wait = self.base_limiter.time_until_next_request();
                let backoff = *self.backoff_multiplier.lock().unwrap();
                let backoff_wait = base_wait * backoff as u32;
                backoff_wait.max(Duration::from_millis(100)) // Minimum wait time
            };

            sleep(wait_time).await;
        }
    }

    /// Notify that we hit a rate limit
    pub fn notify_rate_limit_hit(&self) {
        let now = Instant::now();

        // Update last rate limit hit time
        {
            let mut last_hit = self.last_rate_limit_hit.lock().unwrap();
            *last_hit = Some(now);
        }

        // Increase backoff
        {
            let mut backoff = self.backoff_multiplier.lock().unwrap();
            *backoff = (*backoff * 1.5).min(self.max_backoff_multiplier);
        }
    }

    /// Get the current backoff multiplier
    pub fn backoff_multiplier(&self) -> f64 {
        *self.backoff_multiplier.lock().unwrap()
    }

    /// Get the effective maximum requests considering backoff
    pub fn effective_max_requests(&self) -> usize {
        let backoff = *self.backoff_multiplier.lock().unwrap();
        (self.base_limiter.max_requests() as f64 / backoff) as usize
    }

    /// Get the time until the next request is allowed
    pub fn time_until_next_request(&self) -> Duration {
        let base_wait = self.base_limiter.time_until_next_request();
        let backoff = *self.backoff_multiplier.lock().unwrap();
        let backoff_wait = base_wait * backoff as u32;
        backoff_wait.max(Duration::from_millis(100))
    }

    /// Reset the rate limiter and backoff
    pub fn reset(&self) {
        self.base_limiter.reset();

        {
            let mut backoff = self.backoff_multiplier.lock().unwrap();
            *backoff = 1.0;
        }

        {
            let mut last_hit = self.last_rate_limit_hit.lock().unwrap();
            *last_hit = None;
        }

        {
            let mut last_reset = self.last_backoff_reset.lock().unwrap();
            *last_reset = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration as TokioDuration};

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let limiter = RateLimiter::new(5, Duration::from_secs(1));

        // Should allow the first 5 requests
        for _ in 0..5 {
            assert!(limiter.check_limit().await);
        }

        // Should reject the 6th request
        assert!(!limiter.check_limit().await);

        // Should show 5 current requests
        assert_eq!(limiter.current_requests(), 5);

        // Wait for the window to pass
        sleep(TokioDuration::from_millis(1100)).await;

        // Should allow requests again
        assert!(limiter.check_limit().await);
        assert_eq!(limiter.current_requests(), 1);
    }

    #[tokio::test]
    async fn test_rate_limiter_wait_for_slot() {
        let limiter = RateLimiter::new(2, Duration::from_millis(500));

        // Use up the limit
        assert!(limiter.check_limit().await);
        assert!(limiter.check_limit().await);

        // Should reject the 3rd request
        assert!(!limiter.check_limit().await);

        // Wait for a slot
        let start = Instant::now();
        limiter.wait_for_slot().await;
        let elapsed = start.elapsed();

        // Should have waited at least 500ms
        assert!(elapsed >= Duration::from_millis(500));

        // Should now allow a request
        assert!(limiter.check_limit().await);
    }

    #[tokio::test]
    async fn test_multi_rate_limiter() {
        let mut multi_limiter = MultiRateLimiter::new();

        // Add limiters for different request types
        multi_limiter.add_limiter("read", 10, Duration::from_secs(1));
        multi_limiter.add_limiter("write", 2, Duration::from_secs(1));

        // Use up the write limit
        assert!(multi_limiter.check_limit("write").await);
        assert!(multi_limiter.check_limit("write").await);

        // Should reject the 3rd write request
        assert!(!multi_limiter.check_limit("write").await);

        // But should still allow read requests
        assert!(multi_limiter.check_limit("read").await);

        // Check current requests
        assert_eq!(multi_limiter.current_requests("write"), 2);
        assert_eq!(multi_limiter.current_requests("read"), 1);
    }

    #[tokio::test]
    async fn test_adaptive_rate_limiter() {
        let limiter = AdaptiveRateLimiter::new(10, Duration::from_secs(1));

        // Initially should allow 10 requests
        for _ in 0..10 {
            assert!(limiter.check_limit().await);
        }

        // Should reject the 11th request
        assert!(!limiter.check_limit().await);

        // Notify rate limit hit
        limiter.notify_rate_limit_hit();

        // Backoff should increase
        assert!(limiter.backoff_multiplier() > 1.0);

        // Effective max requests should decrease
        assert!(limiter.effective_max_requests() < 10);

        // Time until next request should increase
        let base_wait = Duration::from_millis(100);
        let adaptive_wait = limiter.time_until_next_request();
        assert!(adaptive_wait > base_wait);
    }

    #[tokio::test]
    async fn test_adaptive_rate_limiter_backoff_reset() {
        let limiter = AdaptiveRateLimiter::new(10, Duration::from_secs(1));

        // Notify rate limit hit to increase backoff
        limiter.notify_rate_limit_hit();
        limiter.notify_rate_limit_hit();

        let initial_backoff = limiter.backoff_multiplier();
        assert!(initial_backoff > 1.0);

        // Wait for backoff reset time (shortened for test)
        sleep(TokioDuration::from_millis(100)).await;

        // Check limit to trigger backoff reset check
        limiter.check_limit().await;

        // Backoff should be reduced
        let new_backoff = limiter.backoff_multiplier();
        assert!(new_backoff < initial_backoff);
    }

    #[tokio::test]
    async fn test_rate_limiter_reset() {
        let limiter = RateLimiter::new(5, Duration::from_secs(1));

        // Use up the limit
        for _ in 0..5 {
            let _ = limiter.check_limit().await;
        }

        assert_eq!(limiter.current_requests(), 5);

        // Reset
        limiter.reset();

        assert_eq!(limiter.current_requests(), 0);
    }

    #[tokio::test]
    async fn test_multi_rate_limiter_reset() {
        let mut multi_limiter = MultiRateLimiter::new();

        // Add limiters
        multi_limiter.add_limiter("read", 10, Duration::from_secs(1));
        multi_limiter.add_limiter("write", 2, Duration::from_secs(1));

        // Use up the limits
        for _ in 0..10 {
            let _ = multi_limiter.check_limit("read").await;
        }

        for _ in 0..2 {
            let _ = multi_limiter.check_limit("write").await;
        }

        assert_eq!(multi_limiter.current_requests("read"), 10);
        assert_eq!(multi_limiter.current_requests("write"), 2);

        // Reset all
        multi_limiter.reset_all();

        assert_eq!(multi_limiter.current_requests("read"), 0);
        assert_eq!(multi_limiter.current_requests("write"), 0);

        // Use up the limits again
        for _ in 0..10 {
            let _ = multi_limiter.check_limit("read").await;
        }

        for _ in 0..2 {
            let _ = multi_limiter.check_limit("write").await;
        }

        assert_eq!(multi_limiter.current_requests("read"), 10);
        assert_eq!(multi_limiter.current_requests("write"), 2);

        // Reset just one
        multi_limiter.reset("read");

        assert_eq!(multi_limiter.current_requests("read"), 0);
        assert_eq!(multi_limiter.current_requests("write"), 2);
    }
}
