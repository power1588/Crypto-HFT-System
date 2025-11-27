use crate::orderbook::{OrderBookLevel, OrderBookDelta};
use crate::types::{Price, Size};
use crate::traits::MarketEvent;
use serde::{Deserialize, Serialize};

/// Binance depth update message
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
    /// Parse a JSON string into a BinanceMessage
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
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
                    format!("Unknown event type: {}", event_type)
                )))
            }
        } else {
            Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Missing event type field 'e'"
            )))
        }
    }

    /// Convert to a MarketEvent
    pub fn to_market_event(self) -> MarketEvent {
        match self {
            BinanceMessage::DepthUpdate(msg) => {
                let bids = msg.b.into_iter()
                    .map(|(price, size)| OrderBookLevel::new(price, size))
                    .collect();
                
                let asks = msg.a.into_iter()
                    .map(|(price, size)| OrderBookLevel::new(price, size))
                    .collect();
                
                let delta = OrderBookDelta::new(msg.s, bids, asks, msg.E);
                MarketEvent::OrderBookDelta(delta)
            }
            BinanceMessage::Trade(msg) => {
                MarketEvent::Trade {
                    symbol: msg.s,
                    price: msg.p,
                    size: msg.q,
                    timestamp: msg.E,
                    is_buyer_maker: msg.m,
                }
            }
        }
    }
}

/// Custom deserializer for price-size pairs
fn deserialize_price_size_pairs<'de, D>(
    deserializer: D,
) -> Result<Vec<(Price, Size)>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    
    #[derive(Deserialize)]
    struct Pair(#[serde(deserialize_with = "deserialize_price")] Price, #[serde(deserialize_with = "deserialize_size")] Size);
    
    let pairs: Vec<Pair> = Vec::deserialize(deserializer)?;
    Ok(pairs.into_iter().map(|Pair(price, size)| (price, size)).collect())
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
                assert_eq!(delta.symbol, "BNBBTC");
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
