use crate::traits::{
    Balance, ExecutionClient, ExecutionReport, NewOrder, OrderId, OrderSide, OrderStatus,
    OrderType, TimeInForce, TradingFees,
};
use crate::types::Size;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

/// Dry-run execution client that prints orders but doesn't actually place them
#[derive(Debug)]
pub struct DryRunExecutionClient {
    orders: Arc<Mutex<HashMap<String, ExecutionReport>>>,
    order_counter: Arc<Mutex<u64>>,
}

impl DryRunExecutionClient {
    pub fn new() -> Self {
        Self {
            orders: Arc::new(Mutex::new(HashMap::new())),
            order_counter: Arc::new(Mutex::new(1)),
        }
    }
}

#[async_trait]
impl ExecutionClient for DryRunExecutionClient {
    type Error = DryRunError;

    async fn place_order(&self, order: NewOrder) -> Result<OrderId, Self::Error> {
        let mut counter = self.order_counter.lock().await;
        let order_id: OrderId = format!("dry_run_{}", *counter);
        *counter += 1;

        // Print order details
        println!("\n╔════════════════════════════════════════════════════════════╗");
        println!("║ 🟢 DRY-RUN ORDER PLACED (模拟下单)                         ║");
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ Order ID:     {:45} ║", order_id.as_str());
        println!("║ Symbol:       {:45} ║", order.symbol.as_str());
        println!(
            "║ Side:         {:45} ║",
            match order.side {
                OrderSide::Buy => "BUY (买入)",
                OrderSide::Sell => "SELL (卖出)",
            }
        );
        println!(
            "║ Type:         {:45} ║",
            match order.order_type {
                OrderType::Market => "MARKET",
                OrderType::Limit => "LIMIT",
                OrderType::StopLoss => "STOP_LOSS",
                OrderType::StopLimit => "STOP_LIMIT",
            }
        );
        if let Some(price) = order.price {
            println!("║ Price:        {:45} ║", format!("{}", price));
        }
        println!("║ Size:         {:45} ║", format!("{}", order.size));
        println!(
            "║ TimeInForce:  {:45} ║",
            match order.time_in_force {
                TimeInForce::GoodTillCancelled => "GTC (Good Till Cancel)",
                TimeInForce::ImmediateOrCancel => "IOC (Immediate Or Cancel)",
                TimeInForce::FillOrKill => "FOK (Fill Or Kill)",
            }
        );
        if let Some(ref client_id) = order.client_order_id {
            println!("║ Client ID:    {:45} ║", client_id);
        }
        println!("╚════════════════════════════════════════════════════════════╝\n");

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let report = ExecutionReport {
            order_id: order_id.as_str().to_string(),
            client_order_id: order.client_order_id.clone(),
            symbol: order.symbol.clone(),
            exchange_id: order.exchange_id.clone(),
            status: OrderStatus::New,
            filled_size: Size::zero(),
            remaining_size: order.size,
            average_price: None,
            timestamp,
        };

        let mut orders = self.orders.lock().await;
        orders.insert(order_id.as_str().to_string(), report);

        Ok(order_id)
    }

    async fn cancel_order(&self, order_id: OrderId) -> Result<(), Self::Error> {
        let mut orders = self.orders.lock().await;
        let order_id_str = order_id.as_str().to_string();

        if let Some(mut report) = orders.remove(&order_id_str) {
            report.status = OrderStatus::Cancelled;

            println!("\n╔════════════════════════════════════════════════════════════╗");
            println!("║ 🔴 DRY-RUN ORDER CANCELED (模拟撤单)                      ║");
            println!("╠════════════════════════════════════════════════════════════╣");
            println!("║ Order ID:     {:45} ║", order_id.as_str());
            println!("║ Symbol:       {:45} ║", report.symbol.as_str());
            println!("╚════════════════════════════════════════════════════════════╝\n");

            orders.insert(order_id_str, report);
            Ok(())
        } else {
            Err(DryRunError::OrderNotFound(order_id))
        }
    }

    async fn get_order_status(&self, order_id: OrderId) -> Result<ExecutionReport, Self::Error> {
        let orders = self.orders.lock().await;
        orders
            .get(order_id.as_str())
            .cloned()
            .ok_or(DryRunError::OrderNotFound(order_id))
    }

    async fn get_balances(&self) -> Result<Vec<Balance>, Self::Error> {
        // Return mock balances for dry-run
        Ok(vec![
            Balance::new(
                "BTC".to_string(),
                Size::from_str("10.0").unwrap(),
                Size::from_str("0.0").unwrap(),
            ),
            Balance::new(
                "USDT".to_string(),
                Size::from_str("100000.0").unwrap(),
                Size::from_str("0.0").unwrap(),
            ),
        ])
    }

    async fn get_open_orders(
        &self,
        symbol: Option<&str>,
    ) -> Result<Vec<ExecutionReport>, Self::Error> {
        let orders = self.orders.lock().await;
        let mut open_orders = Vec::new();

        for report in orders.values() {
            if matches!(
                report.status,
                OrderStatus::New | OrderStatus::PartiallyFilled
            ) {
                if let Some(s) = symbol {
                    if report.symbol.as_str() == s {
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
        let orders = self.orders.lock().await;
        let mut history = Vec::new();

        for report in orders.values() {
            if let Some(s) = symbol {
                if report.symbol.as_str() == s {
                    history.push(report.clone());
                }
            } else {
                history.push(report.clone());
            }
        }

        history.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        if let Some(limit) = limit {
            history.truncate(limit);
        }

        Ok(history)
    }

    async fn get_trading_fees(&self, symbol: &str) -> Result<TradingFees, Self::Error> {
        Ok(TradingFees::new(
            symbol.to_string(),
            Size::from_str("0.001").unwrap(), // 0.1% maker fee
            Size::from_str("0.001").unwrap(), // 0.1% taker fee
        ))
    }
}

/// Dry-run error type
#[derive(Debug, Clone)]
pub enum DryRunError {
    OrderNotFound(OrderId),
    SymbolNotFound(String),
}

impl std::fmt::Display for DryRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DryRunError::OrderNotFound(id) => write!(f, "Order not found: {}", id.as_str()),
            DryRunError::SymbolNotFound(symbol) => write!(f, "Symbol not found: {}", symbol),
        }
    }
}

impl std::error::Error for DryRunError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Price, Symbol};

    #[tokio::test]
    async fn test_dry_run_place_order() {
        let client = DryRunExecutionClient::new();

        let order = NewOrder {
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "dry_run".to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::GoodTillCancelled,
            price: Some(Price::from_str("50000.00").unwrap()),
            size: Size::from_str("1.0").unwrap(),
            client_order_id: Some("test_123".to_string()),
        };

        let result = client.place_order(order).await;
        assert!(result.is_ok());

        let order_id = result.unwrap();
        assert!(order_id.as_str().starts_with("dry_run_"));
    }

    #[tokio::test]
    async fn test_dry_run_cancel_order() {
        let client = DryRunExecutionClient::new();

        let order = NewOrder {
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "dry_run".to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::GoodTillCancelled,
            price: Some(Price::from_str("50000.00").unwrap()),
            size: Size::from_str("1.0").unwrap(),
            client_order_id: None,
        };

        let order_id = client.place_order(order).await.unwrap();
        let cancel_result = client.cancel_order(order_id.clone()).await;
        assert!(cancel_result.is_ok());

        let status = client.get_order_status(order_id).await.unwrap();
        assert_eq!(status.status, OrderStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_dry_run_get_balances() {
        let client = DryRunExecutionClient::new();
        let balances = client.get_balances().await.unwrap();

        assert!(!balances.is_empty());
        // Should have BTC and USDT balances
        let btc_balance = balances.iter().find(|b| b.asset == "BTC");
        assert!(btc_balance.is_some());
    }
}
