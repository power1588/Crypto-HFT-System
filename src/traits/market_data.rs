use async_trait::async_trait;
use crate::traits::events::MarketEvent;
use crate::types::Price;

/// Trait for streaming market data
/// This allows the strategy to be independent of the specific exchange implementation
#[async_trait]
pub trait MarketDataStream {
    /// Error type for this stream
    type Error: std::error::Error + Send + Sync + 'static;

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
    type Error: std::error::Error + Send + Sync + 'static;

    /// Get historical order book snapshots for a symbol
    async fn get_order_book_snapshots(
        &self,
        symbol: &str,
        start_time: u64,
        end_time: u64,
    ) -> Result<Vec<crate::orderbook::OrderBookSnapshot>, Self::Error>;

    /// Get historical trades for a symbol
    async fn get_trades(
        &self,
        symbol: &str,
        start_time: u64,
        end_time: u64,
    ) -> Result<Vec<Trade>, Self::Error>;
}

/// Historical trade data
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trade {
    pub symbol: String,
    pub price: Price,
    pub size: crate::types::Size,
    pub timestamp: u64,
    pub is_buyer_maker: bool,
    pub trade_id: String,
}
