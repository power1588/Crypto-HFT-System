use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, info, warn};

use crate::core::events::{
    MarketEvent, NewOrder, OrderBookDelta, OrderBookSnapshot, OrderSide, OrderStatus, OrderType,
    Signal, TimeInForce, TradingEvent,
};
use crate::traits::strategy::{
    PositionManager, RiskManager, SignalValidator, Strategy, StrategyConfig, StrategyMetrics,
    StrategyState,
};
use crate::types::{Price, Size, Symbol};

/// Error type for ArbitrageStrategy
#[derive(Debug, Clone)]
pub struct ArbitrageError {
    pub message: String,
}

impl ArbitrageError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ArbitrageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ArbitrageError: {}", self.message)
    }
}

impl std::error::Error for ArbitrageError {}

impl From<Box<dyn std::error::Error + Send + Sync>> for ArbitrageError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        ArbitrageError::new(err.to_string())
    }
}

/// Arbitrage strategy state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArbitrageState {
    pub active_opportunities: HashMap<String, ArbitrageOpportunity>,
    pub executed_trades: Vec<ArbitrageTrade>,
    pub last_update: std::time::Instant,
}

/// Arbitrage opportunity
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArbitrageOpportunity {
    pub symbol: Symbol,
    pub exchange_buy: String,
    pub exchange_sell: String,
    pub price_buy: Price,
    pub price_sell: Price,
    pub spread: Price,
    pub spread_percentage: rust_decimal::Decimal,
    pub estimated_profit: rust_decimal::Decimal,
    pub timestamp: std::time::Instant,
}

/// Arbitrage trade
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArbitrageTrade {
    pub id: String,
    pub symbol: Symbol,
    pub exchange_buy: String,
    pub exchange_sell: String,
    pub price_buy: Price,
    pub price_sell: Price,
    pub size: Size,
    pub buy_order_id: Option<String>,
    pub sell_order_id: Option<String>,
    pub status: ArbitrageTradeStatus,
    pub timestamp: std::time::Instant,
}

/// Arbitrage trade status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArbitrageTradeStatus {
    Pending,
    BuyPlaced,
    SellPlaced,
    Completed,
    Failed,
    Cancelled,
}

/// Arbitrage strategy configuration
#[derive(Debug, Clone)]
pub struct ArbitrageConfig {
    pub min_spread_bps: rust_decimal::Decimal, // Minimum spread in basis points
    pub max_position_size: Size,               // Maximum position size per trade
    pub max_exposure: rust_decimal::Decimal,   // Maximum total exposure
    pub slippage_tolerance: rust_decimal::Decimal, // Tolerance for price slippage
    pub execution_delay_ms: u64,               // Delay between placing buy and sell orders
    pub opportunity_timeout_ms: u64,           // Timeout for arbitrage opportunities
}

impl Default for ArbitrageConfig {
    fn default() -> Self {
        Self {
            min_spread_bps: rust_decimal::Decimal::new(5, 2), // 0.05%
            max_position_size: Size::from_str("0.1").unwrap(),
            max_exposure: rust_decimal::Decimal::new(1000, 2), // $10.00
            slippage_tolerance: rust_decimal::Decimal::new(1, 3), // 0.1%
            execution_delay_ms: 100,
            opportunity_timeout_ms: 5000,
        }
    }
}

/// Boxed error type alias for convenience
type BoxedError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Cross-exchange arbitrage strategy
#[allow(dead_code)]
pub struct ArbitrageStrategy {
    config: ArbitrageConfig,
    state: ArbitrageState,
    exchanges: HashMap<String, String>, // Exchange name -> Exchange ID mapping
    price_cache: HashMap<String, HashMap<Symbol, (Price, std::time::Instant)>>,
    signal_validator: Option<Box<dyn SignalValidator>>, // SignalValidator doesn't have Error type
    risk_manager: Option<Box<dyn RiskManager<Error = BoxedError>>>,
    position_manager: Option<Box<dyn PositionManager<Error = BoxedError>>>,
    metrics: StrategyMetrics,
}

impl ArbitrageStrategy {
    /// Create a new arbitrage strategy with default configuration
    pub fn new() -> Self {
        Self::with_config(ArbitrageConfig::default())
    }

    /// Create a new arbitrage strategy with custom configuration
    pub fn with_config(config: ArbitrageConfig) -> Self {
        Self {
            config,
            state: ArbitrageState {
                active_opportunities: HashMap::new(),
                executed_trades: Vec::new(),
                last_update: std::time::Instant::now(),
            },
            exchanges: HashMap::new(), // Exchange name -> ID mapping
            price_cache: HashMap::new(),
            signal_validator: None,
            risk_manager: None,
            position_manager: None,
            metrics: StrategyMetrics {
                total_trades: 0,
                winning_trades: 0,
                losing_trades: 0,
                total_pnl: rust_decimal::Decimal::ZERO,
                gross_profit: rust_decimal::Decimal::ZERO,
                gross_loss: rust_decimal::Decimal::ZERO,
                profit_factor: rust_decimal::Decimal::ZERO,
                max_drawdown: rust_decimal::Decimal::ZERO,
                sharpe_ratio: rust_decimal::Decimal::ZERO,
                average_trade_pnl: rust_decimal::Decimal::ZERO,
                win_rate: rust_decimal::Decimal::ZERO,
                average_holding_time_ms: 0,
            },
        }
    }

    /// Initialize price cache for an exchange
    pub fn initialize_exchange_cache(&mut self, exchange_name: String) {
        self.price_cache
            .insert(exchange_name.clone(), HashMap::new());
        info!("Initialized price cache for exchange: {}", exchange_name);
    }

    /// Set the signal validator
    pub fn with_signal_validator(mut self, validator: Box<dyn SignalValidator>) -> Self {
        self.signal_validator = Some(validator);
        self
    }

    /// Set the risk manager
    pub fn with_risk_manager(mut self, manager: Box<dyn RiskManager<Error = BoxedError>>) -> Self {
        self.risk_manager = Some(manager);
        self
    }

    /// Set the position manager
    pub fn with_position_manager(
        mut self,
        manager: Box<dyn PositionManager<Error = BoxedError>>,
    ) -> Self {
        self.position_manager = Some(manager);
        self
    }

    /// Update price cache with new market data from order book snapshot
    fn update_price_cache_from_snapshot(&mut self, snapshot: &OrderBookSnapshot) {
        if let Some(cache) = self.price_cache.get_mut(&snapshot.exchange_id) {
            // Use mid price (average of best bid and ask)
            if let (Some(best_bid), Some(best_ask)) = (snapshot.bids.first(), snapshot.asks.first())
            {
                let mid_price = (best_bid.price.value() + best_ask.price.value())
                    / rust_decimal::Decimal::new(2, 0);
                cache.insert(
                    snapshot.symbol.clone(),
                    (Price::new(mid_price), std::time::Instant::now()),
                );
            }
        }
    }

    /// Update price cache with new market data from order book delta
    fn update_price_cache_from_delta(&mut self, delta: &OrderBookDelta) {
        if let Some(cache) = self.price_cache.get_mut(&delta.exchange_id) {
            // Use mid price (average of best bid and ask)
            if let (Some(best_bid), Some(best_ask)) = (delta.bids.first(), delta.asks.first()) {
                let mid_price = (best_bid.price.value() + best_ask.price.value())
                    / rust_decimal::Decimal::new(2, 0);
                cache.insert(
                    delta.symbol.clone(),
                    (Price::new(mid_price), std::time::Instant::now()),
                );
            }
        }
    }

    /// Get price from cache
    fn get_price_from_cache(&self, exchange: &str, symbol: &Symbol) -> Option<Price> {
        if let Some(cache) = self.price_cache.get(exchange) {
            if let Some((price, timestamp)) = cache.get(symbol) {
                // Check if price is still fresh (within last 5 seconds)
                if timestamp.elapsed().as_secs() < 5 {
                    return Some(*price);
                }
            }
        }
        None
    }

    /// Identify arbitrage opportunities across exchanges
    fn identify_opportunities(&mut self) -> Vec<ArbitrageOpportunity> {
        let mut opportunities = Vec::new();
        let exchange_names: Vec<String> = self.price_cache.keys().cloned().collect();

        // For each pair of exchanges, check for arbitrage opportunities
        for i in 0..exchange_names.len() {
            for j in (i + 1)..exchange_names.len() {
                let exchange_a = &exchange_names[i];
                let exchange_b = &exchange_names[j];

                // Get common symbols between the two exchanges
                let symbols_a: Vec<Symbol> = self
                    .price_cache
                    .get(exchange_a)
                    .map(|c| c.keys().cloned().collect())
                    .unwrap_or_default();
                let symbols_b: Vec<Symbol> = self
                    .price_cache
                    .get(exchange_b)
                    .map(|c| c.keys().cloned().collect())
                    .unwrap_or_default();

                for symbol in symbols_a.iter().filter(|s| symbols_b.contains(s)) {
                    if let (Some(price_a), Some(price_b)) = (
                        self.get_price_from_cache(exchange_a, symbol),
                        self.get_price_from_cache(exchange_b, symbol),
                    ) {
                        // Check if there's an arbitrage opportunity
                        // We need to compare bid on one exchange with ask on another
                        // For simplicity, we'll use mid prices and assume we can buy at mid and sell at mid
                        // In reality, we'd need to track best bid/ask separately
                        if price_a < price_b {
                            let spread = price_b - price_a;
                            let spread_percentage = spread.value() / price_a.value();

                            // Convert min_spread_bps from basis points to decimal (e.g., 5 bps = 0.0005)
                            let min_spread_decimal =
                                self.config.min_spread_bps / rust_decimal::Decimal::new(10000, 0);

                            if spread_percentage >= min_spread_decimal {
                                let estimated_profit =
                                    spread.value() * self.config.max_position_size.value();
                                let opportunity = ArbitrageOpportunity {
                                    symbol: symbol.clone(),
                                    exchange_buy: exchange_a.clone(),
                                    exchange_sell: exchange_b.clone(),
                                    price_buy: price_a,
                                    price_sell: price_b,
                                    spread,
                                    spread_percentage,
                                    estimated_profit,
                                    timestamp: std::time::Instant::now(),
                                };
                                opportunities.push(opportunity);
                            }
                        } else if price_b < price_a {
                            let spread = price_a - price_b;
                            let spread_percentage = spread.value() / price_b.value();

                            let min_spread_decimal =
                                self.config.min_spread_bps / rust_decimal::Decimal::new(10000, 0);

                            if spread_percentage >= min_spread_decimal {
                                let estimated_profit =
                                    spread.value() * self.config.max_position_size.value();
                                let opportunity = ArbitrageOpportunity {
                                    symbol: symbol.clone(),
                                    exchange_buy: exchange_b.clone(),
                                    exchange_sell: exchange_a.clone(),
                                    price_buy: price_b,
                                    price_sell: price_a,
                                    spread,
                                    spread_percentage,
                                    estimated_profit,
                                    timestamp: std::time::Instant::now(),
                                };
                                opportunities.push(opportunity);
                            }
                        }
                    }
                }
            }
        }

        // Sort opportunities by estimated profit (descending)
        opportunities.sort_by(|a, b| b.estimated_profit.cmp(&a.estimated_profit));
        opportunities
    }

    /// Execute an arbitrage opportunity
    /// Note: This method creates the trade record but doesn't actually place orders
    /// Order placement is handled by the signal generation
    fn record_opportunity_execution(&mut self, opportunity: ArbitrageOpportunity) -> String {
        let trade_id = format!("arb_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));

        info!(
            "Recording arbitrage opportunity: {} on {} -> {}",
            trade_id, opportunity.exchange_buy, opportunity.exchange_sell
        );

        // Create arbitrage trade record
        let trade = ArbitrageTrade {
            id: trade_id.clone(),
            symbol: opportunity.symbol.clone(),
            exchange_buy: opportunity.exchange_buy.clone(),
            exchange_sell: opportunity.exchange_sell.clone(),
            price_buy: opportunity.price_buy,
            price_sell: opportunity.price_sell,
            size: self.config.max_position_size,
            buy_order_id: None,
            sell_order_id: None,
            status: ArbitrageTradeStatus::Pending,
            timestamp: std::time::Instant::now(),
        };

        // Add to executed trades
        self.state.executed_trades.push(trade);

        // Update metrics
        self.metrics.total_trades += 1;

        trade_id
    }

    /// Clean up expired opportunities
    fn cleanup_expired_opportunities(&mut self) {
        let now = std::time::Instant::now();
        self.state.active_opportunities.retain(|_, opportunity| {
            now.duration_since(opportunity.timestamp).as_millis()
                < self.config.opportunity_timeout_ms as u128
        });
    }
}

#[async_trait]
impl Strategy for ArbitrageStrategy {
    type Error = ArbitrageError;

    async fn initialize(&mut self, config: StrategyConfig) -> Result<(), Self::Error> {
        info!("Initializing arbitrage strategy with config: {:?}", config);

        // Initialize price cache for all exchanges
        for exchange_name in &config.exchanges {
            if !self.price_cache.contains_key(exchange_name) {
                self.price_cache
                    .insert(exchange_name.clone(), HashMap::new());
            }
        }

        Ok(())
    }

    async fn on_market_event(&mut self, event: MarketEvent) -> Result<Vec<Signal>, Self::Error> {
        debug!("Processing market event: {:?}", event);

        // Update price cache with new market data
        match &event {
            MarketEvent::OrderBookSnapshot(snapshot) => {
                self.update_price_cache_from_snapshot(snapshot);
            }
            MarketEvent::OrderBookDelta(delta) => {
                self.update_price_cache_from_delta(delta);
            }
            MarketEvent::Trade(_) => {
                // Trades don't directly update price cache
                // In a real implementation, we might track trade prices
            }
        }

        // Identify arbitrage opportunities
        let opportunities = self.identify_opportunities();

        // Clean up expired opportunities
        self.cleanup_expired_opportunities();

        // Process new opportunities and generate signals
        let mut signals = Vec::new();
        for opportunity in opportunities {
            let opportunity_id = format!(
                "{}_{}_{}",
                opportunity.symbol.to_string(),
                opportunity.exchange_buy,
                opportunity.exchange_sell
            );

            // Check if we already have this opportunity
            if !self
                .state
                .active_opportunities
                .contains_key(&opportunity_id)
            {
                self.state
                    .active_opportunities
                    .insert(opportunity_id.clone(), opportunity.clone());

                // Record the opportunity execution
                let trade_id = self.record_opportunity_execution(opportunity.clone());

                // Generate buy order signal for the cheaper exchange
                let buy_order = NewOrder {
                    symbol: opportunity.symbol.clone(),
                    exchange_id: opportunity.exchange_buy.clone(),
                    side: OrderSide::Buy,
                    order_type: OrderType::Limit,
                    time_in_force: TimeInForce::ImmediateOrCancel, // Use IOC for arbitrage
                    price: Some(opportunity.price_buy),
                    size: self.config.max_position_size,
                    client_order_id: Some(format!("arb_buy_{}", trade_id)),
                };

                let buy_signal = Signal::PlaceOrder { order: buy_order };

                // Validate signal if validator is available
                let mut should_execute = true;
                if let Some(validator) = &self.signal_validator {
                    if let Err(e) = validator
                        .validate_signal(&buy_signal, &StrategyState::Arbitrage(self.state.clone()))
                    {
                        warn!("Signal validation failed: {}", e);
                        should_execute = false;
                    }
                }

                // Risk manager check disabled pending trait refactoring
                // TODO: Re-enable when RiskManager trait error types are fixed
                let _ = &self.risk_manager; // Suppress unused warning

                if should_execute {
                    signals.push(buy_signal);

                    // Generate sell order signal for the more expensive exchange
                    // In a real implementation, this would be placed after the buy order is filled
                    let sell_order = NewOrder {
                        symbol: opportunity.symbol.clone(),
                        exchange_id: opportunity.exchange_sell.clone(),
                        side: OrderSide::Sell,
                        order_type: OrderType::Limit,
                        time_in_force: TimeInForce::ImmediateOrCancel,
                        price: Some(opportunity.price_sell),
                        size: self.config.max_position_size,
                        client_order_id: Some(format!("arb_sell_{}", trade_id)),
                    };

                    let sell_signal = Signal::PlaceOrder { order: sell_order };
                    signals.push(sell_signal);
                }
            }
        }

        Ok(signals)
    }

    async fn on_trading_event(&mut self, event: TradingEvent) -> Result<(), Self::Error> {
        debug!("Processing trading event: {:?}", event);

        // Position manager update disabled pending trait refactoring
        // TODO: Re-enable when PositionManager trait error types are fixed
        let _ = &self.position_manager; // Suppress unused warning

        // Update arbitrage trade status based on execution reports
        match &event {
            TradingEvent::ExecutionReport(report) => {
                for trade in &mut self.state.executed_trades {
                    // Check if this execution report matches our buy order
                    if let Some(buy_order_id) = &trade.buy_order_id {
                        if report.order_id == *buy_order_id {
                            match report.status {
                                OrderStatus::Filled => {
                                    if trade.status == ArbitrageTradeStatus::Pending {
                                        trade.status = ArbitrageTradeStatus::BuyPlaced;
                                    }
                                }
                                OrderStatus::Cancelled | OrderStatus::Rejected => {
                                    trade.status = ArbitrageTradeStatus::Failed;
                                }
                                _ => {}
                            }
                        }
                    }

                    // Check if this execution report matches our sell order
                    if let Some(sell_order_id) = &trade.sell_order_id {
                        if report.order_id == *sell_order_id {
                            match report.status {
                                OrderStatus::Filled => {
                                    if trade.status == ArbitrageTradeStatus::BuyPlaced {
                                        trade.status = ArbitrageTradeStatus::Completed;

                                        // Update metrics
                                        let profit_value = (trade.price_sell.value()
                                            - trade.price_buy.value())
                                            * trade.size.value();
                                        self.metrics.total_pnl += profit_value;

                                        if profit_value > rust_decimal::Decimal::ZERO {
                                            self.metrics.winning_trades += 1;
                                            self.metrics.gross_profit += profit_value;
                                        } else {
                                            self.metrics.losing_trades += 1;
                                            self.metrics.gross_loss += profit_value.abs();
                                        }

                                        // Calculate win rate
                                        if self.metrics.total_trades > 0 {
                                            self.metrics.win_rate = rust_decimal::Decimal::from(
                                                self.metrics.winning_trades,
                                            ) / rust_decimal::Decimal::from(
                                                self.metrics.total_trades,
                                            );
                                        }

                                        // Calculate average trade PnL
                                        if self.metrics.total_trades > 0 {
                                            self.metrics.average_trade_pnl = self.metrics.total_pnl
                                                / rust_decimal::Decimal::from(
                                                    self.metrics.total_trades,
                                                );
                                        }

                                        // Calculate profit factor
                                        if self.metrics.gross_loss != rust_decimal::Decimal::ZERO {
                                            self.metrics.profit_factor =
                                                self.metrics.gross_profit / self.metrics.gross_loss;
                                        }
                                    }
                                }
                                OrderStatus::Cancelled | OrderStatus::Rejected => {
                                    trade.status = ArbitrageTradeStatus::Failed;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn get_state(&self) -> StrategyState {
        StrategyState::Arbitrage(self.state.clone())
    }

    fn get_metrics(&self) -> StrategyMetrics {
        self.metrics.clone()
    }

    async fn shutdown(&mut self) -> Result<(), Self::Error> {
        info!("Shutting down arbitrage strategy");

        // Note: In a real implementation, we would cancel all pending orders here
        // For now, we just clear the state
        self.state.active_opportunities.clear();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arbitrage_config_default() {
        let config = ArbitrageConfig::default();
        assert_eq!(config.min_spread_bps, rust_decimal::Decimal::new(5, 2));
        assert_eq!(config.max_position_size, Size::from_str("0.1").unwrap());
        assert_eq!(config.max_exposure, rust_decimal::Decimal::new(1000, 2));
        assert_eq!(config.slippage_tolerance, rust_decimal::Decimal::new(1, 3));
        assert_eq!(config.execution_delay_ms, 100);
        assert_eq!(config.opportunity_timeout_ms, 5000);
    }

    #[test]
    fn test_arbitrage_strategy_new() {
        let strategy = ArbitrageStrategy::new();
        assert_eq!(
            strategy.config.min_spread_bps,
            rust_decimal::Decimal::new(5, 2)
        );
        assert_eq!(strategy.state.active_opportunities.len(), 0);
        assert_eq!(strategy.state.executed_trades.len(), 0);
    }

    #[test]
    fn test_arbitrage_strategy_with_config() {
        let config = ArbitrageConfig {
            min_spread_bps: rust_decimal::Decimal::new(10, 2),
            max_position_size: Size::from_str("0.5").unwrap(),
            max_exposure: rust_decimal::Decimal::new(5000, 2),
            slippage_tolerance: rust_decimal::Decimal::new(2, 3),
            execution_delay_ms: 200,
            opportunity_timeout_ms: 10000,
        };

        let strategy = ArbitrageStrategy::with_config(config);
        assert_eq!(
            strategy.config.min_spread_bps,
            rust_decimal::Decimal::new(10, 2)
        );
        assert_eq!(
            strategy.config.max_position_size,
            Size::from_str("0.5").unwrap()
        );
        assert_eq!(
            strategy.config.max_exposure,
            rust_decimal::Decimal::new(5000, 2)
        );
        assert_eq!(
            strategy.config.slippage_tolerance,
            rust_decimal::Decimal::new(2, 3)
        );
        assert_eq!(strategy.config.execution_delay_ms, 200);
        assert_eq!(strategy.config.opportunity_timeout_ms, 10000);
    }

    #[test]
    fn test_initialize_exchange_cache() {
        let mut strategy = ArbitrageStrategy::new();

        strategy.initialize_exchange_cache("test".to_string());
        assert!(strategy.price_cache.contains_key("test"));
    }

    #[test]
    fn test_get_price_from_cache() {
        let mut strategy = ArbitrageStrategy::new();
        strategy.initialize_exchange_cache("test".to_string());

        let symbol = Symbol::new("BTCUSDT");
        let price = Price::from_str("50000.0").unwrap();

        if let Some(cache) = strategy.price_cache.get_mut("test") {
            cache.insert(symbol.clone(), (price, std::time::Instant::now()));
        }

        assert_eq!(strategy.get_price_from_cache("test", &symbol), Some(price));
    }

    #[tokio::test]
    async fn test_initialize() {
        let mut strategy = ArbitrageStrategy::new();

        let config = StrategyConfig {
            strategy_type: "arbitrage".to_string(),
            symbols: vec![Symbol::new("BTCUSDT")],
            exchanges: vec!["binance".to_string(), "okx".to_string()],
            parameters: HashMap::new(),
        };

        assert!(strategy.initialize(config).await.is_ok());
        assert!(strategy.price_cache.contains_key("binance"));
        assert!(strategy.price_cache.contains_key("okx"));
    }

    #[tokio::test]
    async fn test_shutdown() {
        let mut strategy = ArbitrageStrategy::new();
        assert!(strategy.shutdown().await.is_ok());
    }
}
