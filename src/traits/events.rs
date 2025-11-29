// Re-export all types from core::events to provide a unified interface
// This eliminates duplicate type definitions and ensures consistency across the codebase

pub use crate::core::events::{
    Balance,
    // Type aliases
    ExchangeId,
    ExecutionReport,
    // Events
    MarketEvent,
    NewOrder,
    Order,
    OrderBookDelta,
    // Structs
    OrderBookLevel,
    OrderBookSnapshot,
    OrderId,
    // Enums
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
