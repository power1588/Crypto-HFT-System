pub mod types;
pub mod orderbook;
pub mod traits;
pub mod connectors;
pub mod strategy;

pub use types::{Price, Size};
pub use orderbook::{OrderBook, OrderBookLevel, OrderBookSnapshot, OrderBookDelta};
pub use traits::{
    MarketDataStream, MarketDataHistory, ExecutionClient, OrderManager,
    MarketEvent, NewOrder, OrderId, ExecutionReport, OrderStatus, OrderSide, OrderType, TimeInForce,
    Balance, TradingFees, Trade
};
pub use connectors::{BinanceMessage, MockMarketDataStream, MockExecutionClient};
pub use strategy::engine::{StrategyEngine, Signal, MarketState, Strategy};
pub use strategy::SimpleArbitrageStrategy;
