pub mod types;
pub mod orderbook;
pub mod traits;
pub mod connectors;
pub mod strategy;
pub mod risk;
pub mod oms;

pub use types::{Price, Size};
pub use orderbook::{OrderBook, OrderBookLevel, OrderBookSnapshot, OrderBookDelta};
pub use traits::{
    MarketDataStream, MarketDataHistory, ExecutionClient, OrderManager,
    MarketEvent, NewOrder, OrderId, ExecutionReport, OrderStatus, OrderSide, OrderType, TimeInForce,
    Balance, TradingFees, Trade
};
pub use connectors::{BinanceMessage, MockMarketDataStream, MockExecutionClient};
pub use strategy::{StrategyEngine, Signal, Strategy, MarketState, SimpleArbitrageStrategy};
pub use risk::{RiskEngine, RiskViolation, RiskRule, ShadowLedger, Inventory, Balance};
pub use oms::{OrderManager, RateLimiter};
