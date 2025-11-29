# Strategy API Contract

**Date**: 2025-11-27  
**Feature**: High-Frequency Market Making System

## Overview

This document defines the standardized API contract for trading strategies in the high-frequency market making system. All strategy implementations must adhere to this contract to ensure compatibility with the core system.

## Core Strategy Interface

### Strategy Trait

```rust
#[async_trait]
pub trait Strategy: Send + Sync {
    /// Initialize the strategy with configuration
    async fn initialize(&mut self, config: &StrategyConfig) -> Result<(), StrategyError>;
    
    /// Process a market event and generate signals
    async fn on_market_event(&mut self, event: &MarketEvent) -> Result<Vec<Signal>, StrategyError>;
    
    /// Process a trading event and update state
    async fn on_trading_event(&mut self, event: &TradingEvent) -> Result<(), StrategyError>;
    
    /// Get current strategy state
    fn get_state(&self) -> StrategyState;
    
    /// Get strategy performance metrics
    fn get_metrics(&self) -> StrategyMetrics;
    
    /// Shutdown the strategy gracefully
    async fn shutdown(&mut self) -> Result<(), StrategyError>;
}
```

**Description**: Core interface that all strategies must implement  
**Usage**: Implemented by all trading strategies

## Market Making Strategy

### Market Making Configuration

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketMakingConfig {
    pub symbol: Symbol,
    pub exchange_id: ExchangeId,
    pub spread_bps: u32,              // Spread in basis points
    pub order_size: Size,              // Fixed order size
    pub target_inventory_ratio: Decimal, // Target inventory ratio (0.5 = balanced)
    pub max_position: Size,            // Maximum position size
    pub min_spread_bps: u32,           // Minimum spread in basis points
    pub max_spread_bps: u32,           // Maximum spread in basis points
    pub inventory_tolerance: Decimal,    // Tolerance for inventory imbalance
    pub order_refresh_seconds: u64,     // How often to refresh orders
    pub price_levels: u32,              // Number of price levels to maintain
}
```

**Description**: Configuration parameters for market making strategy  
**Validation**: All values must be within reasonable ranges

### Market Making State

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketMakingState {
    pub current_position: Size,
    pub target_position: Size,
    pub active_orders: Vec<Order>,
    pub last_update: Timestamp,
    pub mid_price: Option<Price>,
    pub current_spread: Decimal,
    pub inventory_ratio: Decimal,
}
```

**Description**: Current state of the market making strategy  
**Usage**: Maintained by strategy implementation

### Market Making Implementation

```rust
pub struct MarketMakingStrategy {
    config: MarketMakingConfig,
    state: MarketMakingState,
    order_book: Option<OrderBookSnapshot>,
    indicators: Option<OrderBookIndicator>,
}

impl Strategy for MarketMakingStrategy {
    async fn initialize(&mut self, config: &StrategyConfig) -> Result<(), StrategyError> {
        // Parse configuration
        // Initialize state
        // Validate parameters
    }
    
    async fn on_market_event(&mut self, event: &MarketEvent) -> Result<Vec<Signal>, StrategyError> {
        match event {
            MarketEvent::OrderBookSnapshot(snapshot) => {
                self.update_order_book(snapshot);
                self.calculate_indicators();
                self.generate_signals()
            }
            MarketEvent::OrderBookDelta(delta) => {
                self.apply_order_book_delta(delta);
                self.calculate_indicators();
                self.generate_signals()
            }
            _ => Ok(vec![])
        }
    }
    
    async fn on_trading_event(&mut self, event: &TradingEvent) -> Result<(), StrategyError> {
        match event {
            TradingEvent::ExecutionReport(report) => {
                self.update_position(report);
                self.cancel_stale_orders()
            }
            _ => Ok(())
        }
    }
    
    fn get_state(&self) -> StrategyState {
        StrategyState::MarketMaking(self.state.clone())
    }
    
    fn get_metrics(&self) -> StrategyMetrics {
        // Calculate and return performance metrics
    }
    
    async fn shutdown(&mut self) -> Result<(), StrategyError> {
        // Cancel all active orders
        // Clean up resources
    }
}
```

**Description**: Implementation of market making strategy  
**Usage**: Used for providing liquidity on exchanges

## Arbitrage Strategy

### Arbitrage Configuration

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArbitrageConfig {
    pub symbol: Symbol,
    pub exchanges: Vec<ExchangeId>,    // Exchanges to monitor
    pub min_profit_bps: u32,            // Minimum profit in basis points
    pub order_size: Size,                // Fixed order size
    pub max_position: Size,              // Maximum position size
    pub execution_timeout_ms: u64,        // Timeout for execution
    pub price_update_threshold: Decimal, // Minimum price change to trigger update
    pub max_latency_ms: u64,            // Maximum acceptable latency
}
```

**Description**: Configuration parameters for arbitrage strategy  
**Validation**: All values must be within reasonable ranges

### Arbitrage State

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArbitrageState {
    pub order_books: HashMap<ExchangeId, OrderBookSnapshot>,
    pub best_bid: Option<(ExchangeId, Price)>,
    pub best_ask: Option<(ExchangeId, Price)>,
    pub current_positions: HashMap<ExchangeId, Size>,
    pub active_arbitrages: Vec<ArbitrageOpportunity>,
    pub last_update: Timestamp,
}
```

**Description**: Current state of the arbitrage strategy  
**Usage**: Maintained by strategy implementation

### Arbitrage Opportunity

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArbitrageOpportunity {
    pub symbol: Symbol,
    pub buy_exchange: ExchangeId,
    pub sell_exchange: ExchangeId,
    pub buy_price: Price,
    pub sell_price: Price,
    pub profit_bps: u32,
    pub timestamp: Timestamp,
}
```

**Description**: Represents an arbitrage opportunity  
**Usage**: Generated by strategy when profitable opportunity found

### Arbitrage Implementation

```rust
pub struct ArbitrageStrategy {
    config: ArbitrageConfig,
    state: ArbitrageState,
}

impl Strategy for ArbitrageStrategy {
    async fn initialize(&mut self, config: &StrategyConfig) -> Result<(), StrategyError> {
        // Parse configuration
        // Initialize state
        // Validate parameters
    }
    
    async fn on_market_event(&mut self, event: &MarketEvent) -> Result<Vec<Signal>, StrategyError> {
        match event {
            MarketEvent::OrderBookSnapshot(snapshot) => {
                self.update_order_book(snapshot);
                self.find_arbitrage_opportunities()
            }
            MarketEvent::OrderBookDelta(delta) => {
                self.apply_order_book_delta(delta);
                self.find_arbitrage_opportunities()
            }
            _ => Ok(vec![])
        }
    }
    
    async fn on_trading_event(&mut self, event: &TradingEvent) -> Result<(), StrategyError> {
        match event {
            TradingEvent::ExecutionReport(report) => {
                self.update_position(report);
                self.check_arbitrage_completion()
            }
            _ => Ok(())
        }
    }
    
    fn get_state(&self) -> StrategyState {
        StrategyState::Arbitrage(self.state.clone())
    }
    
    fn get_metrics(&self) -> StrategyMetrics {
        // Calculate and return performance metrics
    }
    
    async fn shutdown(&mut self) -> Result<(), StrategyError> {
        // Cancel all active orders
        // Clean up resources
    }
}
```

**Description**: Implementation of arbitrage strategy  
**Usage**: Used for cross-exchange arbitrage

## Prediction-Based Strategy

### Prediction Configuration

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PredictionConfig {
    pub symbol: Symbol,
    pub exchange_id: ExchangeId,
    pub order_size: Size,                // Fixed order size
    pub max_position: Size,              // Maximum position size
    pub prediction_horizon_seconds: u64,  // Prediction horizon (5 seconds)
    pub min_confidence: Decimal,         // Minimum confidence to act
    pub lookback_seconds: u64,           // Data lookback period
    pub update_interval_ms: u64,         // Model update interval
    pub spread_bps: u32,                // Base spread in basis points
    pub confidence_adjustment: bool,      // Adjust spread based on confidence
}
```

**Description**: Configuration parameters for prediction-based strategy  
**Validation**: All values must be within reasonable ranges

### Prediction State

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredictionState {
    pub current_position: Size,
    pub active_orders: Vec<Order>,
    pub last_prediction: Option<PricePrediction>,
    pub order_book: Option<OrderBookSnapshot>,
    pub indicators: Option<OrderBookIndicator>,
    pub model: LinearModel,
    pub last_update: Timestamp,
}
```

**Description**: Current state of the prediction-based strategy  
**Usage**: Maintained by strategy implementation

### Price Prediction

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PricePrediction {
    pub symbol: Symbol,
    pub exchange_id: ExchangeId,
    pub current_price: Price,
    pub predicted_price: Price,
    pub confidence: Decimal,
    pub horizon_seconds: u64,
    pub timestamp: Timestamp,
}
```

**Description**: Short-term price prediction  
**Usage**: Generated by linear model

### Linear Model

```rust
pub struct LinearModel {
    weights: Vec<f64>,
    bias: f64,
    learning_rate: f64,
    regularization: f64,
    features: Vec<String>,
}

impl LinearModel {
    pub fn new(features: Vec<String>) -> Self;
    pub fn predict(&self, features: &[f64]) -> f64;
    pub fn update(&mut self, features: &[f64], target: f64);
    pub fn extract_features(&self, order_book: &OrderBookSnapshot, trades: &[Trade]) -> Vec<f64>;
}
```

**Description**: Linear regression model for price prediction  
**Usage**: Used for short-term price prediction

### Prediction Implementation

```rust
pub struct PredictionStrategy {
    config: PredictionConfig,
    state: PredictionState,
    recent_trades: VecDeque<Trade>,
}

impl Strategy for PredictionStrategy {
    async fn initialize(&mut self, config: &StrategyConfig) -> Result<(), StrategyError> {
        // Parse configuration
        // Initialize model
        // Initialize state
    }
    
    async fn on_market_event(&mut self, event: &MarketEvent) -> Result<Vec<Signal>, StrategyError> {
        match event {
            MarketEvent::OrderBookSnapshot(snapshot) => {
                self.update_order_book(snapshot);
                self.update_model();
                self.generate_signals()
            }
            MarketEvent::OrderBookDelta(delta) => {
                self.apply_order_book_delta(delta);
                self.update_model();
                self.generate_signals()
            }
            MarketEvent::Trade(trade) => {
                self.add_trade(trade);
                self.update_model();
                self.generate_signals()
            }
        }
    }
    
    async fn on_trading_event(&mut self, event: &TradingEvent) -> Result<(), StrategyError> {
        match event {
            TradingEvent::ExecutionReport(report) => {
                self.update_position(report);
                self.cancel_stale_orders()
            }
            _ => Ok(())
        }
    }
    
    fn get_state(&self) -> StrategyState {
        StrategyState::Prediction(self.state.clone())
    }
    
    fn get_metrics(&self) -> StrategyMetrics {
        // Calculate and return performance metrics
    }
    
    async fn shutdown(&mut self) -> Result<(), StrategyError> {
        // Cancel all active orders
        // Clean up resources
    }
}
```

**Description**: Implementation of prediction-based strategy  
**Usage**: Used for short-term price prediction and trading

## Signal Generation

### Signal Types

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signal {
    PlaceOrder {
        order: NewOrder,
    },
    CancelOrder {
        order_id: OrderId,
        symbol: Symbol,
        exchange_id: ExchangeId,
    },
    CancelAllOrders {
        symbol: Symbol,
        exchange_id: ExchangeId,
    },
    UpdateOrder {
        order_id: OrderId,
        price: Option<Price>,
        size: Option<Size>,
    },
}
```

**Description**: Types of signals generated by strategies  
**Usage**: Sent to order management system

### Signal Validation

```rust
pub trait SignalValidator: Send + Sync {
    fn validate_signal(&self, signal: &Signal, state: &StrategyState) -> Result<(), StrategyError>;
}
```

**Description**: Interface for signal validation  
**Usage**: Used to validate signals before execution

## Error Handling

### Strategy Error

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrategyError {
    InvalidConfig(String),
    InvalidState(String),
    InvalidSignal(String),
    ModelError(String),
    DataError(String),
    ExecutionError(String),
}
```

**Description**: Enumeration of possible strategy errors  
**Usage**: Returned by strategy methods on failure

## Performance Metrics

### Strategy Metrics

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrategyMetrics {
    pub total_trades: u64,
    pub winning_trades: u64,
    pub losing_trades: u64,
    pub total_pnl: Decimal,
    pub gross_profit: Decimal,
    pub gross_loss: Decimal,
    pub profit_factor: Decimal,
    pub max_drawdown: Decimal,
    pub sharpe_ratio: Decimal,
    pub average_trade_pnl: Decimal,
    pub win_rate: Decimal,
    pub average_holding_time_ms: u64,
}
```

**Description**: Performance metrics for strategy  
**Usage**: Returned by strategy implementation

## Testing Requirements

### Unit Testing

1. Test all strategy initialization scenarios
2. Test signal generation with various market conditions
3. Test state updates with different events
4. Test error handling for edge cases

### Integration Testing

1. Test strategy with real market data
2. Test strategy with mock exchange connectors
3. Test strategy performance under load
4. Test strategy interaction with risk management

### Performance Testing

1. Measure signal generation latency
2. Measure memory usage
3. Measure CPU utilization
4. Test with high-frequency data streams

## Implementation Guidelines

### Performance Considerations

1. Minimize allocations in hot paths
2. Use efficient data structures
3. Implement caching where appropriate
4. Optimize feature extraction

### Reliability Considerations

1. Handle all error cases
2. Implement graceful degradation
3. Maintain consistent state
4. Provide detailed logging

### Maintainability Considerations

1. Use clear naming conventions
2. Document complex algorithms
3. Modularize components
4. Provide comprehensive tests
