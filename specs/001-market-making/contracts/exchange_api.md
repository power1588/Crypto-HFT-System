# Exchange API Contract

**Date**: 2025-11-27  
**Feature**: High-Frequency Market Making System

## Overview

This document defines the standardized API contract for exchange connectors in the high-frequency market making system. All exchange implementations must adhere to this contract to ensure compatibility with the core system.

## Market Data API

### WebSocket Connection

#### Connect

```rust
async fn connect(&mut self) -> Result<(), ExchangeError>
```

**Description**: Establishes WebSocket connection to the exchange  
**Parameters**: None  
**Returns**: Result indicating success or error  
**Error Handling**: Returns ExchangeError with connection details

#### Disconnect

```rust
async fn disconnect(&mut self) -> Result<(), ExchangeError>
```

**Description**: Closes WebSocket connection to the exchange  
**Parameters**: None  
**Returns**: Result indicating success or error  
**Error Handling**: Returns ExchangeError with disconnection details

#### Subscribe to Order Book

```rust
async fn subscribe_order_book(&mut self, symbol: &Symbol) -> Result<(), ExchangeError>
```

**Description**: Subscribes to order book updates for a symbol  
**Parameters**: Symbol to subscribe to  
**Returns**: Result indicating success or error  
**Error Handling**: Returns ExchangeError with subscription details

#### Subscribe to Trades

```rust
async fn subscribe_trades(&mut self, symbol: &Symbol) -> Result<(), ExchangeError>
```

**Description**: Subscribes to trade updates for a symbol  
**Parameters**: Symbol to subscribe to  
**Returns**: Result indicating success or error  
**Error Handling**: Returns ExchangeError with subscription details

#### Get Next Market Event

```rust
async fn next_market_event(&mut self) -> Option<MarketEvent>
```

**Description**: Retrieves the next market event from the exchange  
**Parameters**: None  
**Returns**: Option containing MarketEvent or None if no events available  
**Error Handling**: Returns None on connection errors

### REST API

#### Get Order Book Snapshot

```rust
async fn get_order_book(&self, symbol: &Symbol, limit: Option<u32>) -> Result<OrderBookSnapshot, ExchangeError>
```

**Description**: Retrieves current order book snapshot for a symbol  
**Parameters**: Symbol to retrieve, optional limit for number of levels  
**Returns**: OrderBookSnapshot with current market state  
**Error Handling**: Returns ExchangeError with API details

#### Get Recent Trades

```rust
async fn get_recent_trades(&self, symbol: &Symbol, limit: Option<u32>) -> Result<Vec<Trade>, ExchangeError>
```

**Description**: Retrieves recent trades for a symbol  
**Parameters**: Symbol to retrieve, optional limit for number of trades  
**Returns**: Vector of Trade objects  
**Error Handling**: Returns ExchangeError with API details

#### Get Server Time

```rust
async fn get_server_time(&self) -> Result<Timestamp, ExchangeError>
```

**Description**: Retrieves current server time from exchange  
**Parameters**: None  
**Returns**: Timestamp with server time  
**Error Handling**: Returns ExchangeError with API details

## Trading API

### Place Order

```rust
async fn place_order(&self, order: &NewOrder) -> Result<OrderId, ExchangeError>
```

**Description**: Places a new order on the exchange  
**Parameters**: NewOrder with order details  
**Returns**: OrderId assigned by exchange  
**Error Handling**: Returns ExchangeError with order details

### Cancel Order

```rust
async fn cancel_order(&self, order_id: &OrderId, symbol: &Symbol) -> Result<(), ExchangeError>
```

**Description**: Cancels an existing order on the exchange  
**Parameters**: OrderId to cancel, Symbol of the order  
**Returns**: Result indicating success or error  
**Error Handling**: Returns ExchangeError with cancellation details

### Cancel All Orders

```rust
async fn cancel_all_orders(&self, symbol: &Symbol) -> Result<Vec<OrderId>, ExchangeError>
```

**Description**: Cancels all orders for a symbol on the exchange  
**Parameters**: Symbol to cancel orders for  
**Returns**: Vector of OrderId that were cancelled  
**Error Handling**: Returns ExchangeError with cancellation details

### Get Order Status

```rust
async fn get_order_status(&self, order_id: &OrderId, symbol: &Symbol) -> Result<Order, ExchangeError>
```

**Description**: Retrieves current status of an order  
**Parameters**: OrderId to query, Symbol of the order  
**Returns**: Order with current status  
**Error Handling**: Returns ExchangeError with query details

### Get Open Orders

```rust
async fn get_open_orders(&self, symbol: Option<&Symbol>) -> Result<Vec<Order>, ExchangeError>
```

**Description**: Retrieves all open orders, optionally filtered by symbol  
**Parameters**: Optional Symbol to filter by  
**Returns**: Vector of Order objects  
**Error Handling**: Returns ExchangeError with query details

### Get Order History

```rust
async fn get_order_history(&self, symbol: Option<&Symbol>, limit: Option<u32>) -> Result<Vec<Order>, ExchangeError>
```

**Description**: Retrieves order history, optionally filtered by symbol and limited  
**Parameters**: Optional Symbol to filter by, optional limit for number of orders  
**Returns**: Vector of Order objects  
**Error Handling**: Returns ExchangeError with query details

## Account API

### Get Account Balance

```rust
async fn get_account_balance(&self) -> Result<Vec<Balance>, ExchangeError>
```

**Description**: Retrieves account balance for all assets  
**Parameters**: None  
**Returns**: Vector of Balance objects  
**Error Handling**: Returns ExchangeError with query details

### Get Positions

```rust
async fn get_positions(&self, symbol: Option<&Symbol>) -> Result<Vec<Position>, ExchangeError>
```

**Description**: Retrieves current positions, optionally filtered by symbol  
**Parameters**: Optional Symbol to filter by  
**Returns**: Vector of Position objects  
**Error Handling**: Returns ExchangeError with query details

## Error Handling

### ExchangeError

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExchangeError {
    ConnectionError(String),
    AuthenticationError(String),
    RateLimitError(String),
    InvalidRequest(String),
    OrderNotFound(String),
    InsufficientFunds(String),
    SymbolNotFound(String),
    ExchangeError(String),
    UnknownError(String),
}
```

**Description**: Enumeration of possible exchange errors  
**Usage**: Returned by all API methods on failure

## Rate Limiting

### Rate Limits

Each exchange implementation must respect rate limits defined in the exchange configuration:

```rust
pub struct RateLimit {
    pub requests_per_second: u32,
    pub requests_per_minute: u32,
    pub requests_per_day: u32,
}
```

**Description**: Defines rate limits for API calls  
**Usage**: Must be enforced by exchange implementation

## Event Handling

### Market Events

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarketEvent {
    OrderBookSnapshot(OrderBookSnapshot),
    OrderBookDelta(OrderBookDelta),
    Trade(Trade),
}
```

**Description**: Events generated by market data updates  
**Usage**: Returned by market data API methods

### Trading Events

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradingEvent {
    OrderCreated(NewOrder),
    OrderUpdated(Order),
    ExecutionReport(ExecutionReport),
}
```

**Description**: Events generated by trading activities  
**Usage**: Generated by trading API methods

## Data Normalization

### Symbol Normalization

All exchanges must normalize symbols to a standard format:

```rust
pub fn normalize_symbol(symbol: &str) -> Symbol
```

**Description**: Converts exchange-specific symbol format to standard format  
**Parameters**: Exchange-specific symbol string  
**Returns**: Normalized Symbol object  
**Usage**: Used internally by exchange implementation

### Price Normalization

All exchanges must normalize prices to a standard precision:

```rust
pub fn normalize_price(price: f64, symbol: &Symbol) -> Price
```

**Description**: Converts exchange-specific price format to standard format  
**Parameters**: Exchange-specific price, Symbol for precision context  
**Returns**: Normalized Price object  
**Usage**: Used internally by exchange implementation

### Size Normalization

All exchanges must normalize sizes to a standard precision:

```rust
pub fn normalize_size(size: f64, symbol: &Symbol) -> Size
```

**Description**: Converts exchange-specific size format to standard format  
**Parameters**: Exchange-specific size, Symbol for precision context  
**Returns**: Normalized Size object  
**Usage**: Used internally by exchange implementation

## Authentication

### API Key Authentication

For CEX, implement API key authentication:

```rust
pub fn sign_request(&self, method: &str, path: &str, body: &str, timestamp: u64) -> String
```

**Description**: Generates signature for API request  
**Parameters**: HTTP method, API path, request body, timestamp  
**Returns**: Signature string  
**Usage**: Used internally for authenticated requests

### Wallet Authentication

For DEX, implement wallet signature authentication:

```rust
pub fn sign_message(&self, message: &str) -> String
```

**Description**: Generates wallet signature for message  
**Parameters**: Message to sign  
**Returns**: Signature string  
**Usage**: Used internally for authenticated requests

## WebSocket Message Handling

### Message Parsing

```rust
pub fn parse_message(&self, message: &str) -> Result<MarketEvent, ExchangeError>
```

**Description**: Parses WebSocket message into MarketEvent  
**Parameters**: Raw WebSocket message string  
**Returns**: Parsed MarketEvent or error  
**Usage**: Used internally for message processing

### Message Serialization

```rust
pub fn serialize_message(&self, event: &TradingEvent) -> Result<String, ExchangeError>
```

**Description**: Serializes TradingEvent to WebSocket message  
**Parameters**: TradingEvent to serialize  
**Returns**: Serialized message string or error  
**Usage**: Used internally for message sending

## Implementation Requirements

### Performance Requirements

1. Market data processing latency < 1ms
2. Order placement latency < 10ms
3. WebSocket message parsing < 100μs
4. Connection recovery time < 30s

### Reliability Requirements

1. Automatic reconnection on connection loss
2. Message queueing during disconnection
3. Error recovery mechanisms
4. Rate limit compliance

### Testing Requirements

1. Unit tests for all API methods
2. Integration tests with exchange testnets
3. Mock implementation for testing
4. Performance benchmarks for critical paths

## Exchange-Specific Considerations

### Binance

1. Use combined streams for multiple symbols
2. Implement listen key for user data streams
3. Handle exchange-specific error codes
4. Respect rate limits per endpoint

### OKX

1. Use public and private channels appropriately
2. Implement login for private channels
3. Handle exchange-specific message formats
4. Respect rate limits per instrument family

### Hyperliquid

1. Use subscription-based model
2. Implement wallet authentication
3. Handle exchange-specific data structures
4. Respect rate limits per endpoint

## Versioning

### API Versioning

Exchange implementations must support versioning:

```rust
pub struct ExchangeApiVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}
```

**Description**: Version information for exchange API  
**Usage**: Used for compatibility checking

### Backward Compatibility

1. Maintain backward compatibility for at least one major version
2. Provide migration path for breaking changes
3. Document all breaking changes
4. Support multiple API versions when possible
