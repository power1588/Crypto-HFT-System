use async_trait::async_trait;
use crate::traits::{
    MarketDataStream, ExecutionClient, MarketEvent, NewOrder, OrderId, ExecutionReport,
    OrderStatus, OrderSide, OrderType, TimeInForce, Balance, TradingFees
};
use crate::types::Size;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Mock implementation of MarketDataStream for testing
#[derive(Debug)]
pub struct MockMarketDataStream {
    events: Arc<RwLock<Vec<MarketEvent>>>,
    index: Arc<Mutex<usize>>,
    subscriptions: Arc<RwLock<Vec<String>>>,
    connected: Arc<RwLock<bool>>,
    last_updates: Arc<RwLock<HashMap<String, u64>>>,
}

impl MockMarketDataStream {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            index: Arc::new(Mutex::new(0)),
            subscriptions: Arc::new(RwLock::new(Vec::new())),
            connected: Arc::new(RwLock::new(false)),
            last_updates: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_event(&self, event: MarketEvent) {
        let mut events = self.events.write().await;
        events.push(event);
    }

    pub async fn set_connected(&self, connected: bool) {
        let mut conn = self.connected.write().await;
        *conn = connected;
    }

    pub async fn set_last_update(&self, symbol: &str, timestamp: u64) {
        let mut updates = self.last_updates.write().await;
        updates.insert(symbol.to_string(), timestamp);
    }
}

#[async_trait]
impl MarketDataStream for MockMarketDataStream {
    type Error = MockError;

    async fn subscribe(&mut self, symbols: &[&str]) -> Result<(), Self::Error> {
        let mut subs = self.subscriptions.write().await;
        for symbol in symbols {
            if !subs.contains(&symbol.to_string()) {
                subs.push(symbol.to_string());
            }
        }
        Ok(())
    }

    async fn unsubscribe(&mut self, symbols: &[&str]) -> Result<(), Self::Error> {
        let mut subs = self.subscriptions.write().await;
        subs.retain(|s| !symbols.contains(&s.as_str()));
        Ok(())
    }

    async fn next(&mut self) -> Option<Result<MarketEvent, Self::Error>> {
        let events = self.events.read().await;
        let mut index = self.index.lock().await;
        
        if *index < events.len() {
            let event = events[*index].clone();
            *index += 1;
            Some(Ok(event))
        } else {
            None
        }
    }

    fn is_connected(&self) -> bool {
        // This is a synchronous method, so we can't use async here
        // In a real implementation, you might use a different approach
        true // For simplicity, always return true
    }

    fn last_update(&self, _symbol: &str) -> Option<u64> {
        // This is a synchronous method, so we can't use async here
        // In a real implementation, you might use a different approach
        None // For simplicity, always return None
    }
}

/// Mock implementation of ExecutionClient for testing
#[derive(Debug)]
pub struct MockExecutionClient {
    orders: Arc<RwLock<HashMap<OrderId, ExecutionReport>>>,
    balances: Arc<RwLock<HashMap<String, Balance>>>,
    fees: Arc<RwLock<HashMap<String, TradingFees>>>,
    order_counter: Arc<Mutex<u64>>,
}

impl MockExecutionClient {
    pub fn new() -> Self {
        let mut balances = HashMap::new();
        balances.insert("BTC".to_string(), Balance::new("BTC".to_string(), 
            Size::from_str("10.0").unwrap(), Size::from_str("0.0").unwrap()));
        balances.insert("USDT".to_string(), Balance::new("USDT".to_string(), 
            Size::from_str("100000.0").unwrap(), Size::from_str("0.0").unwrap()));
        
        let mut fees = HashMap::new();
        fees.insert("BTCUSDT".to_string(), TradingFees::new("BTCUSDT".to_string(),
            Size::from_str("0.001").unwrap(), Size::from_str("0.001").unwrap()));
        
        Self {
            orders: Arc::new(RwLock::new(HashMap::new())),
            balances: Arc::new(RwLock::new(balances)),
            fees: Arc::new(RwLock::new(fees)),
            order_counter: Arc::new(Mutex::new(1)),
        }
    }

    pub async fn set_balance(&self, asset: &str, free: Size, locked: Size) {
        let mut balances = self.balances.write().await;
        balances.insert(asset.to_string(), Balance::new(asset.to_string(), free, locked));
    }

    pub async fn set_fee(&self, symbol: &str, maker_fee: Size, taker_fee: Size) {
        let mut fees = self.fees.write().await;
        fees.insert(symbol.to_string(), TradingFees::new(symbol.to_string(), maker_fee, taker_fee));
    }
}

#[async_trait]
impl ExecutionClient for MockExecutionClient {
    type Error = MockError;

    async fn place_order(&self, order: NewOrder) -> Result<OrderId, Self::Error> {
        let mut counter = self.order_counter.lock().await;
        let order_id = OrderId::new(format!("order_{}", *counter));
        *counter += 1;
        
        let report = ExecutionReport {
            order_id: order_id.clone(),
            client_order_id: order.client_order_id.clone(),
            symbol: order.symbol.clone(),
            status: OrderStatus::New,
            side: order.side,
            order_type: order.order_type,
            time_in_force: order.time_in_force,
            quantity: order.quantity,
            price: order.price,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };
        
        let mut orders = self.orders.write().await;
        orders.insert(order_id.clone(), report);
        
        Ok(order_id)
    }

    async fn cancel_order(&self, order_id: OrderId) -> Result<(), Self::Error> {
        let mut orders = self.orders.write().await;
        if let Some(mut report) = orders.remove(&order_id) {
            report.status = OrderStatus::Canceled { 
                remaining_size: report.quantity 
            };
            orders.insert(order_id, report);
            Ok(())
        } else {
            Err(MockError::OrderNotFound(order_id))
        }
    }

    async fn get_order_status(&self, order_id: OrderId) -> Result<ExecutionReport, Self::Error> {
        let orders = self.orders.read().await;
        orders.get(&order_id)
            .cloned()
            .ok_or(MockError::OrderNotFound(order_id))
    }

    async fn get_balances(&self) -> Result<Vec<Balance>, Self::Error> {
        let balances = self.balances.read().await;
        Ok(balances.values().cloned().collect())
    }

    async fn get_open_orders(&self, symbol: Option<&str>) -> Result<Vec<ExecutionReport>, Self::Error> {
        let orders = self.orders.read().await;
        let mut open_orders = Vec::new();
        
        for report in orders.values() {
            if matches!(report.status, OrderStatus::New | OrderStatus::PartiallyFilled { .. }) {
                if let Some(s) = symbol {
                    if report.symbol == s {
                        open_orders.push(report.clone());
                    }
                } else {
                    open_orders.push(report.clone());
                }
            }
        }
        
        Ok(open_orders)
    }

    async fn get_order_history(
        &self,
        symbol: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<ExecutionReport>, Self::Error> {
        let orders = self.orders.read().await;
        let mut history = Vec::new();
        
        for report in orders.values() {
            if let Some(s) = symbol {
                if report.symbol == s {
                    history.push(report.clone());
                }
            } else {
                history.push(report.clone());
            }
        }
        
        // Sort by timestamp descending
        history.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        if let Some(limit) = limit {
            history.truncate(limit);
        }
        
        Ok(history)
    }

    async fn get_trading_fees(&self, symbol: &str) -> Result<TradingFees, Self::Error> {
        let fees = self.fees.read().await;
        fees.get(symbol)
            .cloned()
            .ok_or(MockError::SymbolNotFound(symbol.to_string()))
    }
}

/// Mock error type
#[derive(Debug, Clone)]
pub enum MockError {
    OrderNotFound(OrderId),
    SymbolNotFound(String),
    ConnectionError,
    ParseError(String),
}

impl std::fmt::Display for MockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MockError::OrderNotFound(id) => write!(f, "Order not found: {}", id.as_str()),
            MockError::SymbolNotFound(symbol) => write!(f, "Symbol not found: {}", symbol),
            MockError::ConnectionError => write!(f, "Connection error"),
            MockError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for MockError {}
