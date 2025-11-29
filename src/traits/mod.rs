pub mod events;
pub mod execution;
pub mod market_data;
pub mod strategy;

// Re-export all traits
pub use execution::ExecutionClient;
pub use execution::OrderManager;
pub use market_data::MarketDataHistory;
pub use market_data::MarketDataStream;

// Re-export all types from events (which now re-exports from core::events)
pub use events::{
    Balance,
    // Type aliases
    ExchangeId,
    ExecutionReport,
    // Enums
    MarketEvent,
    NewOrder,
    Order,
    OrderBookDelta,
    // Structs
    OrderBookLevel,
    OrderBookSnapshot,
    OrderId,
    OrderSide,
    OrderStatus,

    OrderType,
    Position,
    RiskViolation,
    Signal,
    SystemEvent,
    TimeInForce,
    Timestamp,

    Trade,
    TradingEvent,
    TradingFees,
};

pub use strategy::{
    PositionManager, RiskManager, SignalValidator, Strategy, StrategyConfig, StrategyMetrics,
    StrategyState,
};
