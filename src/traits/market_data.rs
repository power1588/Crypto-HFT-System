use crate::core::events::{MarketEvent, OrderBookSnapshot, Trade};
use async_trait::async_trait;

/// Trait for streaming market data
/// This allows the strategy to be independent of the specific exchange implementation
#[async_trait]
pub trait MarketDataStream {
    /// Error type for this stream
    /// Note: This can be a concrete error type or a boxed error type
    type Error: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static;

    /// Subscribe to market data for the given symbols
    async fn subscribe(&mut self, symbols: &[&str]) -> Result<(), Self::Error>;

    /// Unsubscribe from market data for the given symbols
    async fn unsubscribe(&mut self, symbols: &[&str]) -> Result<(), Self::Error>;

    /// Get the next market event from the stream
    /// Returns None if the stream is closed
    async fn next(&mut self) -> Option<Result<MarketEvent, Self::Error>>;

    /// Check if the stream is connected
    fn is_connected(&self) -> bool;

    /// Get the last update timestamp for a symbol
    fn last_update(&self, symbol: &str) -> Option<u64>;
}

/// Trait for historical market data access
#[async_trait]
pub trait MarketDataHistory {
    /// Error type for this history provider
    type Error: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static;

    /// Get historical order book snapshots for a symbol
    async fn get_order_book_snapshots(
        &self,
        symbol: &str,
        start_time: u64,
        end_time: u64,
    ) -> Result<Vec<OrderBookSnapshot>, Self::Error>;

    /// Get historical trades for a symbol
    async fn get_trades(
        &self,
        symbol: &str,
        start_time: u64,
        end_time: u64,
    ) -> Result<Vec<Trade>, Self::Error>;
}
