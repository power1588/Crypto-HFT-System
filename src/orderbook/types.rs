// Re-export orderbook types from core::events for consistency
// This ensures a unified type system across the codebase

pub use crate::core::events::{OrderBookDelta, OrderBookLevel, OrderBookSnapshot};
