# Data Model: High-Frequency Market Making System

**Date**: 2025-11-27  
**Feature**: High-Frequency Market Making System

## Core Types

### Price

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Price(pub Decimal);
```

**Description**: Represents a price value with decimal precision  
**Validation**: Must be positive for most use cases  
**Operations**: Addition, subtraction, multiplication, division with Size types

### Size

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Size(pub Decimal);
```

**Description**: Represents a quantity or size with decimal precision  
**Validation**: Must be non-negative  
**Operations**: Addition, subtraction, multiplication, division with Price types

### Symbol

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Symbol(String);
```

**Description**: Represents a trading symbol (e.g., "BTCUSDT")  
**Validation**: Must match exchange-specific format  
**Operations**: Equality, hashing for use as keys

### ExchangeId

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExchangeId(String);
```

**Description**: Unique identifier for an exchange  
**Validation**: Must be one of supported exchanges  
**Operations**: Equality, hashing for use as keys

### OrderId

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderId(String);
```

**Description**: Unique identifier for an order  
**Validation**: Must be unique within exchange  
**Operations**: Equality, hashing for use as keys

### Timestamp

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp(u64);
```

**Description**: Unix timestamp in milliseconds  
**Validation**: Must be reasonable (not too far in past/future)  
**Operations**: Comparison, arithmetic for time differences

## Market Data Types

### OrderBookLevel

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderBookLevel {
    pub price: Price,
    pub size: Size,
}
```

**Description**: Represents a single price level in the order book  
**Validation**: Price must be positive, Size must be non-negative  
**Relationships**: Part of OrderBookSnapshot and OrderBookDelta

### OrderBookSnapshot

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderBookSnapshot {
    pub symbol: Symbol,
    pub exchange_id: ExchangeId,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub timestamp: Timestamp,
}
```

**Description**: Complete snapshot of the order book at a point in time  
**Validation**: Bids sorted descending, Asks sorted ascending, No price overlap  
**Relationships**: Contains OrderBookLevels, associated with Symbol and Exchange

### OrderBookDelta

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderBookDelta {
    pub symbol: Symbol,
    pub exchange_id: ExchangeId,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub timestamp: Timestamp,
}
```

**Description**: Incremental updates to the order book  
**Validation**: Same as OrderBookSnapshot  
**Relationships**: Updates OrderBook, associated with Symbol and Exchange

### Trade

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trade {
    pub symbol: Symbol,
    pub exchange_id: ExchangeId,
    pub price: Price,
    pub size: Size,
    pub side: OrderSide,
    pub timestamp: Timestamp,
    pub trade_id: Option<String>,
}
```

**Description**: Represents a completed trade on an exchange  
**Validation**: Price positive, Size positive, Side valid  
**Relationships**: Associated with Symbol and Exchange

## Order Management Types

### OrderSide

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}
```

**Description**: Side of an order (buy or sell)  
**Validation**: N/A (enum)  
**Operations**: Comparison, conversion to/from string

### OrderType

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
    StopLoss,
    StopLimit,
}
```

**Description**: Type of order  
**Validation**: N/A (enum)  
**Operations**: Comparison, conversion to/from string

### TimeInForce

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    GoodTillCancelled,
    ImmediateOrCancel,
    FillOrKill,
}
```

**Description**: Time in force for orders  
**Validation**: N/A (enum)  
**Operations**: Comparison, conversion to/from string

### OrderStatus

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
    Expired,
}
```

**Description**: Current status of an order  
**Validation**: N/A (enum)  
**State Transitions**: New → PartiallyFilled → Filled, New → Cancelled, etc.

### NewOrder

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewOrder {
    pub symbol: Symbol,
    pub exchange_id: ExchangeId,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub time_in_force: TimeInForce,
    pub price: Option<Price>,
    pub size: Size,
    pub client_order_id: Option<String>,
}
```

**Description**: Request to create a new order  
**Validation**: Size positive, Price required for limit orders  
**Relationships**: Associated with Symbol and Exchange

### Order

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Order {
    pub order_id: OrderId,
    pub client_order_id: Option<String>,
    pub symbol: Symbol,
    pub exchange_id: ExchangeId,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub time_in_force: TimeInForce,
    pub price: Option<Price>,
    pub size: Size,
    pub filled_size: Size,
    pub status: OrderStatus,
    pub timestamp: Timestamp,
}
```

**Description**: Represents an order in the system  
**Validation**: Filled size ≤ Size, Status consistent with Filled Size  
**State Transitions**: Status changes based on execution reports

### ExecutionReport

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionReport {
    pub order_id: OrderId,
    pub client_order_id: Option<String>,
    pub symbol: Symbol,
    pub exchange_id: ExchangeId,
    pub status: OrderStatus,
    pub filled_size: Size,
    pub remaining_size: Size,
    pub average_price: Option<Price>,
    pub timestamp: Timestamp,
}
```

**Description**: Report of order execution status  
**Validation**: Filled + Remaining = Original Size  
**Relationships**: Updates Order status

## Strategy Types

### Signal

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Signal {
    MarketMaking {
        symbol: Symbol,
        exchange_id: ExchangeId,
        bid_price: Price,
        ask_price: Price,
        bid_size: Size,
        ask_size: Size,
    },
    Arbitrage {
        symbol: Symbol,
        buy_exchange: ExchangeId,
        sell_exchange: ExchangeId,
        buy_price: Price,
        sell_price: Price,
        size: Size,
    },
    CancelOrders {
        symbol: Symbol,
        exchange_id: ExchangeId,
        order_ids: Vec<OrderId>,
    },
}
```

**Description**: Signal generated by a strategy  
**Validation**: Prices positive, Sizes positive  
**Relationships**: Associated with Symbol and Exchange(s)

### StrategyConfig

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub strategy_type: String,
    pub symbols: Vec<Symbol>,
    pub exchanges: Vec<ExchangeId>,
    pub parameters: HashMap<String, String>,
}
```

**Description**: Configuration for a strategy  
**Validation**: Strategy type supported, Parameters valid for type  
**Relationships**: Associated with Symbols and Exchanges

## Risk Management Types

### Position

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub symbol: Symbol,
    pub exchange_id: ExchangeId,
    pub size: Size,
    pub average_price: Option<Price>,
    pub unrealized_pnl: Option<Decimal>,
}
```

**Description**: Current position for a symbol on an exchange  
**Validation**: Size can be positive (long) or negative (short)  
**Relationships**: Associated with Symbol and Exchange

### Balance

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Balance {
    pub asset: String,
    pub exchange_id: ExchangeId,
    pub total: Decimal,
    pub free: Decimal,
    pub used: Decimal,
}
```

**Description**: Balance of an asset on an exchange  
**Validation**: Total = Free + Used, All values non-negative  
**Relationships**: Associated with Exchange

### RiskRule

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskRule {
    MaxPositionSize {
        symbol: Symbol,
        max_size: Size,
    },
    MaxOrderSize {
        symbol: Symbol,
        max_size: Size,
    },
    MaxDailyLoss {
        max_loss: Decimal,
    },
    MaxDrawdown {
        max_drawdown_percent: Decimal,
    },
}
```

**Description**: Rule for risk management  
**Validation**: Sizes positive, Percentages between 0 and 100  
**Relationships**: Associated with Symbol (for specific rules)

### RiskViolation

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskViolation {
    pub rule: RiskRule,
    pub details: String,
    pub timestamp: Timestamp,
}
```

**Description**: Violation of a risk rule  
**Validation**: N/A  
**Relationships**: Associated with RiskRule

## Event Types

### MarketEvent

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketEvent {
    OrderBookSnapshot(OrderBookSnapshot),
    OrderBookDelta(OrderBookDelta),
    Trade(Trade),
}
```

**Description**: Event representing market data update  
**Validation**: Valid inner type  
**Relationships**: Contains OrderBook or Trade data

### TradingEvent

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradingEvent {
    OrderCreated(NewOrder),
    OrderUpdated(Order),
    ExecutionReport(ExecutionReport),
    SignalGenerated(Signal),
}
```

**Description**: Event representing trading activity  
**Validation**: Valid inner type  
**Relationships**: Contains Order or Signal data

### SystemEvent

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemEvent {
    ExchangeConnected(ExchangeId),
    ExchangeDisconnected(ExchangeId),
    RiskViolation(RiskViolation),
    Error(String),
}
```

**Description**: Event representing system status  
**Validation**: Valid inner type  
**Relationships**: Associated with Exchange or Risk

## Indicator Types

### OrderBookIndicator

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderBookIndicator {
    pub symbol: Symbol,
    pub exchange_id: ExchangeId,
    pub timestamp: Timestamp,
    pub spread: Decimal,
    pub mid_price: Price,
    pub bid_ask_ratio: Decimal,
    pub order_book_imbalance: Decimal,
}
```

**Description**: Indicators calculated from order book data  
**Validation**: All calculated values within reasonable ranges  
**Relationships**: Associated with Symbol and Exchange

### TradeFlowIndicator

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeFlowIndicator {
    pub symbol: Symbol,
    pub exchange_id: ExchangeId,
    pub timestamp: Timestamp,
    pub volume: Size,
    pub vwap: Option<Price>,
    pub buy_sell_ratio: Decimal,
    pub trade_count: u64,
}
```

**Description**: Indicators calculated from trade data  
**Validation**: All calculated values within reasonable ranges  
**Relationships**: Associated with Symbol and Exchange

### PricePrediction

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PricePrediction {
    pub symbol: Symbol,
    pub exchange_id: ExchangeId,
    pub timestamp: Timestamp,
    pub horizon_seconds: u64,
    pub predicted_price: Price,
    pub confidence: Decimal,
}
```

**Description**: Short-term price prediction  
**Validation**: Confidence between 0 and 1, Horizon positive  
**Relationships**: Associated with Symbol and Exchange

## Configuration Types

### ExchangeConfig

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExchangeConfig {
    pub exchange_id: ExchangeId,
    pub api_key: String,
    pub api_secret: String,
    pub sandbox: bool,
    pub rate_limits: HashMap<String, u32>,
}
```

**Description**: Configuration for an exchange connection  
**Validation**: API keys non-empty, Rate limits positive  
**Relationships**: Associated with Exchange

### SystemConfig

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemConfig {
    pub exchanges: Vec<ExchangeConfig>,
    pub strategies: Vec<StrategyConfig>,
    pub risk_rules: Vec<RiskRule>,
    pub logging: LoggingConfig,
}
```

**Description**: Complete system configuration  
**Validation**: All sub-configurations valid  
**Relationships**: Contains all other configuration types

### LoggingConfig

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file_path: Option<String>,
    pub max_file_size: Option<u64>,
    pub max_files: Option<u32>,
}
```

**Description**: Configuration for logging  
**Validation**: Level valid, File sizes positive  
**Relationships**: Part of SystemConfig
