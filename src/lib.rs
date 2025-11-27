pub mod types;
pub mod orderbook;
pub mod traits;
pub mod connectors;
pub mod strategy;
pub mod risk;
pub mod oms;
pub mod changes;

pub use types::{Price, Size};
pub use orderbook::{OrderBook, OrderBookLevel, OrderBookSnapshot, OrderBookDelta};
pub use traits::{
    MarketDataStream, MarketDataHistory, ExecutionClient, OrderManager,
    MarketEvent, NewOrder, OrderId, ExecutionReport, OrderStatus, OrderSide, OrderType, TimeInForce,
    Balance, TradingFees, Trade
};
pub use connectors::{BinanceMessage, MockMarketDataStream, MockExecutionClient};
pub use strategy::{
    Strategy, MarketState, SimpleArbitrageStrategy, EventDrivenStrategy,
    PortfolioRebalancer, MarketMakingStrategy,
};
pub use risk::{RiskEngine, RiskViolation, RiskRule, ShadowLedger, Inventory, Balance};
pub use oms::{OrderManager, RateLimiter};
pub use changes::{
    binance::BinanceWebSocketAdapter,
    mock::MockExchangeAdapter,
    binance::HighPerformanceBinanceAdapter,
    order_router::OrderRouter,
    connection_manager::ConnectionManager,
    error_handler::ErrorHandler,
};
