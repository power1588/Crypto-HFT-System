use crate::core::events::{OrderBookLevel, OrderBookSnapshot};
use crate::traits::{
    Balance, ExecutionClient, ExecutionReport, MarketDataHistory, MarketDataStream, MarketEvent,
    NewOrder, OrderId, OrderSide, OrderStatus, OrderType, TimeInForce, Trade, TradingFees,
};
use crate::types::{Price, Size};
use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use futures_util::StreamExt;
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde_json::Value;
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Binance API client for market data and order execution
#[allow(dead_code)]
pub struct BinanceClient {
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

impl BinanceClient {
    /// Create a new Binance client
    pub fn new(api_key: String, api_secret: String, testnet: bool) -> Self {
        let (rest_url, ws_url) = if testnet {
            (
                "https://testnet.binance.vision".to_string(),
                "wss://testnet.binance.vision/ws".to_string(),
            )
        } else {
            (
                "https://api.binance.com".to_string(),
                "wss://stream.binance.com/ws".to_string(),
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
    fn sign(&self, query_string: &str) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.api_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(query_string.as_bytes());
        let result = mac.finalize();
        let code_bytes = result.into_bytes();
        general_purpose::STANDARD.encode(code_bytes)
    }

    /// Get current server time
    pub async fn get_server_time(&self) -> Result<u64, BinanceError> {
        let url = format!("{}/api/v3/time", self.rest_url);
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| BinanceError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BinanceError::ApiError(format!(
                "Failed to get server time: {}",
                response.status()
            )));
        }

        let json: Value = response
            .json()
            .await
            .map_err(|e| BinanceError::ParseError(e.to_string()))?;

        json.get("serverTime")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| BinanceError::ParseError("Invalid server time response".to_string()))
    }

    /// Get exchange information for symbols
    pub async fn get_exchange_info(&self) -> Result<Value, BinanceError> {
        let url = format!("{}/api/v3/exchangeInfo", self.rest_url);
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| BinanceError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BinanceError::ApiError(format!(
                "Failed to get exchange info: {}",
                response.status()
            )));
        }

        response
            .json()
            .await
            .map_err(|e| BinanceError::ParseError(e.to_string()))
    }

    /// Get order book snapshot for a symbol
    pub async fn get_order_book(
        &self,
        symbol: &str,
        limit: u32,
    ) -> Result<OrderBookSnapshot, BinanceError> {
        let url = format!(
            "{}/api/v3/depth?symbol={}&limit={}",
            self.rest_url, symbol, limit
        );

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| BinanceError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BinanceError::ApiError(format!(
                "Failed to get order book: {}",
                response.status()
            )));
        }

        let json: Value = response
            .json()
            .await
            .map_err(|e| BinanceError::ParseError(e.to_string()))?;

        // Parse bids and asks
        let bids = json
            .get("bids")
            .and_then(|v| v.as_array())
            .ok_or_else(|| BinanceError::ParseError("Invalid bids in order book".to_string()))?
            .iter()
            .filter_map(|level| {
                if let (Some(price_str), Some(size_str)) = (
                    level.get(0).and_then(|v| v.as_str()),
                    level.get(1).and_then(|v| v.as_str()),
                ) {
                    let price = Price::from_str(price_str).ok()?;
                    let size = Size::from_str(size_str).ok()?;
                    Some(OrderBookLevel::new(price, size))
                } else {
                    None
                }
            })
            .collect();

        let asks = json
            .get("asks")
            .and_then(|v| v.as_array())
            .ok_or_else(|| BinanceError::ParseError("Invalid asks in order book".to_string()))?
            .iter()
            .filter_map(|level| {
                if let (Some(price_str), Some(size_str)) = (
                    level.get(0).and_then(|v| v.as_str()),
                    level.get(1).and_then(|v| v.as_str()),
                ) {
                    let price = Price::from_str(price_str).ok()?;
                    let size = Size::from_str(size_str).ok()?;
                    Some(OrderBookLevel::new(price, size))
                } else {
                    None
                }
            })
            .collect();

        let timestamp = json
            .get("lastUpdateId")
            .and_then(|v| v.as_u64())
            .unwrap_or_else(|| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64
            });

        Ok(OrderBookSnapshot::new(
            symbol, "binance", bids, asks, timestamp,
        ))
    }

    /// Place a new order
    pub async fn place_order(&self, order: &NewOrder) -> Result<OrderId, BinanceError> {
        let server_time = self.get_server_time().await?;

        let mut params = vec![
            ("symbol".to_string(), order.symbol.as_str().to_string()),
            (
                "side".to_string(),
                match order.side {
                    OrderSide::Buy => "BUY".to_string(),
                    OrderSide::Sell => "SELL".to_string(),
                },
            ),
            (
                "type".to_string(),
                match order.order_type {
                    OrderType::Market => "MARKET".to_string(),
                    OrderType::Limit => "LIMIT".to_string(),
                    _ => "LIMIT".to_string(), // Default to LIMIT for other types
                },
            ),
            ("quantity".to_string(), order.size.to_string()),
            ("timestamp".to_string(), server_time.to_string()),
        ];

        if let Some(price) = order.price {
            params.push(("price".to_string(), price.to_string()));
        }

        params.push((
            "timeInForce".to_string(),
            match order.time_in_force {
                TimeInForce::GoodTillCancelled => "GTC".to_string(),
                TimeInForce::ImmediateOrCancel => "IOC".to_string(),
                TimeInForce::FillOrKill => "FOK".to_string(),
            },
        ));

        if let Some(client_order_id) = &order.client_order_id {
            params.push(("newClientOrderId".to_string(), client_order_id.clone()));
        }

        // Create query string
        let query_string = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        // Add signature
        let signature = self.sign(&query_string);
        let signed_query = format!("{}&signature={}", query_string, signature);

        let url = format!("{}/api/v3/order", self.rest_url);

        let response = self
            .http_client
            .post(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(signed_query)
            .send()
            .await
            .map_err(|e| BinanceError::NetworkError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(BinanceError::ApiError(format!(
                "Failed to place order: {} - {}",
                status, error_text
            )));
        }

        let json: Value = response
            .json()
            .await
            .map_err(|e| BinanceError::ParseError(e.to_string()))?;

        let order_id: OrderId = json
            .get("orderId")
            .and_then(|v| v.as_i64())
            .map(|id| id.to_string())
            .ok_or_else(|| BinanceError::ParseError("Invalid order ID in response".to_string()))?;

        Ok(order_id)
    }

    /// Cancel an order
    pub async fn cancel_order(&self, symbol: &str, order_id: OrderId) -> Result<(), BinanceError> {
        let server_time = self.get_server_time().await?;

        let params = vec![
            ("symbol".to_string(), symbol.to_string()),
            ("orderId".to_string(), order_id.as_str().to_string()),
            ("timestamp".to_string(), server_time.to_string()),
        ];

        // Create query string
        let query_string = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        // Add signature
        let signature = self.sign(&query_string);
        let signed_query = format!("{}&signature={}", query_string, signature);

        let url = format!("{}/api/v3/order", self.rest_url);

        let response = self
            .http_client
            .delete(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(signed_query)
            .send()
            .await
            .map_err(|e| BinanceError::NetworkError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(BinanceError::ApiError(format!(
                "Failed to cancel order: {} - {}",
                status, error_text
            )));
        }

        Ok(())
    }

    /// Get account information
    pub async fn get_account_info(&self) -> Result<Vec<Balance>, BinanceError> {
        let server_time = self.get_server_time().await?;

        let params = vec![("timestamp".to_string(), server_time.to_string())];

        // Create query string
        let query_string = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        // Add signature
        let signature = self.sign(&query_string);
        let signed_query = format!("{}&signature={}", query_string, signature);

        let url = format!("{}/api/v3/account", self.rest_url);

        let response = self
            .http_client
            .get(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .body(signed_query)
            .send()
            .await
            .map_err(|e| BinanceError::NetworkError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(BinanceError::ApiError(format!(
                "Failed to get account info: {} - {}",
                status, error_text
            )));
        }

        let json: Value = response
            .json()
            .await
            .map_err(|e| BinanceError::ParseError(e.to_string()))?;

        let balances = json
            .get("balances")
            .and_then(|v| v.as_array())
            .ok_or_else(|| BinanceError::ParseError("Invalid balances in response".to_string()))?
            .iter()
            .filter_map(|balance| {
                let asset = balance.get("asset")?.as_str()?.to_string();
                let free = balance.get("free")?.as_str()?;
                let locked = balance.get("locked")?.as_str()?;

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
    pub async fn get_open_orders(
        &self,
        symbol: Option<&str>,
    ) -> Result<Vec<ExecutionReport>, BinanceError> {
        let server_time = self.get_server_time().await?;

        let mut params = vec![("timestamp".to_string(), server_time.to_string())];

        if let Some(sym) = symbol {
            params.push(("symbol".to_string(), sym.to_string()));
        }

        // Create query string
        let query_string = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        // Add signature
        let signature = self.sign(&query_string);
        let signed_query = format!("{}&signature={}", query_string, signature);

        let url = format!("{}/api/v3/openOrders", self.rest_url);

        let response = self
            .http_client
            .get(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .body(signed_query)
            .send()
            .await
            .map_err(|e| BinanceError::NetworkError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(BinanceError::ApiError(format!(
                "Failed to get open orders: {} - {}",
                status, error_text
            )));
        }

        let json: Value = response
            .json()
            .await
            .map_err(|e| BinanceError::ParseError(e.to_string()))?;

        let orders = json
            .as_array()
            .ok_or_else(|| BinanceError::ParseError("Invalid orders in response".to_string()))?
            .iter()
            .filter_map(|order| {
                use crate::types::Symbol;

                let order_id = order.get("orderId")?.as_i64()?.to_string();
                let client_order_id = order
                    .get("clientOrderId")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let symbol_str = order.get("symbol")?.as_str()?;
                let orig_qty = Size::from_str(order.get("origQty")?.as_str()?).ok()?;
                let executed_qty = Size::from_str(order.get("executedQty")?.as_str()?).ok()?;

                let status = match order.get("status")?.as_str()? {
                    "NEW" => OrderStatus::New,
                    "PARTIALLY_FILLED" => OrderStatus::PartiallyFilled,
                    "FILLED" => OrderStatus::Filled,
                    "CANCELED" | "CANCELLED" => OrderStatus::Cancelled,
                    "REJECTED" => OrderStatus::Rejected,
                    "EXPIRED" => OrderStatus::Expired,
                    _ => return None,
                };

                let avg_price = order
                    .get("price")
                    .and_then(|p| p.as_str())
                    .and_then(|p_str| Price::from_str(p_str).ok());
                let timestamp = order.get("time")?.as_u64()?;

                // Calculate filled_size and remaining_size
                let filled_size = executed_qty;
                let remaining_size = Size::new(orig_qty.value() - executed_qty.value());

                Some(ExecutionReport {
                    order_id,
                    client_order_id,
                    symbol: Symbol::new(symbol_str),
                    exchange_id: "binance".to_string(),
                    status,
                    filled_size,
                    remaining_size,
                    average_price: avg_price,
                    timestamp,
                })
            })
            .collect();

        Ok(orders)
    }
}

/// Binance WebSocket stream for market data
#[allow(dead_code)]
pub struct BinanceWebSocket {
    /// WebSocket connection
    ws_sender: Option<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    /// Subscribed symbols
    subscriptions: Arc<RwLock<Vec<String>>>,
    /// Connection status
    connected: Arc<RwLock<bool>>,
    /// Last update timestamps
    last_updates: Arc<RwLock<HashMap<String, u64>>>,
}

impl BinanceWebSocket {
    /// Create a new Binance WebSocket stream
    pub fn new() -> Self {
        Self {
            ws_sender: None,
            subscriptions: Arc::new(RwLock::new(Vec::new())),
            connected: Arc::new(RwLock::new(false)),
            last_updates: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Connect to the WebSocket stream
    pub async fn connect(&mut self, symbols: &[&str]) -> Result<(), BinanceError> {
        // Build stream URL for multiple symbols
        // Binance supports two formats:
        // 1. Single stream: wss://stream.binance.com:9443/ws/btcusdt@depth
        // 2. Multiple streams: wss://stream.binance.com:9443/stream?streams=btcusdt@depth/ethusdt@depth
        let streams: Vec<String> = symbols
            .iter()
            .map(|symbol| format!("{}@depth", symbol.to_lowercase()))
            .collect();

        let stream_url = if streams.len() == 1 {
            // Single stream format
            format!("wss://stream.binance.com:9443/ws/{}", streams[0])
        } else {
            // Multiple streams format
            format!(
                "wss://stream.binance.com:9443/stream?streams={}",
                streams.join("/")
            )
        };

        log::info!("Connecting to Binance WebSocket: {}", stream_url);

        let (ws_stream, _) = connect_async(&stream_url)
            .await
            .map_err(|e| BinanceError::ConnectionError(e.to_string()))?;

        self.ws_sender = Some(ws_stream);

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
    pub async fn disconnect(&mut self) -> Result<(), BinanceError> {
        if let Some(mut ws) = self.ws_sender.take() {
            ws.close(None)
                .await
                .map_err(|e| BinanceError::ConnectionError(e.to_string()))?;
        }

        // Update connection status
        let mut connected = self.connected.write().await;
        *connected = false;

        Ok(())
    }
}

#[async_trait]
impl MarketDataStream for BinanceWebSocket {
    type Error = BinanceError;

    async fn subscribe(&mut self, symbols: &[&str]) -> Result<(), Self::Error> {
        // For Binance, we need to reconnect with new subscriptions
        self.disconnect().await?;
        self.connect(symbols).await
    }

    async fn unsubscribe(&mut self, symbols: &[&str]) -> Result<(), Self::Error> {
        // For Binance, we need to reconnect with updated subscriptions
        // First, update subscriptions and collect the remaining symbols
        let remaining_symbols: Vec<String> = {
            let mut subs = self.subscriptions.write().await;
            subs.retain(|s| !symbols.contains(&s.as_str()));
            subs.clone()
        };
        // Now the lock is dropped, we can safely call methods on self

        if remaining_symbols.is_empty() {
            self.disconnect().await?;
        } else {
            let symbol_refs: Vec<&str> = remaining_symbols.iter().map(|s| s.as_str()).collect();
            self.disconnect().await?;
            self.connect(&symbol_refs).await?;
        }

        Ok(())
    }

    async fn next(&mut self) -> Option<Result<MarketEvent, Self::Error>> {
        if let Some(ws) = &mut self.ws_sender {
            match ws.next().await {
                Some(Ok(Message::Text(text))) => {
                    // Parse JSON message
                    match crate::connectors::BinanceMessage::from_json(&text) {
                        Ok(message) => {
                            // Convert to MarketEvent
                            Some(Ok(message.to_market_event()))
                        }
                        Err(e) => Some(Err(BinanceError::ParseError(e.to_string()))),
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
                    Some(Err(BinanceError::ConnectionError(e.to_string())))
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

    fn last_update(&self, _symbol: &str) -> Option<u64> {
        // This is a synchronous method, so we can't use async here
        // In a real implementation, you might use a different approach
        None // For simplicity, always return None
    }
}

/// Binance error types
#[derive(Debug, Clone)]
pub enum BinanceError {
    NetworkError(String),
    ConnectionError(String),
    ApiError(String),
    ParseError(String),
    AuthenticationError(String),
    RateLimitError(String),
}

impl std::fmt::Display for BinanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinanceError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            BinanceError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            BinanceError::ApiError(msg) => write!(f, "API error: {}", msg),
            BinanceError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            BinanceError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            BinanceError::RateLimitError(msg) => write!(f, "Rate limit error: {}", msg),
        }
    }
}

impl std::error::Error for BinanceError {}

/// Binance adapter that implements both MarketDataStream and ExecutionClient
pub struct BinanceAdapter {
    /// Binance client for REST API
    client: BinanceClient,
    /// Binance WebSocket for market data
    websocket: Arc<Mutex<BinanceWebSocket>>,
}

impl BinanceAdapter {
    /// Create a new Binance adapter
    pub fn new(api_key: String, api_secret: String, testnet: bool) -> Self {
        Self {
            client: BinanceClient::new(api_key, api_secret, testnet),
            websocket: Arc::new(Mutex::new(BinanceWebSocket::new())),
        }
    }
}

#[async_trait]
impl ExecutionClient for BinanceAdapter {
    type Error = BinanceError;

    async fn place_order(&self, order: NewOrder) -> Result<OrderId, Self::Error> {
        self.client.place_order(&order).await
    }

    async fn cancel_order(&self, _order_id: OrderId) -> Result<(), Self::Error> {
        // For Binance, we need to know the symbol to cancel an order
        // In a real implementation, we'd track this information
        // For now, we'll return an error
        Err(BinanceError::ApiError(
            "Symbol required to cancel order".to_string(),
        ))
    }

    async fn get_order_status(&self, _order_id: OrderId) -> Result<ExecutionReport, Self::Error> {
        // For Binance, we need to know the symbol to get order status
        // In a real implementation, we'd track this information
        // For now, we'll return an error
        Err(BinanceError::ApiError(
            "Symbol required to get order status".to_string(),
        ))
    }

    async fn get_balances(&self) -> Result<Vec<Balance>, Self::Error> {
        self.client.get_account_info().await
    }

    async fn get_open_orders(
        &self,
        symbol: Option<&str>,
    ) -> Result<Vec<ExecutionReport>, Self::Error> {
        self.client.get_open_orders(symbol).await
    }

    async fn get_order_history(
        &self,
        _symbol: Option<&str>,
        _limit: Option<usize>,
    ) -> Result<Vec<ExecutionReport>, Self::Error> {
        // For Binance, we'd use the /api/v3/allOrders endpoint
        // For now, we'll return an error
        Err(BinanceError::ApiError("Not implemented".to_string()))
    }

    async fn get_trading_fees(&self, symbol: &str) -> Result<TradingFees, Self::Error> {
        // For Binance, we'd use the /api/v3/myTrades endpoint to calculate fees
        // For now, we'll return default fees
        Ok(TradingFees::new(
            symbol.to_string(),
            Size::from_str("0.001").unwrap(), // 0.1% maker fee
            Size::from_str("0.001").unwrap(), // 0.1% taker fee
        ))
    }
}

#[async_trait]
impl MarketDataHistory for BinanceAdapter {
    type Error = BinanceError;

    async fn get_order_book_snapshots(
        &self,
        _symbol: &str,
        _start_time: u64,
        _end_time: u64,
    ) -> Result<Vec<OrderBookSnapshot>, Self::Error> {
        // Binance doesn't provide historical order book snapshots
        // In a real implementation, we'd store snapshots ourselves
        Err(BinanceError::ApiError(
            "Historical order book snapshots not available".to_string(),
        ))
    }

    async fn get_trades(
        &self,
        _symbol: &str,
        _start_time: u64,
        _end_time: u64,
    ) -> Result<Vec<Trade>, Self::Error> {
        // For Binance, we'd use the /api/v3/myTrades endpoint
        // For now, we'll return an error
        Err(BinanceError::ApiError("Not implemented".to_string()))
    }
}

#[async_trait]
impl crate::exchanges::connection_manager::ExchangeAdapter for BinanceAdapter {
    async fn connect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Binance connection is handled through WebSocket subscription
        // For now, just return Ok
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut ws = self.websocket.lock().await;
        ws.disconnect()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn get_market_data_stream(
        &self,
    ) -> Result<
        Arc<
            tokio::sync::Mutex<
                dyn MarketDataStream<Error = crate::exchanges::error::BoxedError> + Send + Sync,
            >,
        >,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        // Create the adapter
        let adapter = BinanceWebSocketAdapter {
            websocket: self.websocket.clone(),
        };

        // Wrap it in a type-erased wrapper
        // We need to convert ExchangeError to Box<dyn Error>
        // For now, we'll use a workaround by creating a wrapper that converts errors
        struct ErrorWrapper {
            inner: Arc<tokio::sync::Mutex<BinanceWebSocketAdapter>>,
        }

        #[async_trait]
        impl MarketDataStream for ErrorWrapper {
            type Error = crate::exchanges::error::BoxedError;

            async fn subscribe(&mut self, symbols: &[&str]) -> Result<(), Self::Error> {
                let mut inner = self.inner.lock().await;
                inner
                    .subscribe(symbols)
                    .await
                    .map_err(|e| crate::exchanges::error::BoxedError::new(e))
            }

            async fn unsubscribe(&mut self, symbols: &[&str]) -> Result<(), Self::Error> {
                let mut inner = self.inner.lock().await;
                inner
                    .unsubscribe(symbols)
                    .await
                    .map_err(|e| crate::exchanges::error::BoxedError::new(e))
            }

            async fn next(&mut self) -> Option<Result<MarketEvent, Self::Error>> {
                let mut inner = self.inner.lock().await;
                inner
                    .next()
                    .await
                    .map(|r| r.map_err(|e| crate::exchanges::error::BoxedError::new(e)))
            }

            fn is_connected(&self) -> bool {
                true
            }

            fn last_update(&self, _symbol: &str) -> Option<u64> {
                None
            }
        }

        Ok(Arc::new(tokio::sync::Mutex::new(ErrorWrapper {
            inner: Arc::new(tokio::sync::Mutex::new(adapter)),
        })))
    }

    async fn place_order(
        &self,
        order: NewOrder,
    ) -> Result<OrderId, Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .place_order(&order)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn cancel_order(
        &self,
        _order_id: OrderId,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // For Binance, we need the symbol - this is a limitation
        Err("Symbol required to cancel order".into())
    }

    async fn get_order_status(
        &self,
        _order_id: OrderId,
    ) -> Result<ExecutionReport, Box<dyn std::error::Error + Send + Sync>> {
        // For Binance, we need the symbol - this is a limitation
        Err("Symbol required to get order status".into())
    }

    async fn get_balances(&self) -> Result<Vec<Balance>, Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .get_account_info()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn get_open_orders(
        &self,
        symbol: Option<&str>,
    ) -> Result<Vec<ExecutionReport>, Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .get_open_orders(symbol)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn get_order_book(
        &self,
        symbol: &str,
        limit: u32,
    ) -> Result<OrderBookSnapshot, Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .get_order_book(symbol, limit)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn get_trading_fees(
        &self,
        symbol: &str,
    ) -> Result<TradingFees, Box<dyn std::error::Error + Send + Sync>> {
        Ok(TradingFees::new(
            symbol.to_string(),
            Size::from_str("0.001").unwrap(),
            Size::from_str("0.001").unwrap(),
        ))
    }
}

/// Wrapper for BinanceWebSocket to implement the required MarketDataStream trait
pub struct BinanceWebSocketAdapter {
    websocket: Arc<tokio::sync::Mutex<BinanceWebSocket>>,
}

#[async_trait]
impl MarketDataStream for BinanceWebSocketAdapter {
    type Error = crate::exchanges::error::ExchangeError;

    async fn subscribe(&mut self, symbols: &[&str]) -> Result<(), Self::Error> {
        let mut ws = self.websocket.lock().await;
        ws.subscribe(symbols)
            .await
            .map_err(|e| crate::exchanges::error::ExchangeError::new(e))
    }

    async fn unsubscribe(&mut self, symbols: &[&str]) -> Result<(), Self::Error> {
        let mut ws = self.websocket.lock().await;
        ws.unsubscribe(symbols)
            .await
            .map_err(|e| crate::exchanges::error::ExchangeError::new(e))
    }

    async fn next(&mut self) -> Option<Result<MarketEvent, Self::Error>> {
        let mut ws = self.websocket.lock().await;
        ws.next()
            .await
            .map(|r| r.map_err(|e| crate::exchanges::error::ExchangeError::new(e)))
    }

    fn is_connected(&self) -> bool {
        // This is a limitation - we can't easily check connection status synchronously
        true
    }

    fn last_update(&self, _symbol: &str) -> Option<u64> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binance_client_creation() {
        let client = BinanceClient::new(
            "test_key".to_string(),
            "test_secret".to_string(),
            true, // testnet
        );

        // Verify client was created with correct URLs
        assert_eq!(client.rest_url, "https://testnet.binance.vision");
        assert_eq!(client.ws_url, "wss://testnet.binance.vision/ws");
    }

    #[test]
    fn test_binance_websocket_creation() {
        let ws = BinanceWebSocket::new();

        // Verify WebSocket was created
        assert!(ws.ws_sender.is_none());
        assert!(ws.subscriptions.try_read().unwrap().is_empty());
    }

    #[test]
    fn test_binance_adapter_creation() {
        let _adapter = BinanceAdapter::new(
            "test_key".to_string(),
            "test_secret".to_string(),
            true, // testnet
        );

        // Verify adapter was created
        // We can't easily test the internal structure without exposing it
        // In a real test, we'd test the behavior
    }

    #[test]
    fn test_binance_error_display() {
        let error = BinanceError::NetworkError("Connection failed".to_string());
        assert_eq!(error.to_string(), "Network error: Connection failed");

        let error = BinanceError::ApiError("Invalid symbol".to_string());
        assert_eq!(error.to_string(), "API error: Invalid symbol");
    }
}
