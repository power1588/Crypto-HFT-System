use crate::traits::{
    ExecutionReport, OrderId, OrderManager, OrderSide, OrderStatus, OrderType, TimeInForce,
};
use crate::types::{Price, Size, Symbol};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::prelude::ToPrimitive;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Order information tracked by the order manager
#[derive(Debug, Clone)]
pub struct OrderInfo {
    /// Order ID
    pub order_id: OrderId,
    /// Client order ID
    pub client_order_id: Option<String>,
    /// Symbol
    pub symbol: Symbol,
    /// Order side
    pub side: OrderSide,
    /// Order type
    pub order_type: OrderType,
    /// Time in force
    pub time_in_force: TimeInForce,
    /// Quantity
    pub quantity: Size,
    /// Price (for limit orders)
    pub price: Option<Price>,
    /// Order status
    pub status: OrderStatus,
    /// Filled quantity
    pub filled_quantity: Size,
    /// Remaining quantity
    pub remaining_quantity: Size,
    /// Average fill price
    pub average_fill_price: Option<Price>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Exchange ID
    pub exchange_id: String,
}

impl OrderInfo {
    /// Create a new order info
    pub fn new(
        order_id: OrderId,
        client_order_id: Option<String>,
        symbol: Symbol,
        side: OrderSide,
        order_type: OrderType,
        time_in_force: TimeInForce,
        quantity: Size,
        price: Option<Price>,
        exchange_id: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            order_id,
            client_order_id,
            symbol,
            side,
            order_type,
            time_in_force,
            quantity,
            price,
            status: OrderStatus::New,
            filled_quantity: Size::new(rust_decimal::Decimal::ZERO),
            remaining_quantity: quantity,
            average_fill_price: None,
            created_at: now,
            updated_at: now,
            exchange_id,
        }
    }

    /// Update order info with an execution report
    pub fn update(&mut self, report: &ExecutionReport) {
        self.status = report.status;
        self.updated_at = Utc::now();

        match report.status {
            OrderStatus::New => {
                // No change to quantities
            }
            OrderStatus::PartiallyFilled => {
                self.filled_quantity = report.filled_size;
                self.remaining_quantity = report.remaining_size;

                // Update average fill price if provided
                if let Some(avg_price) = report.average_price {
                    self.average_fill_price = Some(avg_price);
                }
            }
            OrderStatus::Filled => {
                self.filled_quantity = report.filled_size;
                self.remaining_quantity = Size::new(rust_decimal::Decimal::ZERO);

                // Update average fill price if provided
                if let Some(avg_price) = report.average_price {
                    self.average_fill_price = Some(avg_price);
                }
            }
            OrderStatus::Cancelled => {
                self.remaining_quantity = report.remaining_size;
            }
            OrderStatus::Rejected | OrderStatus::Expired => {
                // No change to quantities
            }
        }
    }

    /// Check if the order is active (new or partially filled)
    pub fn is_active(&self) -> bool {
        matches!(self.status, OrderStatus::New | OrderStatus::PartiallyFilled)
    }

    /// Check if the order is filled
    pub fn is_filled(&self) -> bool {
        matches!(self.status, OrderStatus::Filled)
    }

    /// Check if the order is canceled
    pub fn is_canceled(&self) -> bool {
        matches!(self.status, OrderStatus::Cancelled)
    }

    /// Check if the order is rejected
    pub fn is_rejected(&self) -> bool {
        matches!(self.status, OrderStatus::Rejected)
    }

    /// Get the fill percentage
    pub fn fill_percentage(&self) -> f64 {
        if self.quantity.is_zero() {
            return 0.0;
        }

        let filled_ratio = self.filled_quantity.value() / self.quantity.value();
        filled_ratio.to_f64().unwrap_or(0.0) * 100.0
    }
}

/// Order manager implementation
#[allow(dead_code)]
pub struct OrderManagerImpl {
    /// All tracked orders by order ID
    orders: Arc<RwLock<HashMap<OrderId, OrderInfo>>>,
    /// Orders by symbol
    orders_by_symbol: Arc<RwLock<HashMap<String, Vec<OrderId>>>>,
    /// Active orders by symbol
    active_orders_by_symbol: Arc<RwLock<HashMap<String, Vec<OrderId>>>>,
    /// Orders by client order ID
    orders_by_client_id: Arc<RwLock<HashMap<String, OrderId>>>,
    /// Exchange ID
    exchange_id: String,
}

impl OrderManagerImpl {
    /// Create a new order manager
    pub fn new(exchange_id: String) -> Self {
        Self {
            orders: Arc::new(RwLock::new(HashMap::new())),
            orders_by_symbol: Arc::new(RwLock::new(HashMap::new())),
            active_orders_by_symbol: Arc::new(RwLock::new(HashMap::new())),
            orders_by_client_id: Arc::new(RwLock::new(HashMap::new())),
            exchange_id,
        }
    }

    /// Add a new order to track
    pub async fn add_order(&self, order_info: OrderInfo) {
        let order_id = order_info.order_id.clone();
        let symbol = order_info.symbol.value().to_string();

        // Add to main orders map
        let mut orders = self.orders.write().await;
        orders.insert(order_id.clone(), order_info.clone());

        // Add to symbol map
        let mut orders_by_symbol = self.orders_by_symbol.write().await;
        let symbol_orders = orders_by_symbol
            .entry(symbol.clone())
            .or_insert_with(Vec::new);
        symbol_orders.push(order_id.clone());

        // Add to active orders if active
        if order_info.is_active() {
            let mut active_orders = self.active_orders_by_symbol.write().await;
            let active_symbol_orders = active_orders.entry(symbol.clone()).or_insert_with(Vec::new);
            active_symbol_orders.push(order_id.clone());
        }

        // Add to client ID map if present
        if let Some(ref client_id) = order_info.client_order_id {
            let mut orders_by_client_id = self.orders_by_client_id.write().await;
            orders_by_client_id.insert(client_id.clone(), order_id.clone());
        }
    }

    /// Get order by ID
    pub async fn get_order(&self, order_id: &OrderId) -> Option<OrderInfo> {
        let orders = self.orders.read().await;
        orders.get(order_id).cloned()
    }

    /// Get order by client order ID
    pub async fn get_order_by_client_id(&self, client_order_id: &str) -> Option<OrderInfo> {
        let orders_by_client_id = self.orders_by_client_id.read().await;
        if let Some(order_id) = orders_by_client_id.get(client_order_id) {
            let orders = self.orders.read().await;
            orders.get(order_id).cloned()
        } else {
            None
        }
    }

    /// Get all orders for a symbol
    pub async fn get_orders_by_symbol(&self, symbol: &str) -> Vec<OrderInfo> {
        let orders_by_symbol = self.orders_by_symbol.read().await;
        if let Some(order_ids) = orders_by_symbol.get(symbol) {
            let orders = self.orders.read().await;
            order_ids
                .iter()
                .filter_map(|order_id| orders.get(order_id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get active orders for a symbol
    pub async fn get_active_orders_by_symbol(&self, symbol: &str) -> Vec<OrderInfo> {
        let active_orders_by_symbol = self.active_orders_by_symbol.read().await;
        if let Some(order_ids) = active_orders_by_symbol.get(symbol) {
            let orders = self.orders.read().await;
            order_ids
                .iter()
                .filter_map(|order_id| orders.get(order_id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get all active orders
    pub async fn get_all_active_orders(&self) -> Vec<OrderInfo> {
        let orders = self.orders.read().await;
        orders
            .values()
            .filter(|order| order.is_active())
            .cloned()
            .collect()
    }

    /// Cancel all orders for a symbol
    pub async fn cancel_all_orders_for_symbol(&self, symbol: &str) -> Vec<OrderId> {
        let active_orders = self.get_active_orders_by_symbol(symbol).await;
        let order_ids: Vec<OrderId> = active_orders
            .iter()
            .map(|order| order.order_id.clone())
            .collect();

        // Update status of all active orders to canceled
        let mut orders = self.orders.write().await;
        for order_id in &order_ids {
            if let Some(order) = orders.get_mut(order_id) {
                order.status = OrderStatus::Cancelled;
                order.updated_at = Utc::now();
            }
        }

        // Remove from active orders
        let mut active_orders_by_symbol = self.active_orders_by_symbol.write().await;
        active_orders_by_symbol.remove(symbol);

        order_ids
    }

    /// Get total position for a symbol
    pub async fn get_position_for_symbol(&self, symbol: &str) -> Size {
        let orders = self.get_orders_by_symbol(symbol).await;

        let mut total_bought = Size::new(rust_decimal::Decimal::ZERO);
        let mut total_sold = Size::new(rust_decimal::Decimal::ZERO);

        for order in orders {
            if order.is_filled() {
                match order.side {
                    OrderSide::Buy => total_bought = total_bought + order.filled_quantity,
                    OrderSide::Sell => total_sold = total_sold + order.filled_quantity,
                }
            }
        }

        total_bought - total_sold
    }

    /// Get total exposure for a symbol
    pub async fn get_exposure_for_symbol(&self, symbol: &str) -> Option<rust_decimal::Decimal> {
        let orders = self.get_orders_by_symbol(symbol).await;

        let mut total_exposure = rust_decimal::Decimal::ZERO;

        for order in orders {
            if order.is_filled() {
                let fill_price = order.average_fill_price?;
                let fill_value = order.filled_quantity.value() * fill_price.value();

                match order.side {
                    OrderSide::Buy => total_exposure += fill_value,
                    OrderSide::Sell => total_exposure -= fill_value,
                }
            }
        }

        Some(total_exposure)
    }

    /// Get order statistics for a symbol
    pub async fn get_order_stats_for_symbol(&self, symbol: &str) -> OrderStats {
        let orders = self.get_orders_by_symbol(symbol).await;

        let mut total_orders = 0;
        let mut filled_orders = 0;
        let mut canceled_orders = 0;
        let mut rejected_orders = 0;
        let mut total_volume = Size::new(rust_decimal::Decimal::ZERO);
        let mut total_value = rust_decimal::Decimal::ZERO;

        for order in orders {
            total_orders += 1;

            if order.is_filled() {
                filled_orders += 1;
                total_volume = total_volume + order.filled_quantity;

                if let Some(fill_price) = order.average_fill_price {
                    total_value += order.filled_quantity.value() * fill_price.value();
                }
            } else if order.is_canceled() {
                canceled_orders += 1;
            } else if order.is_rejected() {
                rejected_orders += 1;
            }
        }

        let fill_rate = if total_orders > 0 {
            filled_orders as f64 / total_orders as f64 * 100.0
        } else {
            0.0
        };

        let cancel_rate = if total_orders > 0 {
            canceled_orders as f64 / total_orders as f64 * 100.0
        } else {
            0.0
        };

        let reject_rate = if total_orders > 0 {
            rejected_orders as f64 / total_orders as f64 * 100.0
        } else {
            0.0
        };

        OrderStats {
            total_orders,
            filled_orders,
            canceled_orders,
            rejected_orders,
            fill_rate,
            cancel_rate,
            reject_rate,
            total_volume,
            total_value,
        }
    }
}

/// Order statistics
#[derive(Debug, Clone)]
pub struct OrderStats {
    /// Total number of orders
    pub total_orders: u32,
    /// Number of filled orders
    pub filled_orders: u32,
    /// Number of canceled orders
    pub canceled_orders: u32,
    /// Number of rejected orders
    pub rejected_orders: u32,
    /// Fill rate (percentage)
    pub fill_rate: f64,
    /// Cancel rate (percentage)
    pub cancel_rate: f64,
    /// Reject rate (percentage)
    pub reject_rate: f64,
    /// Total volume traded
    pub total_volume: Size,
    /// Total value traded
    pub total_value: rust_decimal::Decimal,
}

#[async_trait]
impl OrderManager for OrderManagerImpl {
    type Error = OrderManagerError;

    async fn handle_execution_report(
        &mut self,
        report: ExecutionReport,
    ) -> Result<(), Self::Error> {
        let order_id = report.order_id.clone();

        // Check if we're tracking this order
        let mut orders = self.orders.write().await;
        if let Some(order) = orders.get_mut(&order_id) {
            // Update existing order
            let old_status = order.status.clone();
            order.update(&report);

            // Update symbol mappings if status changed
            if old_status != order.status {
                let symbol = order.symbol.value().to_string();

                // Update active orders
                let mut active_orders = self.active_orders_by_symbol.write().await;
                let symbol_active_orders =
                    active_orders.entry(symbol.clone()).or_insert_with(Vec::new);

                if order.is_active()
                    && !matches!(old_status, OrderStatus::New | OrderStatus::PartiallyFilled)
                {
                    // Order became active
                    symbol_active_orders.push(order_id.clone());
                } else if matches!(old_status, OrderStatus::New | OrderStatus::PartiallyFilled)
                    && !order.is_active()
                {
                    // Order is no longer active
                    symbol_active_orders.retain(|id| id != &order_id);

                    // Remove empty symbol entry
                    if symbol_active_orders.is_empty() {
                        active_orders.remove(&symbol);
                    }
                }
            }

            Ok(())
        } else {
            // This is an order we're not tracking - skip it
            // We don't have enough info in ExecutionReport to create a full OrderInfo
            // (missing side, order_type, time_in_force, quantity, price)
            drop(orders);
            Ok(())
        }
    }

    async fn get_all_orders(&self) -> Result<Vec<ExecutionReport>, Self::Error> {
        let orders = self.orders.read().await;
        let execution_reports: Vec<ExecutionReport> = orders
            .values()
            .map(|order| ExecutionReport {
                order_id: order.order_id.clone(),
                client_order_id: order.client_order_id.clone(),
                symbol: order.symbol.clone(),
                exchange_id: order.exchange_id.clone(),
                status: order.status,
                filled_size: order.filled_quantity,
                remaining_size: order.remaining_quantity,
                average_price: order.average_fill_price,
                timestamp: order.updated_at.timestamp_millis() as u64,
            })
            .collect();

        Ok(execution_reports)
    }

    async fn get_orders_by_symbol(
        &self,
        symbol: &str,
    ) -> Result<Vec<ExecutionReport>, Self::Error> {
        let orders = OrderManagerImpl::get_orders_by_symbol(self, symbol).await;
        let execution_reports: Vec<ExecutionReport> = orders
            .iter()
            .map(|order| ExecutionReport {
                order_id: order.order_id.clone(),
                client_order_id: order.client_order_id.clone(),
                symbol: order.symbol.clone(),
                exchange_id: order.exchange_id.clone(),
                status: order.status,
                filled_size: order.filled_quantity,
                remaining_size: order.remaining_quantity,
                average_price: order.average_fill_price,
                timestamp: order.updated_at.timestamp_millis() as u64,
            })
            .collect();

        Ok(execution_reports)
    }

    async fn get_open_orders(&self) -> Result<Vec<ExecutionReport>, Self::Error> {
        let active_orders = self.get_all_active_orders().await;
        let execution_reports: Vec<ExecutionReport> = active_orders
            .iter()
            .map(|order| ExecutionReport {
                order_id: order.order_id.clone(),
                client_order_id: order.client_order_id.clone(),
                symbol: order.symbol.clone(),
                exchange_id: order.exchange_id.clone(),
                status: order.status,
                filled_size: order.filled_quantity,
                remaining_size: order.remaining_quantity,
                average_price: order.average_fill_price,
                timestamp: order.updated_at.timestamp_millis() as u64,
            })
            .collect();

        Ok(execution_reports)
    }
}

/// Order manager error types
#[derive(Debug, Clone)]
pub enum OrderManagerError {
    OrderNotFound(OrderId),
    InvalidOrder(String),
    InternalError(String),
}

impl std::fmt::Display for OrderManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderManagerError::OrderNotFound(id) => write!(f, "Order not found: {}", id),
            OrderManagerError::InvalidOrder(msg) => write!(f, "Invalid order: {}", msg),
            OrderManagerError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for OrderManagerError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_info_creation() {
        let order_id = "12345".to_string();
        let order_info = OrderInfo::new(
            order_id.clone(),
            Some("client_123".to_string()),
            Symbol::new("BTCUSDT"),
            OrderSide::Buy,
            OrderType::Limit,
            TimeInForce::GoodTillCancelled,
            Size::from_str("1.0").unwrap(),
            Some(Price::from_str("50000.0").unwrap()),
            "binance".to_string(),
        );

        assert_eq!(order_info.order_id, order_id);
        assert_eq!(order_info.client_order_id, Some("client_123".to_string()));
        assert_eq!(order_info.symbol.value(), "BTCUSDT");
        assert_eq!(order_info.side, OrderSide::Buy);
        assert_eq!(order_info.order_type, OrderType::Limit);
        assert_eq!(order_info.time_in_force, TimeInForce::GoodTillCancelled);
        assert_eq!(order_info.quantity, Size::from_str("1.0").unwrap());
        assert_eq!(order_info.price, Some(Price::from_str("50000.0").unwrap()));
        assert_eq!(order_info.status, OrderStatus::New);
        assert_eq!(
            order_info.filled_quantity,
            Size::new(rust_decimal::Decimal::ZERO)
        );
        assert_eq!(
            order_info.remaining_quantity,
            Size::from_str("1.0").unwrap()
        );
        assert!(order_info.is_active());
        assert!(!order_info.is_filled());
        assert!(!order_info.is_canceled());
        assert!(!order_info.is_rejected());
    }

    #[test]
    fn test_order_info_update() {
        let order_id = "12345".to_string();
        let mut order_info = OrderInfo::new(
            order_id.clone(),
            Some("client_123".to_string()),
            Symbol::new("BTCUSDT"),
            OrderSide::Buy,
            OrderType::Limit,
            TimeInForce::GoodTillCancelled,
            Size::from_str("1.0").unwrap(),
            Some(Price::from_str("50000.0").unwrap()),
            "binance".to_string(),
        );

        // Update with partial fill
        let report = ExecutionReport {
            order_id: order_id.clone(),
            client_order_id: Some("client_123".to_string()),
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "binance".to_string(),
            status: OrderStatus::PartiallyFilled,
            filled_size: Size::from_str("0.5").unwrap(),
            remaining_size: Size::from_str("0.5").unwrap(),
            average_price: Some(Price::from_str("50000.0").unwrap()),
            timestamp: 1638368000000,
        };

        order_info.update(&report);

        assert_eq!(order_info.status, OrderStatus::PartiallyFilled);
        assert_eq!(order_info.filled_quantity, Size::from_str("0.5").unwrap());
        assert_eq!(
            order_info.remaining_quantity,
            Size::from_str("0.5").unwrap()
        );
        assert_eq!(order_info.fill_percentage(), 50.0);

        // Update with full fill
        let report = ExecutionReport {
            order_id: order_id.clone(),
            client_order_id: Some("client_123".to_string()),
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "binance".to_string(),
            status: OrderStatus::Filled,
            filled_size: Size::from_str("1.0").unwrap(),
            remaining_size: Size::from_str("0.0").unwrap(),
            average_price: Some(Price::from_str("50000.0").unwrap()),
            timestamp: 1638368000000,
        };

        order_info.update(&report);

        assert_eq!(order_info.status, OrderStatus::Filled);
        assert_eq!(order_info.filled_quantity, Size::from_str("1.0").unwrap());
        assert_eq!(
            order_info.remaining_quantity,
            Size::new(rust_decimal::Decimal::ZERO)
        );
        assert_eq!(order_info.fill_percentage(), 100.0);
    }

    #[tokio::test]
    async fn test_order_manager_creation() {
        let order_manager = OrderManagerImpl::new("binance".to_string());

        // Verify order manager was created
        assert_eq!(order_manager.exchange_id, "binance");

        // Initially no orders
        let all_orders = order_manager.get_all_orders().await.unwrap();
        assert!(all_orders.is_empty());

        let open_orders = order_manager.get_open_orders().await.unwrap();
        assert!(open_orders.is_empty());
    }

    #[tokio::test]
    async fn test_order_manager_add_order() {
        let order_manager = OrderManagerImpl::new("binance".to_string());

        let order_id = "12345".to_string();
        let order_info = OrderInfo::new(
            order_id.clone(),
            Some("client_123".to_string()),
            Symbol::new("BTCUSDT"),
            OrderSide::Buy,
            OrderType::Limit,
            TimeInForce::GoodTillCancelled,
            Size::from_str("1.0").unwrap(),
            Some(Price::from_str("50000.0").unwrap()),
            "binance".to_string(),
        );

        order_manager.add_order(order_info).await;

        // Verify order was added
        let retrieved_order = order_manager.get_order(&order_id).await;
        assert!(retrieved_order.is_some());
        assert_eq!(retrieved_order.unwrap().order_id, order_id);

        // Verify order can be retrieved by client ID
        let client_order = order_manager.get_order_by_client_id("client_123").await;
        assert!(client_order.is_some());
        assert_eq!(client_order.unwrap().order_id, order_id);

        // Verify order is in symbol orders
        let symbol_orders = OrderManagerImpl::get_orders_by_symbol(&order_manager, "BTCUSDT").await;
        assert_eq!(symbol_orders.len(), 1);
        assert_eq!(symbol_orders[0].order_id, order_id);

        // Verify order is in active orders
        let active_orders = order_manager.get_active_orders_by_symbol("BTCUSDT").await;
        assert_eq!(active_orders.len(), 1);
        assert_eq!(active_orders[0].order_id, order_id);
    }

    #[tokio::test]
    async fn test_order_manager_cancel_all() {
        let order_manager = OrderManagerImpl::new("binance".to_string());

        // Add multiple orders
        for i in 1..=3 {
            let order_id = format!("order_{}", i);
            let order_info = OrderInfo::new(
                order_id.clone(),
                Some(format!("client_{}", i)),
                Symbol::new("BTCUSDT"),
                OrderSide::Buy,
                OrderType::Limit,
                TimeInForce::GoodTillCancelled,
                Size::from_str("1.0").unwrap(),
                Some(Price::from_str("50000.0").unwrap()),
                "binance".to_string(),
            );

            order_manager.add_order(order_info).await;
        }

        // Verify all orders are active
        let active_orders = order_manager.get_active_orders_by_symbol("BTCUSDT").await;
        assert_eq!(active_orders.len(), 3);

        // Cancel all orders
        let canceled_orders = order_manager.cancel_all_orders_for_symbol("BTCUSDT").await;
        assert_eq!(canceled_orders.len(), 3);

        // Verify no active orders
        let active_orders = order_manager.get_active_orders_by_symbol("BTCUSDT").await;
        assert!(active_orders.is_empty());

        // Verify orders are marked as canceled
        let symbol_orders = OrderManagerImpl::get_orders_by_symbol(&order_manager, "BTCUSDT").await;
        assert_eq!(symbol_orders.len(), 3);
        for order in symbol_orders {
            assert!(order.is_canceled());
        }
    }

    #[tokio::test]
    async fn test_order_manager_position() {
        let mut order_manager = OrderManagerImpl::new("binance".to_string());

        // Add a buy order
        let buy_order_id = "buy_order".to_string();
        let buy_order_info = OrderInfo::new(
            buy_order_id.clone(),
            Some("buy_client".to_string()),
            Symbol::new("BTCUSDT"),
            OrderSide::Buy,
            OrderType::Limit,
            TimeInForce::GoodTillCancelled,
            Size::from_str("1.0").unwrap(),
            Some(Price::from_str("50000.0").unwrap()),
            "binance".to_string(),
        );

        // Add a sell order
        let sell_order_id = "sell_order".to_string();
        let sell_order_info = OrderInfo::new(
            sell_order_id.clone(),
            Some("sell_client".to_string()),
            Symbol::new("BTCUSDT"),
            OrderSide::Sell,
            OrderType::Limit,
            TimeInForce::GoodTillCancelled,
            Size::from_str("0.5").unwrap(),
            Some(Price::from_str("51000.0").unwrap()),
            "binance".to_string(),
        );

        order_manager.add_order(buy_order_info).await;
        order_manager.add_order(sell_order_info).await;

        // Initially, position should be zero (no filled orders)
        let position = order_manager.get_position_for_symbol("BTCUSDT").await;
        assert_eq!(position, Size::new(rust_decimal::Decimal::ZERO));

        // Simulate fills by updating orders
        let buy_report = ExecutionReport {
            order_id: buy_order_id,
            client_order_id: Some("buy_client".to_string()),
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "binance".to_string(),
            status: OrderStatus::Filled,
            filled_size: Size::from_str("1.0").unwrap(),
            remaining_size: Size::from_str("0.0").unwrap(),
            average_price: Some(Price::from_str("50000.0").unwrap()),
            timestamp: 1638368000000,
        };

        let sell_report = ExecutionReport {
            order_id: sell_order_id,
            client_order_id: Some("sell_client".to_string()),
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "binance".to_string(),
            status: OrderStatus::Filled,
            filled_size: Size::from_str("0.5").unwrap(),
            remaining_size: Size::from_str("0.0").unwrap(),
            average_price: Some(Price::from_str("51000.0").unwrap()),
            timestamp: 1638368000000,
        };

        order_manager
            .handle_execution_report(buy_report)
            .await
            .unwrap();
        order_manager
            .handle_execution_report(sell_report)
            .await
            .unwrap();

        // Position should be 0.5 BTC (1.0 bought - 0.5 sold)
        let position = order_manager.get_position_for_symbol("BTCUSDT").await;
        assert_eq!(position, Size::from_str("0.5").unwrap());
    }

    #[tokio::test]
    async fn test_order_manager_stats() {
        let mut order_manager = OrderManagerImpl::new("binance".to_string());

        // Add multiple orders with different statuses
        for i in 1..=5 {
            let order_id = format!("order_{}", i);
            let order_info = OrderInfo::new(
                order_id.clone(),
                Some(format!("client_{}", i)),
                Symbol::new("BTCUSDT"),
                if i % 2 == 0 {
                    OrderSide::Buy
                } else {
                    OrderSide::Sell
                },
                OrderType::Limit,
                TimeInForce::GoodTillCancelled,
                Size::from_str("1.0").unwrap(),
                Some(Price::from_str("50000.0").unwrap()),
                "binance".to_string(),
            );

            order_manager.add_order(order_info).await;
        }

        // Simulate different outcomes
        for i in 1..=5 {
            let order_id = format!("order_{}", i);
            let (status, filled_size, remaining_size) = match i {
                1 => (
                    OrderStatus::Filled,
                    Size::from_str("1.0").unwrap(),
                    Size::from_str("0.0").unwrap(),
                ),
                2 => (
                    OrderStatus::Filled,
                    Size::from_str("1.0").unwrap(),
                    Size::from_str("0.0").unwrap(),
                ),
                3 => (
                    OrderStatus::Cancelled,
                    Size::from_str("0.0").unwrap(),
                    Size::from_str("1.0").unwrap(),
                ),
                4 => (
                    OrderStatus::Rejected,
                    Size::from_str("0.0").unwrap(),
                    Size::from_str("1.0").unwrap(),
                ),
                5 => (
                    OrderStatus::New,
                    Size::from_str("0.0").unwrap(),
                    Size::from_str("1.0").unwrap(),
                ),
                _ => unreachable!(),
            };

            let report = ExecutionReport {
                order_id,
                client_order_id: Some(format!("client_{}", i)),
                symbol: Symbol::new("BTCUSDT"),
                exchange_id: "binance".to_string(),
                status,
                filled_size,
                remaining_size,
                average_price: Some(Price::from_str("50000.0").unwrap()),
                timestamp: 1638368000000,
            };

            order_manager.handle_execution_report(report).await.unwrap();
        }

        // Check statistics
        let stats = order_manager.get_order_stats_for_symbol("BTCUSDT").await;
        assert_eq!(stats.total_orders, 5);
        assert_eq!(stats.filled_orders, 2);
        assert_eq!(stats.canceled_orders, 1);
        assert_eq!(stats.rejected_orders, 1);
        assert_eq!(stats.fill_rate, 40.0); // 2/5 * 100
        assert_eq!(stats.cancel_rate, 20.0); // 1/5 * 100
        assert_eq!(stats.reject_rate, 20.0); // 1/5 * 100
        assert_eq!(stats.total_volume, Size::from_str("2.0").unwrap()); // 2 filled orders * 1.0 each
    }

    #[test]
    fn test_order_manager_error_display() {
        let error = OrderManagerError::OrderNotFound("12345".to_string());
        assert_eq!(error.to_string(), "Order not found: 12345");

        let error = OrderManagerError::InvalidOrder("Invalid price".to_string());
        assert_eq!(error.to_string(), "Invalid order: Invalid price");
    }
}
