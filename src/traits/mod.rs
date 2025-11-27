pub mod market_data;
pub mod execution;
pub mod events;

// Re-export all traits
pub use market_data::MarketDataStream;
pub use market_data::MarketDataHistory;
pub use execution::ExecutionClient;
pub use execution::OrderManager;

// Re-export all types
pub use events::{
    MarketEvent, NewOrder, OrderId, ExecutionReport, OrderStatus, OrderSide, OrderType, TimeInForce
};
pub use execution::{Balance, TradingFees};
pub use market_data::Trade;
