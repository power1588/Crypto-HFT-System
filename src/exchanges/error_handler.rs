use crate::traits::{MarketDataStream, MarketEvent, ExecutionClient, OrderManager};
use std::collections::HashMap;

/// Error types for exchange adapters
#[derive(Debug, Clone)]
pub enum ExchangeError {
    /// Connection error
    ConnectionError(String),
    /// API error
    ApiError(String),
    /// Rate limit error
    RateLimitError(String),
    /// Order error
    OrderError(String),
    /// Network error
    NetworkError(String),
}

/// Error handler for exchange adapters
pub struct ErrorHandler {
    /// Error handlers by error type
    handlers: HashMap<String, Box<dyn Fn(Box<dyn std::error::Error>) + Send>>,
}

impl ErrorHandler {
    /// Create a new error handler
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Add an error handler
    pub fn add_handler(&mut self, error_type: String, handler: Box<dyn Fn(Box<dyn std::error::Error>) + Send>) {
        self.handlers.insert(error_type, handler);
    }

    /// Handle an error
    pub fn handle_error(&self, error: Box<dyn std::error::Error>) {
        if let Some(handler) = self.handlers.get(&error.to_string()) {
            handler(error);
        } else {
            eprintln!("No handler for error type: {}", error.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Price, Size};

    #[test]
    fn test_error_handler() {
        let mut handler = ErrorHandler::new();
        
        // Add handlers
        handler.add_handler("connection".to_string(), Box::new(|e| {
            eprintln!("Connection error: {}", e);
        }));
        handler.add_handler("api".to_string(), Box::new(|e| {
            eprintln!("API error: {}", e);
        }));
        
        // Test connection error
        let connection_error = ExchangeError::ConnectionError("Failed to connect".to_string());
        handler.handle_error(Box::new(connection_error));
        
        // Test API error
        let api_error = ExchangeError::ApiError("Invalid API key".to_string());
        handler.handle_error(Box::new(api_error));
        
        // Test unknown error
        let unknown_error = ExchangeError::NetworkError("Unknown network error".to_string());
        handler.handle_error(Box::new(unknown_error));
        
        // Test error without handler
        let unhandled_error = ExchangeError::OrderError("Order rejected".to_string());
        handler.handle_error(Box::new(unhandled_error));
    }
}
