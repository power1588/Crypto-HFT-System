use async_trait::async_trait;
use crate::traits::{
    MarketDataStream, MarketDataHistory, ExecutionClient, OrderManager,
    MarketEvent, NewOrder, OrderId, ExecutionReport, OrderStatus, OrderSide, OrderType, TimeInForce,
    Balance, TradingFees, Trade
};
use crate::types::{Price, Size, Symbol};
use crate::core::events::{OrderBookSnapshot, OrderBookDelta, OrderBookLevel};
use crate::exchanges::connection_manager::ExchangeAdapter;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use reqwest::Client;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use base64::{Engine as _, engine::general_purpose};
use chrono::{DateTime, Utc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// OKX API client for market data and order execution
pub struct OkxClient {
    /// API key
    api_key: String,
    /// API secret
    api_secret: String,
    /// API passphrase
    passphrase: String,
    /// Base URL for REST API
    rest_url: String,
    /// Base URL for WebSocket
    ws_url: String,
    /// HTTP client
    http_client: Client,
    /// Last update timestamps for each symbol
    last_updates: Arc<RwLock<HashMap<String, u64>>>,
    /// Current connection status
    connected: Arc<RwLock<bool>>,
}

impl OkxClient {
    /// Create a new OKX client
    pub fn new(api_key: String, api_secret: String, passphrase: String, sandbox: bool) -> Self {
        let (rest_url, ws_url) = if sandbox {
            (
                "https://www.okx.com/api/v5".to_string(),
                "wss://wspap.okx.com:8443/ws/v5/public".to_string(),
            )
        } else {
            (
                "https://www.okx.com/api/v5".to_string(),
                "wss://ws.okx.com:8443/ws/v5/public".to_string(),
            )
        };

        Self {
            api_key,
            api_secret,
            passphrase,
            rest_url,
            ws_url,
            http_client: Client::new(),
            last_updates: Arc::new(RwLock::new(HashMap::new())),
            connected: Arc::new(RwLock::new(false)),
        }
    }

    /// Generate signature for API request
    fn sign(&self, timestamp: &str, method: &str, request_path: &str, body: &str) -> String {
        let message = format!("{}{}{}{}", timestamp, method, request_path, body);
        let mut mac = Hmac::<Sha256>::new_from_slice(self.api_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(message.as_bytes());
        let result = mac.finalize();
        let code_bytes = result.into_bytes();
        general_purpose::STANDARD.encode(code_bytes)
    }

    /// Get current server time
    pub async fn get_server_time(&self) -> Result<u64, OkxError> {
        let url = format!("{}/public/time", self.rest_url);
        let response = self.http_client.get(&url).send().await
            .map_err(|e| OkxError::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(OkxError::ApiError(format!("Failed to get server time: {}", response.status())));
        }
        
        let json: Value = response.json().await
            .map_err(|e| OkxError::ParseError(e.to_string()))?;
        
        json.get("data")
            .and_then(|d| d.get(0))
            .and_then(|item| item.get("ts"))
            .and_then(|v| v.as_u64())
            .ok_or_else(|| OkxError::ParseError("Invalid server time response".to_string()))
    }

    /// Get exchange information for symbols
    pub async fn get_exchange_info(&self) -> Result<Value, OkxError> {
        let url = format!("{}/public/instruments?instType=SPOT", self.rest_url);
        let response = self.http_client.get(&url).send().await
            .map_err(|e| OkxError::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(OkxError::ApiError(format!("Failed to get exchange info: {}", response.status())));
        }
        
        response.json().await
            .map_err(|e| OkxError::ParseError(e.to_string()))
    }

    /// Get order book snapshot for a symbol
    pub async fn get_order_book(&self, symbol: &str, limit: u32) -> Result<OrderBookSnapshot, OkxError> {
        let url = format!(
            "{}/market/books?instId={}&sz={}",
            self.rest_url, symbol, limit
        );
        
        let response = self.http_client.get(&url).send().await
            .map_err(|e| OkxError::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(OkxError::ApiError(format!("Failed to get order book: {}", response.status())));
        }
        
        let json: Value = response.json().await
            .map_err(|e| OkxError::ParseError(e.to_string()))?;
        
        // Parse bids and asks
        let data = json.get("data")
            .and_then(|d| d.get(0))
            .ok_or_else(|| OkxError::ParseError("Invalid data in order book".to_string()))?;
        
        let bids = data.get("bids")
            .and_then(|v| v.as_array())
            .ok_or_else(|| OkxError::ParseError("Invalid bids in order book".to_string()))?
            .iter()
            .filter_map(|level| {
                if let (Some(price_str), Some(size_str)) = (level.get(0).and_then(|v| v.as_str()), level.get(1).and_then(|v| v.as_str())) {
                    let price = Price::from_str(price_str).ok()?;
                    let size = Size::from_str(size_str).ok()?;
                    Some(OrderBookLevel::new(price, size))
                } else {
                    None
                }
            })
            .collect();
        
        let asks = data.get("asks")
            .and_then(|v| v.as_array())
            .ok_or_else(|| OkxError::ParseError("Invalid asks in order book".to_string()))?
            .iter()
            .filter_map(|level| {
                if let (Some(price_str), Some(size_str)) = (level.get(0).and_then(|v| v.as_str()), level.get(1).and_then(|v| v.as_str())) {
                    let price = Price::from_str(price_str).ok()?;
                    let size = Size::from_str(size_str).ok()?;
                    Some(OrderBookLevel::new(price, size))
                } else {
                    None
                }
            })
            .collect();
        
        let timestamp = data.get("ts")
            .and_then(|v| v.as_u64())
            .unwrap_or_else(|| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64
            });
        
        Ok(OrderBookSnapshot::new(symbol.to_string(), bids, asks, timestamp))
    }

    /// Place a new order
    pub async fn place_order(&self, order: &NewOrder) -> Result<OrderId, OkxError> {
        let server_time = self.get_server_time().await?;
        let timestamp = server_time.to_string();
        
        // Convert symbol to OKX format (BTC-USDT instead of BTCUSDT)
        let okx_symbol = order.symbol.replace("USDT", "-USDT");
        
        let mut params = json!({
            "instId": okx_symbol,
            "tdMode": "cash", // Cash mode for spot trading
            "side": match order.side {
                OrderSide::Buy => "buy",
                OrderSide::Sell => "sell",
            },
            "ordType": match order.order_type {
                OrderType::Market => "market",
                OrderType::Limit => "limit",
                _ => "limit", // Default to limit for other types
            },
            "sz": order.quantity.to_string(),
        });
        
        if let Some(price) = order.price {
            params["px"] = json!(price.to_string());
        }
        
        if let Some(client_order_id) = &order.client_order_id {
            params["clOrdId"] = json!(client_order_id);
        }
        
        let body = params.to_string();
        let method = "POST";
        let request_path = "/api/v5/trade/order";
        
        // Generate signature
        let signature = self.sign(&timestamp, method, request_path, &body);
        
        let url = format!("{}{}", self.rest_url, request_path);
        
        let response = self.http_client
            .post(&url)
            .header("OK-ACCESS-KEY", &self.api_key)
            .header("OK-ACCESS-SIGN", signature)
            .header("OK-ACCESS-TIMESTAMP", timestamp)
            .header("OK-ACCESS-PASSPHRASE", &self.passphrase)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| OkxError::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OkxError::ApiError(format!("Failed to place order: {} - {}", response.status(), error_text)));
        }
        
        let json: Value = response.json().await
            .map_err(|e| OkxError::ParseError(e.to_string()))?;
        
        let order_id = json.get("data")
            .and_then(|d| d.get(0))
            .and_then(|item| item.get("ordId"))
            .and_then(|v| v.as_str())
            .map(|id| OrderId::new(id.to_string()))
            .ok_or_else(|| OkxError::ParseError("Invalid order ID in response".to_string()))?;
        
        Ok(order_id)
    }

    /// Cancel an order
    pub async fn cancel_order(&self, symbol: &str, order_id: OrderId) -> Result<(), OkxError> {
        let server_time = self.get_server_time().await?;
        let timestamp = server_time.to_string();
        
        // Convert symbol to OKX format
        let okx_symbol = symbol.replace("USDT", "-USDT");
        
        let params = json!({
            "instId": okx_symbol,
            "ordId": order_id.as_str(),
        });
        
        let body = params.to_string();
        let method = "POST";
        let request_path = "/api/v5/trade/cancel-order";
        
        // Generate signature
        let signature = self.sign(&timestamp, method, request_path, &body);
        
        let url = format!("{}{}", self.rest_url, request_path);
        
        let response = self.http_client
            .post(&url)
            .header("OK-ACCESS-KEY", &self.api_key)
            .header("OK-ACCESS-SIGN", signature)
            .header("OK-ACCESS-TIMESTAMP", timestamp)
            .header("OK-ACCESS-PASSPHRASE", &self.passphrase)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| OkxError::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OkxError::ApiError(format!("Failed to cancel order: {} - {}", response.status(), error_text)));
        }
        
        Ok(())
    }

    /// Get account information
    pub async fn get_account_info(&self) -> Result<Vec<Balance>, OkxError> {
        let server_time = self.get_server_time().await?;
        let timestamp = server_time.to_string();
        
        let body = "".to_string();
        let method = "GET";
        let request_path = "/api/v5/account/balance";
        
        // Generate signature
        let signature = self.sign(&timestamp, method, request_path, &body);
        
        let url = format!("{}{}", self.rest_url, request_path);
        
        let response = self.http_client
            .get(&url)
            .header("OK-ACCESS-KEY", &self.api_key)
            .header("OK-ACCESS-SIGN", signature)
            .header("OK-ACCESS-TIMESTAMP", timestamp)
            .header("OK-ACCESS-PASSPHRASE", &self.passphrase)
            .send()
            .await
            .map_err(|e| OkxError::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OkxError::ApiError(format!("Failed to get account info: {} - {}", response.status(), error_text)));
        }
        
        let json: Value = response.json().await
            .map_err(|e| OkxError::ParseError(e.to_string()))?;
        
        let balances = json.get("data")
            .and_then(|d| d.get(0))
            .and_then(|account| account.get("details"))
            .and_then(|details| details.as_array())
            .ok_or_else(|| OkxError::ParseError("Invalid balances in response".to_string()))?
            .iter()
            .filter_map(|balance| {
                let asset = balance.get("ccy")?.as_str()?.to_string();
                let free = balance.get("availBal")?.as_str()?;
                let locked = balance.get("frozenBal")?.as_str()?;
                
                Some(Balance::new(
                    asset,
                    Size::from_str(free).ok()?,
                    Size::from_str(locked).ok()?,
                ))
            })
            .collect();
        
        Ok(balances)
    }

    /// Get open orders
    pub async fn get_open_orders(&self, symbol: Option<&str>) -> Result<Vec<ExecutionReport>, OkxError> {
        let server_time = self.get_server_time().await?;
        let timestamp = server_time.to_string();
        
        let mut request_path = "/api/v5/trade/orders-pending".to_string();
        
        if let Some(sym) = symbol {
            // Convert symbol to OKX format
            let okx_symbol = sym.replace("USDT", "-USDT");
            request_path = format!("{}?instId={}", request_path, okx_symbol);
        }
        
        let body = "".to_string();
        let method = "GET";
        
        // Generate signature
        let signature = self.sign(&timestamp, method, &request_path, &body);
        
        let url = format!("{}{}", self.rest_url, request_path);
        
        let response = self.http_client
            .get(&url)
            .header("OK-ACCESS-KEY", &self.api_key)
            .header("OK-ACCESS-SIGN", signature)
            .header("OK-ACCESS-TIMESTAMP", timestamp)
            .header("OK-ACCESS-PASSPHRASE", &self.passphrase)
            .send()
            .await
            .map_err(|e| OkxError::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OkxError::ApiError(format!("Failed to get open orders: {} - {}", response.status(), error_text)));
        }
        
        let json: Value = response.json().await
            .map_err(|e| OkxError::ParseError(e.to_string()))?;
        
        let orders = json.get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| OkxError::ParseError("Invalid orders in response".to_string()))?
            .iter()
            .filter_map(|order| {
                let order_id = order.get("ordId")?.as_str()?.to_string();
                let client_order_id = order.get("clOrdId")?.as_str().map(|s| s.to_string());
                let symbol = order.get("instId")?.as_str()?.to_string();
                // Convert OKX symbol format back to standard format
                let symbol = symbol.replace("-USDT", "USDT");
                
                let status = match order.get("state")?.as_str()? {
                    "live" => OrderStatus::New,
                    "partially_filled" => {
                        let filled_size = Size::from_str(order.get("fillSz")?.as_str()?).ok()?;
                        let remaining_size = Size::from_str(order.get("sz")?.as_str()?).ok()? - filled_size.value();
                        OrderStatus::PartiallyFilled { filled_size, remaining_size }
                    }
                    "filled" => {
                        let filled_size = Size::from_str(order.get("fillSz")?.as_str()?).ok()?;
                        OrderStatus::Filled { filled_size }
                    }
                    "canceled" => {
                        let remaining_size = Size::from_str(order.get("sz")?.as_str()?).ok()?;
                        OrderStatus::Canceled { remaining_size }
                    }
                    _ => return None,
                };
                let side = match order.get("side")?.as_str()? {
                    "buy" => OrderSide::Buy,
                    "sell" => OrderSide::Sell,
                    _ => return None,
                };
                let order_type = match order.get("ordType")?.as_str()? {
                    "market" => OrderType::Market,
                    "limit" => OrderType::Limit,
                    _ => return None,
                };
                let time_in_force = match order.get("tdMode")?.as_str()? {
                    "cash" => TimeInForce::GoodTillCancelled, // Default to GTC for cash mode
                    _ => TimeInForce::GoodTillCancelled,
                };
                let quantity = Size::from_str(order.get("sz")?.as_str()?).ok()?;
                let price = order.get("px")
                    .and_then(|p| p.as_str())
                    .and_then(|p_str| Price::from_str(p_str).ok());
                let timestamp = order.get("cTime")?.as_u64()?;
                
                Some(ExecutionReport {
                    order_id: OrderId::new(order_id),
                    client_order_id,
                    symbol,
                    status,
                    side,
                    order_type,
                    time_in_force,
                    quantity,
                    price,
                    timestamp,
                })
            })
            .collect();
        
        Ok(orders)
    }
}

/// OKX WebSocket stream for market data
pub struct OkxWebSocket {
    /// WebSocket connection
    ws_sender: Option<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
    /// Subscribed symbols
    subscriptions: Arc<RwLock<Vec<String>>>,
    /// Connection status
    connected: Arc<RwLock<bool>>,
    /// Last update timestamps
    last_updates: Arc<RwLock<HashMap<String, u64>>>,
}

impl OkxWebSocket {
    /// Create a new OKX WebSocket stream
    pub fn new() -> Self {
        Self {
            ws_sender: None,
            subscriptions: Arc::new(RwLock::new(Vec::new())),
            connected: Arc::new(RwLock::new(false)),
            last_updates: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Connect to the WebSocket stream
    pub async fn connect(&mut self, symbols: &[&str]) -> Result<(), OkxError> {
        let (ws_stream, _) = connect_async("wss://ws.okx.com:8443/ws/v5/public").await
            .map_err(|e| OkxError::ConnectionError(e.to_string()))?;
        
        self.ws_sender = Some(ws_stream);
        
        // Subscribe to order book data for each symbol
        if let Some(ws) = &mut self.ws_sender {
            for symbol in symbols {
                // Convert symbol to OKX format
                let okx_symbol = symbol.replace("USDT", "-USDT");
                
                let subscribe_msg = json!({
                    "op": "subscribe",
                    "args": [{
                        "channel": "books",
                        "instId": okx_symbol
                    }]
                });
                
                ws.send(Message::Text(subscribe_msg.to_string())).await
                    .map_err(|e| OkxError::ConnectionError(e.to_string()))?;
            }
        }
        
        // Update subscriptions
        let mut subs = self.subscriptions.write().await;
        for symbol in symbols {
            if !subs.contains(&symbol.to_string()) {
                subs.push(symbol.to_string());
            }
        }
        
        // Update connection status
        let mut connected = self.connected.write().await;
        *connected = true;
        
        Ok(())
    }

    /// Disconnect from the WebSocket stream
    pub async fn disconnect(&mut self) -> Result<(), OkxError> {
        if let Some(mut ws) = self.ws_sender.take() {
            ws.close(None).await
                .map_err(|e| OkxError::ConnectionError(e.to_string()))?;
        }
        
        // Update connection status
        let mut connected = self.connected.write().await;
        *connected = false;
        
        Ok(())
    }
}

#[async_trait]
impl MarketDataStream for OkxWebSocket {
    type Error = OkxError;

    async fn subscribe(&mut self, symbols: &[&str]) -> Result<(), Self::Error> {
        // For OKX, we can subscribe to additional symbols without reconnecting
        if let Some(ws) = &mut self.ws_sender {
            for symbol in symbols {
                // Check if already subscribed
                let subs = self.subscriptions.read().await;
                if subs.contains(&symbol.to_string()) {
                    continue;
                }
                drop(subs);
                
                // Convert symbol to OKX format
                let okx_symbol = symbol.replace("USDT", "-USDT");
                
                let subscribe_msg = json!({
                    "op": "subscribe",
                    "args": [{
                        "channel": "books",
                        "instId": okx_symbol
                    }]
                });
                
                ws.send(Message::Text(subscribe_msg.to_string())).await
                    .map_err(|e| OkxError::ConnectionError(e.to_string()))?;
                
                // Update subscriptions
                let mut subs = self.subscriptions.write().await;
                subs.push(symbol.to_string());
            }
        } else {
            // Not connected, need to connect first
            self.connect(symbols).await?;
        }
        
        Ok(())
    }

    async fn unsubscribe(&mut self, symbols: &[&str]) -> Result<(), Self::Error> {
        if let Some(ws) = &mut self.ws_sender {
            for symbol in symbols {
                // Convert symbol to OKX format
                let okx_symbol = symbol.replace("USDT", "-USDT");
                
                let unsubscribe_msg = json!({
                    "op": "unsubscribe",
                    "args": [{
                        "channel": "books",
                        "instId": okx_symbol
                    }]
                });
                
                ws.send(Message::Text(unsubscribe_msg.to_string())).await
                    .map_err(|e| OkxError::ConnectionError(e.to_string()))?;
                
                // Update subscriptions
                let mut subs = self.subscriptions.write().await;
                subs.retain(|s| !symbols.contains(&s.as_str()));
            }
        }
        
        Ok(())
    }

    async fn next(&mut self) -> Option<Result<MarketEvent, Self::Error>> {
        if let Some(ws) = &mut self.ws_sender {
            match ws.next().await {
                Some(Ok(Message::Text(text))) => {
                    // Parse JSON message
                    let json: Value = serde_json::from_str(&text)
                        .map_err(|e| OkxError::ParseError(e.to_string()))?;
                    
                    // Check if it's a data message
                    if let Some("data") = json.get("event").and_then(|e| e.as_str()) {
                        // Convert to MarketEvent
                        // This is a simplified implementation
                        // In a real implementation, you'd parse the full OKX message format
                        Some(Err(OkxError::ParseError("Not implemented".to_string())))
                    } else {
                        // Ignore other message types
                        None
                    }
                }
                Some(Ok(Message::Close(_))) => {
                    // Connection closed
                    let mut connected = self.connected.write().await;
                    *connected = false;
                    None
                }
                Some(Err(e)) => {
                    // WebSocket error
                    Some(Err(OkxError::ConnectionError(e.to_string())))
                }
                _ => None, // Ignore other message types
            }
        } else {
            None
        }
    }

    fn is_connected(&self) -> bool {
        // This is a synchronous method, so we can't use async here
        // In a real implementation, you might use a different approach
        true // For simplicity, always return true
    }

    fn last_update(&self, symbol: &str) -> Option<u64> {
        // This is a synchronous method, so we can't use async here
        // In a real implementation, you might use a different approach
        None // For simplicity, always return None
    }
}

/// OKX error types
#[derive(Debug, Clone)]
pub enum OkxError {
    NetworkError(String),
    ConnectionError(String),
    ApiError(String),
    ParseError(String),
    AuthenticationError(String),
    RateLimitError(String),
}

impl std::fmt::Display for OkxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OkxError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            OkxError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            OkxError::ApiError(msg) => write!(f, "API error: {}", msg),
            OkxError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            OkxError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            OkxError::RateLimitError(msg) => write!(f, "Rate limit error: {}", msg),
        }
    }
}

impl std::error::Error for OkxError {}

/// OKX adapter that implements both MarketDataStream and ExecutionClient
pub struct OkxAdapter {
    /// OKX client for REST API
    client: OkxClient,
    /// OKX WebSocket for market data
    websocket: Arc<Mutex<OkxWebSocket>>,
}

impl OkxAdapter {
    /// Create a new OKX adapter
    pub fn new(api_key: String, api_secret: String, passphrase: String, sandbox: bool) -> Self {
        Self {
            client: OkxClient::new(api_key, api_secret, passphrase, sandbox),
            websocket: Arc::new(Mutex::new(OkxWebSocket::new())),
        }
    }
}

#[async_trait]
impl ExecutionClient for OkxAdapter {
    type Error = OkxError;

    async fn place_order(&self, order: NewOrder) -> Result<OrderId, Self::Error> {
        self.client.place_order(&order).await
    }

    async fn cancel_order(&self, order_id: OrderId) -> Result<(), Self::Error> {
        // For OKX, we need to know the symbol to cancel an order
        // In a real implementation, we'd track this information
        // For now, we'll return an error
        Err(OkxError::ApiError("Symbol required to cancel order".to_string()))
    }

    async fn get_order_status(&self, order_id: OrderId) -> Result<ExecutionReport, Self::Error> {
        // For OKX, we need to know the symbol to get order status
        // In a real implementation, we'd track this information
        // For now, we'll return an error
        Err(OkxError::ApiError("Symbol required to get order status".to_string()))
    }

    async fn get_balances(&self) -> Result<Vec<Balance>, Self::Error> {
        self.client.get_account_info().await
    }

    async fn get_open_orders(&self, symbol: Option<&str>) -> Result<Vec<ExecutionReport>, Self::Error> {
        self.client.get_open_orders(symbol).await
    }

    async fn get_order_history(
        &self,
        symbol: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<ExecutionReport>, Self::Error> {
        // For OKX, we'd use the /api/v5/trade/orders-history endpoint
        // For now, we'll return an error
        Err(OkxError::ApiError("Not implemented".to_string()))
    }

    async fn get_trading_fees(&self, symbol: &str) -> Result<TradingFees, Self::Error> {
        // For OKX, we'd use the /api/v5/account/trade-fee endpoint
        // For now, we'll return default fees
        Ok(TradingFees::new(
            symbol.to_string(),
            Size::from_str("0.0008").unwrap(), // 0.08% maker fee
            Size::from_str("0.001").unwrap(), // 0.1% taker fee
        ))
    }
}

#[async_trait]
impl MarketDataHistory for OkxAdapter {
    type Error = OkxError;

    async fn get_order_book_snapshots(
        &self,
        symbol: &str,
        start_time: u64,
        end_time: u64,
    ) -> Result<Vec<OrderBookSnapshot>, Self::Error> {
        // OKX doesn't provide historical order book snapshots
        // In a real implementation, we'd store snapshots ourselves
        Err(OkxError::ApiError("Historical order book snapshots not available".to_string()))
    }

    async fn get_trades(
        &self,
        symbol: &str,
        start_time: u64,
        end_time: u64,
    ) -> Result<Vec<Trade>, Self::Error> {
        // For OKX, we'd use the /api/v5/trade/fills-history endpoint
        // For now, we'll return an error
        Err(OkxError::ApiError("Not implemented".to_string()))
    }
}

#[async_trait]
impl crate::exchanges::connection_manager::ExchangeAdapter for OkxAdapter {
    async fn connect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // OKX connection is handled through WebSocket subscription
        // For now, just return Ok
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut ws = self.websocket.lock().await;
        ws.disconnect().await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn get_market_data_stream(&self) -> Result<Arc<tokio::sync::Mutex<dyn MarketDataStream<Error = Box<dyn std::error::Error + Send + Sync>> + Send + Sync>>, Box<dyn std::error::Error + Send + Sync>> {
        // Return a wrapper that implements the required trait
        Ok(Arc::new(tokio::sync::Mutex::new(OkxWebSocketAdapter {
            websocket: self.websocket.clone(),
        })))
    }

    async fn place_order(&self, order: NewOrder) -> Result<OrderId, Box<dyn std::error::Error + Send + Sync>> {
        self.client.place_order(&order).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn cancel_order(&self, order_id: OrderId) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // For OKX, we need the symbol - this is a limitation
        Err("Symbol required to cancel order".into())
    }

    async fn get_order_status(&self, order_id: OrderId) -> Result<ExecutionReport, Box<dyn std::error::Error + Send + Sync>> {
        // For OKX, we need the symbol - this is a limitation
        Err("Symbol required to get order status".into())
    }

    async fn get_balances(&self) -> Result<Vec<Balance>, Box<dyn std::error::Error + Send + Sync>> {
        self.client.get_account_info().await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn get_open_orders(&self, symbol: Option<&str>) -> Result<Vec<ExecutionReport>, Box<dyn std::error::Error + Send + Sync>> {
        self.client.get_open_orders(symbol).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn get_order_book(&self, symbol: &str, limit: u32) -> Result<OrderBookSnapshot, Box<dyn std::error::Error + Send + Sync>> {
        self.client.get_order_book(symbol, limit).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn get_trading_fees(&self, symbol: &str) -> Result<TradingFees, Box<dyn std::error::Error + Send + Sync>> {
        Ok(TradingFees::new(
            symbol.to_string(),
            Size::from_str("0.0008").unwrap(),
            Size::from_str("0.001").unwrap(),
        ))
    }
}

/// Wrapper for OkxWebSocket to implement the required MarketDataStream trait
pub struct OkxWebSocketAdapter {
    websocket: Arc<tokio::sync::Mutex<OkxWebSocket>>,
}

#[async_trait]
impl MarketDataStream for OkxWebSocketAdapter {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    async fn subscribe(&mut self, symbols: &[&str]) -> Result<(), Self::Error> {
        let mut ws = self.websocket.lock().await;
        ws.subscribe(symbols).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn unsubscribe(&mut self, symbols: &[&str]) -> Result<(), Self::Error> {
        let mut ws = self.websocket.lock().await;
        ws.unsubscribe(symbols).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn next(&mut self) -> Option<Result<MarketEvent, Self::Error>> {
        let mut ws = self.websocket.lock().await;
        ws.next().await.map(|r| r.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>))
    }

    fn is_connected(&self) -> bool {
        // This is a limitation - we can't easily check connection status synchronously
        true
    }

    fn last_update(&self, symbol: &str) -> Option<u64> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_okx_client_creation() {
        let client = OkxClient::new(
            "test_key".to_string(),
            "test_secret".to_string(),
            "test_passphrase".to_string(),
            true, // sandbox
        );
        
        // Verify client was created with correct URLs
        assert_eq!(client.rest_url, "https://www.okx.com/api/v5");
        assert_eq!(client.ws_url, "wss://wspap.okx.com:8443/ws/v5/public");
    }

    #[test]
    fn test_okx_websocket_creation() {
        let ws = OkxWebSocket::new();
        
        // Verify WebSocket was created
        assert!(ws.ws_sender.is_none());
        assert!(ws.subscriptions.try_read().unwrap().is_empty());
    }

    #[test]
    fn test_okx_adapter_creation() {
        let adapter = OkxAdapter::new(
            "test_key".to_string(),
            "test_secret".to_string(),
            "test_passphrase".to_string(),
            true, // sandbox
        );
        
        // Verify adapter was created
        // We can't easily test the internal structure without exposing it
        // In a real test, we'd test the behavior
    }

    #[test]
    fn test_okx_error_display() {
        let error = OkxError::NetworkError("Connection failed".to_string());
        assert_eq!(error.to_string(), "Network error: Connection failed");
        
        let error = OkxError::ApiError("Invalid symbol".to_string());
        assert_eq!(error.to_string(), "API error: Invalid symbol");
    }
}
