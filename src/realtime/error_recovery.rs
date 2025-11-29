use log::{error, info, warn};
use std::sync::Arc;
/// Error recovery mechanisms for resilient operation
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker for preventing cascading failures
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<RwLock<u32>>,
    failure_threshold: u32,
    timeout: Duration,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    success_count: Arc<RwLock<u32>>,
    success_threshold: u32,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(failure_threshold: u32, timeout: Duration, success_threshold: u32) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            failure_threshold,
            timeout,
            last_failure_time: Arc::new(RwLock::new(None)),
            success_count: Arc::new(RwLock::new(0)),
            success_threshold,
        }
    }

    /// Check if operation is allowed
    pub async fn can_execute(&self) -> bool {
        let state = *self.state.read().await;
        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has passed
                if let Some(last_failure) = *self.last_failure_time.read().await {
                    if last_failure.elapsed() >= self.timeout {
                        // Transition to half-open
                        let mut state_guard = self.state.write().await;
                        *state_guard = CircuitState::HalfOpen;
                        *self.success_count.write().await = 0;
                        info!("Circuit breaker transitioning to half-open state");
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record a successful operation
    pub async fn record_success(&self) {
        let state = *self.state.read().await;
        match state {
            CircuitState::Closed => {
                // Reset failure count on success
                *self.failure_count.write().await = 0;
            }
            CircuitState::HalfOpen => {
                let mut success_count = self.success_count.write().await;
                *success_count += 1;
                if *success_count >= self.success_threshold {
                    // Transition back to closed
                    let mut state_guard = self.state.write().await;
                    *state_guard = CircuitState::Closed;
                    *self.failure_count.write().await = 0;
                    info!("Circuit breaker closed - service recovered");
                }
            }
            CircuitState::Open => {
                // Should not happen, but handle gracefully
            }
        }
    }

    /// Record a failed operation
    pub async fn record_failure(&self) {
        let mut state_guard = self.state.write().await;
        let state = *state_guard;

        match state {
            CircuitState::Closed | CircuitState::HalfOpen => {
                let mut failure_count = self.failure_count.write().await;
                *failure_count += 1;
                *self.last_failure_time.write().await = Some(Instant::now());

                if *failure_count >= self.failure_threshold {
                    *state_guard = CircuitState::Open;
                    error!("Circuit breaker opened - too many failures");
                }
            }
            CircuitState::Open => {
                // Already open, update failure time
                *self.last_failure_time.write().await = Some(Instant::now());
            }
        }
    }

    /// Get current state
    pub async fn state(&self) -> CircuitState {
        *self.state.read().await
    }
}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            multiplier: 2.0,
            jitter: true,
        }
    }
}

/// Retry helper with exponential backoff
pub async fn retry_with_backoff<F, T, E>(config: &RetryConfig, mut operation: F) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
{
    let mut delay = config.initial_delay;

    for attempt in 0..config.max_attempts {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempt == config.max_attempts - 1 {
                    return Err(e);
                }

                // Calculate next delay with exponential backoff
                let mut next_delay = delay;
                if config.jitter {
                    // Add jitter to prevent thundering herd
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = DefaultHasher::new();
                    attempt.hash(&mut hasher);
                    let jitter_ms = (hasher.finish() % 100) as u64;
                    next_delay += Duration::from_millis(jitter_ms);
                }

                next_delay = next_delay.min(config.max_delay);

                warn!(
                    "Operation failed (attempt {}/{}), retrying in {:?}",
                    attempt + 1,
                    config.max_attempts,
                    next_delay
                );

                tokio::time::sleep(next_delay).await;

                // Increase delay for next attempt
                delay = Duration::from_secs_f64(
                    (delay.as_secs_f64() * config.multiplier).min(config.max_delay.as_secs_f64()),
                );
            }
        }
    }

    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_closed() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(1), 2);
        assert!(cb.can_execute().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_failures() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(1), 2);

        // Record failures
        cb.record_failure().await;
        cb.record_failure().await;
        assert!(cb.can_execute().await); // Still closed

        cb.record_failure().await;
        assert!(!cb.can_execute().await); // Now open
    }

    #[tokio::test]
    async fn test_circuit_breaker_recovery() {
        let cb = CircuitBreaker::new(2, Duration::from_millis(100), 2);

        // Open the circuit
        cb.record_failure().await;
        cb.record_failure().await;
        assert!(!cb.can_execute().await);

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should be half-open now
        assert!(cb.can_execute().await);

        // Record successes
        cb.record_success().await;
        cb.record_success().await;

        // Should be closed again
        assert_eq!(cb.state().await, CircuitState::Closed);
    }
}
