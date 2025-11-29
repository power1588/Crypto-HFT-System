use crate::core::events::{OrderBookDelta, OrderBookLevel, OrderSide, Trade};
use crate::traits::MarketEvent;
use crate::types::{Price, Size, Symbol};
use serde::{Deserialize, Serialize};

/// Binance depth update message
#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthUpdateMessage {
    /// Event type
    pub e: String,
    /// Event time
    pub E: u64,
    /// Symbol
    pub s: String,
    /// First update ID in event
    pub U: u64,
    /// Final update ID in event
    pub u: u64,
    /// Bids to be updated
    #[serde(default, deserialize_with = "deserialize_price_size_pairs")]
    pub b: Vec<(Price, Size)>,
    /// Asks to be updated
    #[serde(default, deserialize_with = "deserialize_price_size_pairs")]
    pub a: Vec<(Price, Size)>,
}

/// Binance trade message
#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeMessage {
    /// Event type
    pub e: String,
    /// Event time
    pub E: u64,
    /// Symbol
    pub s: String,
    /// Trade ID
    pub t: u64,
    /// Price
    #[serde(deserialize_with = "deserialize_price")]
    pub p: Price,
    /// Quantity
    #[serde(deserialize_with = "deserialize_size")]
    pub q: Size,
    /// Buyer order ID
    pub b: u64,
    /// Seller order ID
    pub a: u64,
    /// Trade time
    pub T: u64,
    /// Is buyer market maker?
    pub m: bool,
}

/// Binance WebSocket message types
#[derive(Debug, Clone)]
pub enum BinanceMessage {
    DepthUpdate(DepthUpdateMessage),
    Trade(TradeMessage),
}

impl BinanceMessage {
    /// Parse a JSON string into a BinanceMessage using SIMD-accelerated parsing
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        Self::from_json_simd(json)
    }

    /// Parse using SIMD-accelerated JSON parsing (faster for large messages)
    pub fn from_json_simd(json: &str) -> Result<Self, serde_json::Error> {
        // Use simd-json for better performance on large messages
        // Fall back to serde_json if simd-json fails
        let mut json_bytes = json.as_bytes().to_vec();

        // First, determine the message type using standard serde_json for simplicity
        // Then use simd-json for the actual parsing
        let value: serde_json::Value = serde_json::from_str(json)?;

        if let Some(event_type) = value.get("e").and_then(|v| v.as_str()) {
            match event_type {
                "depthUpdate" => {
                    // Try SIMD parsing first
                    match simd_json::from_slice::<DepthUpdateMessage>(&mut json_bytes) {
                        Ok(msg) => Ok(BinanceMessage::DepthUpdate(msg)),
                        Err(_) => {
                            // Fall back to standard parsing
                            let msg: DepthUpdateMessage = serde_json::from_value(value)?;
                            Ok(BinanceMessage::DepthUpdate(msg))
                        }
                    }
                }
                "trade" => {
                    // Try SIMD parsing first
                    match simd_json::from_slice::<TradeMessage>(&mut json_bytes) {
                        Ok(msg) => Ok(BinanceMessage::Trade(msg)),
                        Err(_) => {
                            // Fall back to standard parsing
                            let msg: TradeMessage = serde_json::from_value(value)?;
                            Ok(BinanceMessage::Trade(msg))
                        }
                    }
                }
                _ => {
                    // Fall back to standard parsing for unknown types
                    Self::from_json_fallback(json)
                }
            }
        } else {
            // Fall back if event type not found
            Self::from_json_fallback(json)
        }
    }

    /// Fallback to standard serde_json parsing
    fn from_json_fallback(json: &str) -> Result<Self, serde_json::Error> {
        // First parse to a generic JSON value to determine the message type
        let value: serde_json::Value = serde_json::from_str(json)?;

        if let Some(event_type) = value.get("e").and_then(|v| v.as_str()) {
            match event_type {
                "depthUpdate" => {
                    let msg: DepthUpdateMessage = serde_json::from_value(value)?;
                    Ok(BinanceMessage::DepthUpdate(msg))
                }
                "trade" => {
                    let msg: TradeMessage = serde_json::from_value(value)?;
                    Ok(BinanceMessage::Trade(msg))
                }
                _ => Err(serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Unknown event type: {}", event_type),
                ))),
            }
        } else {
            Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Missing event type field 'e'",
            )))
        }
    }

    /// Convert to a MarketEvent
    pub fn to_market_event(self) -> MarketEvent {
        match self {
            BinanceMessage::DepthUpdate(msg) => {
                let bids = msg
                    .b
                    .into_iter()
                    .map(|(price, size)| OrderBookLevel::new(price, size))
                    .collect();

                let asks = msg
                    .a
                    .into_iter()
                    .map(|(price, size)| OrderBookLevel::new(price, size))
                    .collect();

                let delta = OrderBookDelta::new(msg.s, "binance", bids, asks, msg.E);
                MarketEvent::OrderBookDelta(delta)
            }
            BinanceMessage::Trade(msg) => {
                // Convert is_buyer_maker to OrderSide
                // If buyer is maker (passive), the trade is a sell (seller is taker/aggressor)
                // If buyer is taker (aggressive), the trade is a buy
                let side = if msg.m {
                    OrderSide::Sell
                } else {
                    OrderSide::Buy
                };

                let trade = Trade {
                    symbol: Symbol::new(msg.s),
                    exchange_id: "binance".to_string(),
                    price: msg.p,
                    size: msg.q,
                    side,
                    timestamp: msg.E,
                    trade_id: None,
                };

                MarketEvent::Trade(trade)
            }
        }
    }
}

/// Custom deserializer for price-size pairs
fn deserialize_price_size_pairs<'de, D>(deserializer: D) -> Result<Vec<(Price, Size)>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Pair(
        #[serde(deserialize_with = "deserialize_price")] Price,
        #[serde(deserialize_with = "deserialize_size")] Size,
    );

    let pairs: Vec<Pair> = Vec::deserialize(deserializer)?;
    Ok(pairs
        .into_iter()
        .map(|Pair(price, size)| (price, size))
        .collect())
}

/// Custom deserializer for Price
fn deserialize_price<'de, D>(deserializer: D) -> Result<Price, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    let s: String = Deserialize::deserialize(deserializer)?;
    Price::from_str(&s).map_err(serde::de::Error::custom)
}

/// Custom deserializer for Size
fn deserialize_size<'de, D>(deserializer: D) -> Result<Size, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    let s: String = Deserialize::deserialize(deserializer)?;
    Size::from_str(&s).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_depth_update() {
        let json = r#"{
            "e": "depthUpdate",
            "E": 1672515782136,
            "s": "BNBBTC",
            "U": 157,
            "u": 160,
            "b": [
                ["0.0024", "10"],
                ["0.0023", "100"]
            ],
            "a": [
                ["0.0026", "100"],
                ["0.0027", "10"]
            ]
        }"#;

        let message = BinanceMessage::from_json(json).unwrap();

        match message {
            BinanceMessage::DepthUpdate(msg) => {
                assert_eq!(msg.e, "depthUpdate");
                assert_eq!(msg.s, "BNBBTC");
                assert_eq!(msg.U, 157);
                assert_eq!(msg.u, 160);
                assert_eq!(msg.b.len(), 2);
                assert_eq!(msg.a.len(), 2);

                // Check first bid
                assert_eq!(msg.b[0].0, Price::from_str("0.0024").unwrap());
                assert_eq!(msg.b[0].1, Size::from_str("10").unwrap());

                // Check first ask
                assert_eq!(msg.a[0].0, Price::from_str("0.0026").unwrap());
                assert_eq!(msg.a[0].1, Size::from_str("100").unwrap());
            }
            _ => panic!("Expected DepthUpdate message"),
        }
    }

    #[test]
    fn test_parse_trade() {
        let json = r#"{
            "e": "trade",
            "E": 1672515782136,
            "s": "BNBBTC",
            "t": 12345,
            "p": "0.001",
            "q": "100",
            "b": 88,
            "a": 50,
            "T": 1672515782136,
            "m": true,
            "M": true
        }"#;

        let message = BinanceMessage::from_json(json).unwrap();

        match message {
            BinanceMessage::Trade(msg) => {
                assert_eq!(msg.e, "trade");
                assert_eq!(msg.s, "BNBBTC");
                assert_eq!(msg.t, 12345);
                assert_eq!(msg.p, Price::from_str("0.001").unwrap());
                assert_eq!(msg.q, Size::from_str("100").unwrap());
                assert_eq!(msg.m, true);
            }
            _ => panic!("Expected Trade message"),
        }
    }

    #[test]
    fn test_to_market_event() {
        let json = r#"{
            "e": "depthUpdate",
            "E": 1672515782136,
            "s": "BNBBTC",
            "U": 157,
            "u": 160,
            "b": [
                ["0.0024", "10"]
            ],
            "a": [
                ["0.0026", "100"]
            ]
        }"#;

        let message = BinanceMessage::from_json(json).unwrap();
        let event = message.to_market_event();

        match event {
            MarketEvent::OrderBookDelta(delta) => {
                assert_eq!(delta.symbol.as_str(), "BNBBTC");
                assert_eq!(delta.timestamp, 1672515782136);
                assert_eq!(delta.bids.len(), 1);
                assert_eq!(delta.asks.len(), 1);

                assert_eq!(delta.bids[0].price, Price::from_str("0.0024").unwrap());
                assert_eq!(delta.bids[0].size, Size::from_str("10").unwrap());

                assert_eq!(delta.asks[0].price, Price::from_str("0.0026").unwrap());
                assert_eq!(delta.asks[0].size, Size::from_str("100").unwrap());
            }
            _ => panic!("Expected OrderBookDelta event"),
        }
    }
}
