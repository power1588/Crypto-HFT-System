use crate::core::events::{Balance, ExecutionReport, NewOrder, OrderId, TradingFees};
use async_trait::async_trait;

/// Trait for order execution
/// This allows the strategy to be independent of the specific exchange implementation
#[async_trait]
pub trait ExecutionClient {
    /// Error type for this client
    type Error: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static;

    /// Place a new order
    async fn place_order(&self, order: NewOrder) -> Result<OrderId, Self::Error>;

    /// Cancel an existing order
    async fn cancel_order(&self, order_id: OrderId) -> Result<(), Self::Error>;

    /// Get the status of an order
    async fn get_order_status(&self, order_id: OrderId) -> Result<ExecutionReport, Self::Error>;

    /// Get account balances
    async fn get_balances(&self) -> Result<Vec<Balance>, Self::Error>;

    /// Get open orders
    async fn get_open_orders(
        &self,
        symbol: Option<&str>,
    ) -> Result<Vec<ExecutionReport>, Self::Error>;

    /// Get order history
    async fn get_order_history(
        &self,
        symbol: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<ExecutionReport>, Self::Error>;

    /// Get trading fees for a symbol
    async fn get_trading_fees(&self, symbol: &str) -> Result<TradingFees, Self::Error>;
}

/// Trait for order management
#[async_trait]
pub trait OrderManager {
    /// Error type for this manager
    type Error: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static;

    /// Update order status based on execution report
    async fn handle_execution_report(&mut self, report: ExecutionReport)
        -> Result<(), Self::Error>;

    /// Get all tracked orders
    async fn get_all_orders(&self) -> Result<Vec<ExecutionReport>, Self::Error>;

    /// Get orders by symbol
    async fn get_orders_by_symbol(&self, symbol: &str)
        -> Result<Vec<ExecutionReport>, Self::Error>;

    /// Get open orders
    async fn get_open_orders(&self) -> Result<Vec<ExecutionReport>, Self::Error>;
}
