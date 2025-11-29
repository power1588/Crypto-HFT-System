use crate::core::events::{MarketEvent, Signal, TradingEvent};
use crate::types::Symbol;
use async_trait::async_trait;

/// Strategy state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrategyState {
    MarketMaking,
    Arbitrage(crate::strategies::arbitrage::ArbitrageState),
    Prediction,
}

/// Strategy metrics
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrategyMetrics {
    pub total_trades: u64,
    pub winning_trades: u64,
    pub losing_trades: u64,
    pub total_pnl: rust_decimal::Decimal,
    pub gross_profit: rust_decimal::Decimal,
    pub gross_loss: rust_decimal::Decimal,
    pub profit_factor: rust_decimal::Decimal,
    pub max_drawdown: rust_decimal::Decimal,
    pub sharpe_ratio: rust_decimal::Decimal,
    pub average_trade_pnl: rust_decimal::Decimal,
    pub win_rate: rust_decimal::Decimal,
    pub average_holding_time_ms: u64,
}

/// Strategy configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrategyConfig {
    pub strategy_type: String,
    pub symbols: Vec<Symbol>,
    pub exchanges: Vec<String>,
    pub parameters: std::collections::HashMap<String, String>,
}

/// Main strategy trait
/// All trading strategies must implement this trait
#[async_trait]
pub trait Strategy: Send + Sync {
    /// Error type for this strategy
    type Error: std::error::Error + Send + Sync + 'static;

    /// Initialize the strategy with configuration
    async fn initialize(&mut self, config: StrategyConfig) -> Result<(), Self::Error>;

    /// Process a market event and generate signals
    async fn on_market_event(&mut self, event: MarketEvent) -> Result<Vec<Signal>, Self::Error>;

    /// Process a trading event and update state
    async fn on_trading_event(&mut self, event: TradingEvent) -> Result<(), Self::Error>;

    /// Get current strategy state
    fn get_state(&self) -> StrategyState;

    /// Get strategy performance metrics
    fn get_metrics(&self) -> StrategyMetrics;

    /// Shutdown the strategy gracefully
    async fn shutdown(&mut self) -> Result<(), Self::Error>;
}

/// Signal validator trait
/// Used to validate signals before execution
pub trait SignalValidator: Send + Sync {
    /// Validate a signal
    fn validate_signal(&self, signal: &Signal, state: &StrategyState) -> Result<(), String>;
}

/// Risk manager trait
/// Used to manage risk across strategies
#[async_trait]
pub trait RiskManager: Send + Sync {
    /// Error type for this risk manager
    type Error: std::error::Error + Send + Sync + 'static;

    /// Check if a signal violates risk rules
    async fn check_signal(&self, signal: &Signal, state: &StrategyState)
        -> Result<(), Self::Error>;

    /// Handle a risk violation
    async fn handle_violation(&mut self, violation: &str) -> Result<(), Self::Error>;
}

/// Position manager trait
/// Used to manage positions across exchanges
#[async_trait]
pub trait PositionManager: Send + Sync {
    /// Error type for this position manager
    type Error: std::error::Error + Send + Sync + 'static;

    /// Update position based on execution report
    async fn update_position(&mut self, report: &TradingEvent) -> Result<(), Self::Error>;

    /// Get current position for a symbol
    async fn get_position(
        &self,
        symbol: &Symbol,
        exchange: &str,
    ) -> Result<rust_decimal::Decimal, Self::Error>;

    /// Get all positions
    async fn get_all_positions(
        &self,
    ) -> Result<std::collections::HashMap<String, rust_decimal::Decimal>, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_config() {
        let mut parameters = std::collections::HashMap::new();
        parameters.insert("spread_bps".to_string(), "10".to_string());
        parameters.insert("order_size".to_string(), "0.01".to_string());

        let config = StrategyConfig {
            strategy_type: "market_making".to_string(),
            symbols: vec![Symbol::new("BTCUSDT")],
            exchanges: vec!["binance".to_string()],
            parameters,
        };

        assert_eq!(config.strategy_type, "market_making");
        assert_eq!(config.symbols.len(), 1);
        assert_eq!(config.exchanges.len(), 1);
        assert_eq!(config.parameters.get("spread_bps"), Some(&"10".to_string()));
    }

    #[test]
    fn test_strategy_metrics() {
        let metrics = StrategyMetrics {
            total_trades: 100,
            winning_trades: 55,
            losing_trades: 45,
            total_pnl: rust_decimal::Decimal::new(500, 2), // 5.00
            gross_profit: rust_decimal::Decimal::new(1100, 2), // 11.00
            gross_loss: rust_decimal::Decimal::new(600, 2), // 6.00
            profit_factor: rust_decimal::Decimal::new(183, 2), // 1.83
            max_drawdown: rust_decimal::Decimal::new(200, 2), // 2.00
            sharpe_ratio: rust_decimal::Decimal::new(150, 2), // 1.50
            average_trade_pnl: rust_decimal::Decimal::new(50, 2), // 0.50
            win_rate: rust_decimal::Decimal::new(55, 2),   // 0.55
            average_holding_time_ms: 5000,
        };

        assert_eq!(metrics.total_trades, 100);
        assert_eq!(metrics.winning_trades, 55);
        assert_eq!(metrics.win_rate, rust_decimal::Decimal::new(55, 2));
    }
}
