use async_trait::async_trait;
use crate::traits::{
    MarketDataStream, MarketDataHistory, ExecutionClient, OrderManager,
    MarketEvent, NewOrder, OrderId, ExecutionReport, OrderStatus, OrderSide, OrderType, TimeInForce,
    Balance, TradingFees, Trade
};
use crate::types::{Price, Size};
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
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Gate.io API client for market data and order execution
pub struct GateClient {
    /// API key
    api_key: String,
    /// API secret
    api_secret: String,
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

impl GateClient {
    /// Create a new Gate.io client
    pub fn new(api_key: String, api_secret: String, testnet: bool) -> Self {
        let (rest_url, ws_url) = if testnet {
            (
                "https://fx-api-testnet.gateio.ws".to_string(),
                "wss://fx-ws-testnet.gateio.ws/v4/ws".to_string(),
            )
        } else {
            (
                "https://api.gateio.ws".to_string(),
                "wss://fx-ws.gateio.ws/v4/ws".to_string(),
            )
        };

        Self {
            api_key,
            api_secret,
            rest_url,
            ws_url,
            http_client: Client::new(),
            last_updates: Arc::new(RwLock::new(HashMap::new())),
            connected: Arc::new(RwLock::new(false)),
        }
    }

    /// Generate signature for API request
    fn sign(&self, method: &str, url_path: &str, query_string: &str, payload: &str) -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();
        
        let sign_string = format!("{}\n{}\n{}\n{}\n{}", method, url_path, query_string, 
            sha2::Sha256::digest(payload.as_bytes()).iter().map(|b| format!("{:02x}", b)).collect::<String>(), timestamp);
        
        let mut mac = Hmac::<Sha256>::new_from_slice(self.api_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(sign_string.as_bytes());
        let result = mac.finalize();
        let code_bytes = result.into_bytes();
        general_purpose::STANDARD.encode(code_bytes)
    }

    /// Get current server time
    pub async fn get_server_time(&self) -> Result<u64, GateError> {
        let url = format!("{}/api/v4/time", self.rest_url);
        let response = self.http_client.get(&url).send().await
            .map_err(|e| GateError::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(GateError::ApiError(format!("Failed to get server time: {}", response.status())));
        }
        
        let json: Value = response.json().await
            .map_err(|e| GateError::ParseError(e.to_string()))?;
        
        json.get("server_time")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| GateError::ParseError("Invalid server time response".to_string()))
    }

    /// Get order book snapshot for a symbol
    pub async fn get_order_book(&self, symbol: &str, limit: u32) -> Result<OrderBookSnapshot, GateError> {
        let url = format!(
            "{}/api/v4/spot/order_book?currency_pair={}&limit={}",
            self.rest_url, symbol, limit
        );
        
        let response = self.http_client.get(&url).send().await
            .map_err(|e| GateError::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(GateError::ApiError(format!("Failed to get order book: {}", response.status())));
        }
        
        let json: Value = response.json().await
            .map_err(|e| GateError::ParseError(e.to_string()))?;
        
        // Parse bids and asks
        let bids = json.get("bids")
            .and_then(|v| v.as_array())
            .ok_or_else(|| GateError::ParseError("Invalid bids in order book".to_string()))?
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
        
        let asks = json.get("asks")
            .and_then(|v| v.as_array())
            .ok_or_else(|| GateError::ParseError("Invalid asks in order book".to_string()))?
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
        
        let timestamp = json.get("update")
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
    pub async fn place_order(&self, order: &NewOrder) -> Result<OrderId, GateError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();
        
        let mut params = json!({
            "currency_pair": order.symbol,
            "side": match order.side {
                OrderSide::Buy => "buy",
                OrderSide::Sell => "sell",
            },
            "amount": order.quantity.to_string(),
        });
        
        if let Some(price) = order.price {
            params["price"] = json!(price.to_string());
            params["type"] = json!("limit");
        } else {
            params["type"] = json!("market");
        }
        
        if let Some(client_order_id) = &order.client_order_id {
            params["text"] = json!(client_order_id);
        }
        
        let body = params.to_string();
        let method = "POST";
        let url_path = "/api/v4/spot/orders";
        let query_string = "";
        
        // Generate signature
        let signature = self.sign(method, url_path, query_string, &body);
        
        let url = format!("{}{}", self.rest_url, url_path);
        
        let response = self.http_client
            .post(&url)
            .header("KEY", &self.api_key)
            .header("Timestamp", &timestamp)
            .header("SIGN", signature)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| GateError::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(GateError::ApiError(format!("Failed to place order: {} - {}", response.status(), error_text)));
        }
        
        let json: Value = response.json().await
            .map_err(|e| GateError::ParseError(e.to_string()))?;
        
        let order_id = json.get("id")
            .and_then(|v| v.as_str())
            .map(|id| OrderId::new(id.to_string()))
            .or_else(|| {
                json.get("id")
                    .and_then(|v| v.as_i64())
                    .map(|id| OrderId::new(id.to_string()))
            })
            .ok_or_else(|| GateError::ParseError("Invalid order ID in response".to_string()))?;
        
        Ok(order_id)
    }

    /// Cancel an order
    pub async fn cancel_order(&self, symbol: &str, order_id: OrderId) -> Result<(), GateError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();
        
        let url_path = format!("/api/v4/spot/orders/{}", order_id.as_str());
        let query_string = format!("currency_pair={}", symbol);
        let body = "".to_string();
        let method = "DELETE";
        
        // Generate signature
        let signature = self.sign(method, &url_path, &query_string, &body);
        
        let url = format!("{}?{}", format!("{}{}", self.rest_url, url_path), query_string);
        
        let response = self.http_client
            .delete(&url)
            .header("KEY", &self.api_key)
            .header("Timestamp", &timestamp)
            .header("SIGN", signature)
            .send()
            .await
            .map_err(|e| GateError::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(GateError::ApiError(format!("Failed to cancel order: {} - {}", response.status(), error_text)));
        }
        
        Ok(())
    }

    /// Get account information
    pub async fn get_account_info(&self) -> Result<Vec<Balance>, GateError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();
        
        let url_path = "/api/v4/spot/accounts";
        let query_string = "";
        let body = "".to_string();
        let method = "GET";
        
        // Generate signature
        let signature = self.sign(method, url_path, query_string, &body);
        
        let url = format!("{}{}", self.rest_url, url_path);
        
        let response = self.http_client
            .get(&url)
            .header("KEY", &self.api_key)
            .header("Timestamp", &timestamp)
            .header("SIGN", signature)
            .send()
            .await
            .map_err(|e| GateError::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(GateError::ApiError(format!("Failed to get account info: {} - {}", response.status(), error_text)));
        }
        
        let json: Value = response.json().await
            .map_err(|e| GateError::ParseError(e.to_string()))?;
        
        let balances = json.as_array()
            .ok_or_else(|| GateError::ParseError("Invalid balances in response".to_string()))?
            .iter()
            .filter_map(|balance| {
                let asset = balance.get("currency")?.as_str()?.to_string();
                let free = balance.get("available")?.as_str()?;
                let locked = balance.get("locked")?.as_str()?;
                
                Some(Balance {
                    asset,
                    exchange_id: "gate".to_string(),
                    total: Size::from_str(free).ok()?.value() + Size::from_str(locked).ok()?.value(),
                    free: Size::from_str(free).ok()?.value(),
                    used: Size::from_str(locked).ok()?.value(),
                })
            })
            .collect();
        
        Ok(balances)
    }

    /// Get open orders
    pub async fn get_open_orders(&self, symbol: Option<&str>) -> Result<Vec<ExecutionReport>, GateError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();
        
        let mut url_path = "/api/v4/spot/open_orders".to_string();
        let mut query_string = String::new();
        if let Some(sym) = symbol {
            query_string = format!("currency_pair={}", sym);
        }
        
        let body = "".to_string();
        let method = "GET";
        
        // Generate signature
        let signature = self.sign(method, &url_path, &query_string, &body);
        
        let url = if query_string.is_empty() {
            format!("{}{}", self.rest_url, url_path)
        } else {
            format!("{}?{}", format!("{}{}", self.rest_url, url_path), query_string)
        };
        
        let response = self.http_client
            .get(&url)
            .header("KEY", &self.api_key)
            .header("Timestamp", &timestamp)
            .header("SIGN", signature)
            .send()
            .await
            .map_err(|e| GateError::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(GateError::ApiError(format!("Failed to get open orders: {} - {}", response.status(), error_text)));
        }
        
        let json: Value = response.json().await
            .map_err(|e| GateError::ParseError(e.to_string()))?;
        
        let orders = json.as_array()
            .ok_or_else(|| GateError::ParseError("Invalid orders in response".to_string()))?
            .iter()
            .filter_map(|order| {
                let order_id = order.get("id")?.as_str()?.to_string();
                let client_order_id = order.get("text")?.as_str().map(|s| s.to_string());
                let symbol = order.get("currency_pair")?.as_str()?.to_string();
                
                let status = match order.get("status")?.as_str()? {
                    "open" => OrderStatus::New,
                    "cancelled" => {
                        let remaining_size = Size::from_str(order.get("left")?.as_str()?).ok()?;
                        OrderStatus::Canceled { remaining_size }
                    }
                    "closed" => {
                        let filled_size = Size::from_str(order.get("filled_total")?.as_str()?).ok()?;
                        OrderStatus::Filled { filled_size }
                    }
                    _ => return None,
                };
                
                let side = match order.get("side")?.as_str()? {
                    "buy" => OrderSide::Buy,
                    "sell" => OrderSide::Sell,
                    _ => return None,
                };
                
                let order_type = match order.get("type")?.as_str()? {
                    "market" => OrderType::Market,
                    "limit" => OrderType::Limit,
                    _ => return None,
                };
                
                let time_in_force = TimeInForce::GoodTillCancelled; // Gate.io default
                let quantity = Size::from_str(order.get("amount")?.as_str()?).ok()?;
                let price = order.get("price")
                    .and_then(|p| p.as_str())
                    .and_then(|p_str| Price::from_str(p_str).ok());
                let timestamp = order.get("create_time_ms")?.as_u64()?;
                
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

/// Gate.io WebSocket stream for market data
pub struct GateWebSocket {
    /// WebSocket connection
    ws_sender: Option<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
    /// Subscribed symbols
    subscriptions: Arc<RwLock<Vec<String>>>,
    /// Connection status
    connected: Arc<RwLock<bool>>,
    /// Last update timestamps
    last_updates: Arc<RwLock<HashMap<String, u64>>>,
}

impl GateWebSocket {
    /// Create a new Gate.io WebSocket stream
    pub fn new() -> Self {
        Self {
            ws_sender: None,
            subscriptions: Arc::new(RwLock::new(Vec::new())),
            connected: Arc::new(RwLock::new(false)),
            last_updates: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Connect to the WebSocket stream
    pub async fn connect(&mut self, symbols: &[&str]) -> Result<(), GateError> {
        let (ws_stream, _) = connect_async("wss://fx-ws.gateio.ws/v4/ws").await
            .map_err(|e| GateError::ConnectionError(e.to_string()))?;
        
        self.ws_sender = Some(ws_stream);
        
        // Subscribe to order book data for each symbol
        if let Some(ws) = &mut self.ws_sender {
            for symbol in symbols {
                let subscribe_msg = json!({
                    "time": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                    "channel": "spot.order_book",
                    "event": "subscribe",
                    "payload": [symbol]
                });
                
                ws.send(Message::Text(subscribe_msg.to_string())).await
                    .map_err(|e| GateError::ConnectionError(e.to_string()))?;
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
    pub async fn disconnect(&mut self) -> Result<(), GateError> {
        if let Some(mut ws) = self.ws_sender.take() {
            ws.close(None).await
                .map_err(|e| GateError::ConnectionError(e.to_string()))?;
        }
        
        // Update connection status
        let mut connected = self.connected.write().await;
        *connected = false;
        
        Ok(())
    }
}

#[async_trait]
impl MarketDataStream for GateWebSocket {
    type Error = GateError;

    async fn subscribe(&mut self, symbols: &[&str]) -> Result<(), Self::Error> {
        if let Some(ws) = &mut self.ws_sender {
            for symbol in symbols {
                // Check if already subscribed
                let subs = self.subscriptions.read().await;
                if subs.contains(&symbol.to_string()) {
                    continue;
                }
                drop(subs);
                
                let subscribe_msg = json!({
                    "time": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                    "channel": "spot.order_book",
                    "event": "subscribe",
                    "payload": [symbol]
                });
                
                ws.send(Message::Text(subscribe_msg.to_string())).await
                    .map_err(|e| GateError::ConnectionError(e.to_string()))?;
                
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
                let unsubscribe_msg = json!({
                    "time": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                    "channel": "spot.order_book",
                    "event": "unsubscribe",
                    "payload": [symbol]
                });
                
                ws.send(Message::Text(unsubscribe_msg.to_string())).await
                    .map_err(|e| GateError::ConnectionError(e.to_string()))?;
                
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
                    let json: Value = match serde_json::from_str(&text) {
                        Ok(j) => j,
                        Err(e) => return Some(Err(GateError::ParseError(e.to_string()))),
                    };
                    
                    // Convert to MarketEvent
                    // This is a simplified implementation
                    // In a real implementation, you'd parse the full Gate.io message format
                    Some(Err(GateError::ParseError("Not implemented".to_string())))
                }
                Some(Ok(Message::Close(_))) => {
                    // Connection closed
                    let mut connected = self.connected.write().await;
                    *connected = false;
                    None
                }
                Some(Err(e)) => {
                    // WebSocket error
                    Some(Err(GateError::ConnectionError(e.to_string())))
                }
                _ => None, // Ignore other message types
            }
        } else {
            None
        }
    }

    fn is_connected(&self) -> bool {
        *self.connected.try_read().unwrap_or(&false)
    }

    fn last_update(&self, symbol: &str) -> Option<u64> {
        self.last_updates.try_read().ok()
            .and_then(|updates| updates.get(symbol).copied())
    }
}

/// Gate.io error types
#[derive(Debug, Clone)]
pub enum GateError {
    NetworkError(String),
    ConnectionError(String),
    ApiError(String),
    ParseError(String),
    AuthenticationError(String),
    RateLimitError(String),
}

impl std::fmt::Display for GateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GateError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            GateError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            GateError::ApiError(msg) => write!(f, "API error: {}", msg),
            GateError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            GateError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            GateError::RateLimitError(msg) => write!(f, "Rate limit error: {}", msg),
        }
    }
}

impl std::error::Error for GateError {}

/// Gate.io adapter that implements ExchangeAdapter
pub struct GateAdapter {
    /// Gate.io client for REST API
    client: GateClient,
    /// Gate.io WebSocket for market data
    websocket: Arc<Mutex<GateWebSocket>>,
}

impl GateAdapter {
    /// Create a new Gate.io adapter
    pub fn new(api_key: String, api_secret: String, testnet: bool) -> Self {
        Self {
            client: GateClient::new(api_key, api_secret, testnet),
            websocket: Arc::new(Mutex::new(GateWebSocket::new())),
        }
    }
}

#[async_trait]
impl ExchangeAdapter for GateAdapter {
    async fn connect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut ws = self.websocket.lock().await;
        ws.disconnect().await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn get_market_data_stream(&self) -> Result<Arc<Mutex<dyn MarketDataStream<Error = Box<dyn std::error::Error + Send + Sync>> + Send + Sync>>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Arc::new(Mutex::new(GateWebSocketAdapter {
            websocket: self.websocket.clone(),
        })))
    }

    async fn place_order(&self, order: NewOrder) -> Result<OrderId, Box<dyn std::error::Error + Send + Sync>> {
        self.client.place_order(&order).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn cancel_order(&self, order_id: OrderId) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // For Gate.io, we need the symbol - this is a limitation
        Err("Symbol required to cancel order".into())
    }

    async fn get_order_status(&self, order_id: OrderId) -> Result<ExecutionReport, Box<dyn std::error::Error + Send + Sync>> {
        // For Gate.io, we need the symbol - this is a limitation
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
        Ok(TradingFees {
            maker_fee: rust_decimal::Decimal::new(2, 4), // 0.0002 = 0.02%
            taker_fee: rust_decimal::Decimal::new(2, 4), // 0.0002 = 0.02%
        })
    }
}

/// Wrapper for GateWebSocket to implement the required MarketDataStream trait
pub struct GateWebSocketAdapter {
    websocket: Arc<tokio::sync::Mutex<GateWebSocket>>,
}

#[async_trait]
impl MarketDataStream for GateWebSocketAdapter {
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
        true
    }

    fn last_update(&self, symbol: &str) -> Option<u64> {
        self.websocket.try_lock().ok()
            .and_then(|ws| ws.last_update(symbol))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gate_client_creation() {
        let client = GateClient::new(
            "test_key".to_string(),
            "test_secret".to_string(),
            true, // testnet
        );
        
        // Verify client was created with correct URLs
        assert_eq!(client.rest_url, "https://fx-api-testnet.gateio.ws");
        assert_eq!(client.ws_url, "wss://fx-ws-testnet.gateio.ws/v4/ws");
    }

    #[test]
    fn test_gate_websocket_creation() {
        let ws = GateWebSocket::new();
        
        // Verify WebSocket was created
        assert!(ws.ws_sender.is_none());
        assert!(ws.subscriptions.try_read().unwrap().is_empty());
    }

    #[test]
    fn test_gate_adapter_creation() {
        let adapter = GateAdapter::new(
            "test_key".to_string(),
            "test_secret".to_string(),
            true, // testnet
        );
        
        // Verify adapter was created
    }

    #[test]
    fn test_gate_error_display() {
        let error = GateError::NetworkError("Connection failed".to_string());
        assert_eq!(error.to_string(), "Network error: Connection failed");
        
        let error = GateError::ApiError("Invalid symbol".to_string());
        assert_eq!(error.to_string(), "API error: Invalid symbol");
    }
}

