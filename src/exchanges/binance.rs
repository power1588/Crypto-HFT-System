use crate::traits::{MarketDataStream, MarketEvent, ExecutionClient, OrderManager};
use crate::types::{Price, Size};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;
use futures_util::StreamExt;

/// High-performance Binance WebSocket adapter with connection pooling
pub struct HighPerformanceBinanceAdapter {
    /// Connection pool
    connection_pool: Vec<tokio_tungstenite::WebSocketStream<tokio_tungstenite::tungstenite::Message>>,
    /// Market data channels
    market_data_txs: HashMap<String, tokio::sync::mpsc::Sender<MarketEvent>>,
    /// Execution report channels
    execution_report_txs: HashMap<String, tokio::sync::mpsc::Sender<crate::traits::ExecutionReport>>,
    /// Connection count
    connection_count: std::sync::atomic::AtomicUsize,
}

impl HighPerformanceBinanceAdapter {
    /// Create a new high-performance adapter
    pub fn new() -> Self {
        Self {
            connection_pool: Vec::new(),
            market_data_txs: HashMap::new(),
            execution_report_txs: HashMap::new(),
            connection_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Add a market data channel
    pub fn add_market_data_channel(&mut self, symbol: String) -> tokio::sync::mpsc::Receiver<MarketEvent> {
        let (tx, rx) = tokio::sync::mpsc::channel(1000);
        self.market_data_txs.insert(symbol, tx);
        rx
    }

    /// Add an execution report channel
    pub fn add_execution_report_channel(&mut self, symbol: String) -> tokio::sync::mpsc::Receiver<crate::traits::ExecutionReport> {
        let (tx, rx) = tokio::sync::mpsc::channel(1000);
        self.execution_report_txs.insert(symbol, tx);
        rx
    }

    /// Connect to Binance WebSocket with connection pooling
    pub async fn connect(&mut self, symbols: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
        let connection_count = self.connection_count.fetch_add(1);
        
        // Create connections for each symbol
        let mut handles = Vec::new();
        for symbol in symbols {
            let (tx, rx) = tokio::sync::mpsc::channel(1000);
            self.market_data_txs.insert(symbol.clone(), tx.clone());
            self.execution_report_txs.insert(symbol.clone(), tx.clone());
            
            // Connect to Binance WebSocket
            let url = format!("wss://stream.binance.com:9443/ws/{}@1", symbol);
            let ws_stream = tokio_tungstenite::connect_async(&url).await?;
            
            // Spawn message handler
            let market_data_tx = self.market_data_txs.get(&symbol).unwrap();
            let execution_report_tx = self.execution_report_txs.get(&symbol).unwrap();
            
            let handle = tokio::spawn(async move {
                let mut ws_rx = ws_rx.resubscribe();
                
                while let Some(ws_result) = ws_rx.next().await {
                    match ws_result {
                        Ok(message) => {
                            // Parse market data message
                            if let Ok(market_event) = parse_binance_message(&message) {
                                // Send to all market data channels
                                for (symbol, tx) in &self.market_data_txs {
                                    if let Err(_) = tx.send(market_event).await {
                                        eprintln!("Failed to send market data to {}: {:?}", symbol);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("WebSocket error: {}", e);
                            break;
                        }
                    }
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all connections to be established
        for handle in handles {
            handle.await?;
        }
        
        // Update connection count
        self.connection_count.store(connection_count);
        
        Ok(())
    }

    /// Get market data receiver for a symbol
    pub fn get_market_data_receiver(&self, symbol: &str) -> Option<tokio::sync::mpsc::Receiver<MarketEvent>> {
        self.market_data_txs.get(symbol).map(|tx| tx.clone())
    }

    /// Get execution report receiver for a symbol
    pub fn get_execution_report_receiver(&self, symbol: &str) -> Option<tokio::sync::mpsc::Receiver<crate::traits::ExecutionReport>> {
        self.execution_report_txs.get(symbol).map(|tx| tx.clone())
    }

    /// Get current connection count
    pub fn get_connection_count(&self) -> usize {
        self.connection_count.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// Parse Binance WebSocket message
fn parse_binance_message(message: &str) -> Result<MarketEvent, Box<dyn std::error::Error>> {
    // This is a simplified parser for demonstration
    // In a real implementation, you would use a proper JSON parser
    if message.contains("\"depthUpdate\"") {
        parse_binance_depth_update(message)
    } else if message.contains("\"trade\"") {
        parse_binance_trade(message)
    } else {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Unknown message type: {}", message)
        )))
    }
}

/// Parse Binance depth update message
fn parse_binance_depth_update(message: &str) -> Result<MarketEvent, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = message.split(",").collect();
    
    if parts.len() < 11 {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid depth update message: {}", message)
        )));
    }
    
    // Extract event type
    let event_type = parts.get(0).and_then(|s| s.trim()).unwrap_or("");
    
    if event_type != "depthUpdate" {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid event type: {}", event_type)
        )));
    }
    
    // Extract symbol
    let symbol = parts.get(1).and_then(|s| s.trim()).unwrap_or("");
    
    // Extract update ID
    let update_id = parts.get(2).and_then(|s| s.trim()).unwrap_or("0").parse::<u64>().unwrap_or(0);
    
    // Extract first update ID
    let first_update_id = parts.get(3).and_then(|s| s.trim()).unwrap_or("0").parse::<u64>().unwrap_or(0);
    
    // Extract last update ID
    let last_update_id = parts.get(4).and_then(|s| s.trim()).unwrap_or("0").parse::<u64>().unwrap_or(0);
    
    // Extract bids and asks
    let mut bids = Vec::new();
    let mut asks = Vec::new();
    
    // Parse price levels (starting from index 5)
    for i in (5..parts.len()).step_by(2) {
        if i + 1 >= parts.len() {
            break;
        }
        
        let price_level = parts.get(i).and_then(|s| s.trim()).unwrap_or("");
        let mut price_levels = Vec::new();
        
        for j in (0..price_level.len()).step_by(2) {
            if j + 2 >= price_level.len() {
                break;
            }
            
            let price = price_level.get(j).and_then(|s| s.trim()).unwrap_or("");
            let size = price_level.get(j + 1).and_then(|s| s.trim()).unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            
            if price > 0.0 && size > 0.0 {
                price_levels.push((crate::types::Price::from_str(price).unwrap(), crate::types::Size::from_str(size).unwrap()));
            }
        }
        
        // Parse asks (starting from index 5 + price_levels.len())
        for i in (5..parts.len()).step_by(2) {
            if i + 1 >= parts.len() {
                break;
            }
            
            let ask_level = parts.get(i).and_then(|s| s.trim()).unwrap_or("");
            let mut ask_levels = Vec::new();
            
            for j in (0..ask_level.len()).step_by(2) {
                if j + 1 >= ask_level.len() {
                    break;
                }
                
                let ask_price = ask_level.get(j).and_then(|s| s.trim()).unwrap_or("");
                let ask_size = ask_level.get(j + 1).and_then(|s| s.trim()).unwrap_or("0").parse::<f64>().unwrap_or(0.0);
                
                if ask_price > 0.0 && ask_size > 0.0 {
                    ask_levels.push((crate::types::Price::from_str(ask_price).unwrap(), crate::types::Size::from_str(ask_size).unwrap()));
                }
            }
        }
        
        // Create order book snapshot
        let snapshot = crate::orderbook::OrderBookSnapshot::new(
            symbol.to_string(),
            bids,
            asks,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .as_millis() as u64,
        );
    
    Ok(MarketEvent::OrderBookSnapshot(snapshot))
}

/// Parse Binance trade message
fn parse_binance_trade(message: &str) -> Result<MarketEvent, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = message.split(",").collect();
    
    if parts.len() < 6 {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid trade message: {}", message)
        )));
    }
    
    // Extract event type
    let event_type = parts.get(0).and_then(|s| s.trim()).unwrap_or("");
    
    if event_type != "trade" {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid event type: {}", event_type)
        )));
    }
    
    // Extract symbol
    let symbol = parts.get(1).and_then(|s| s.trim()).unwrap_or("");
    
    // Extract trade ID
    let trade_id = parts.get(2).and_then(|s| s.trim()).unwrap_or("0").parse::<u64>().unwrap_or(0);
    
    // Extract price
    let price = parts.get(3).and_then(|s| s.trim()).unwrap_or("");
    let price = crate::types::Price::from_str(price).unwrap();
    
    // Extract quantity
    let quantity = parts.get(4).and_then(|s| s.trim()).unwrap_or("");
    let quantity = crate::types::Size::from_str(quantity).unwrap();
    
    // Extract timestamp
    let timestamp = parts.get(5).and_then(|s| s.trim()).unwrap_or("0").parse::<u64>().unwrap_or(0);
    
    // Extract buyer is maker
    let is_buyer_maker = parts.get(5).and_then(|s| s.trim()).unwrap_or("false");
    let is_buyer_maker = is_buyer_maker == "true";
    
    // Create trade event
    let trade = crate::traits::Trade {
        symbol: symbol.to_string(),
        price,
        quantity,
        timestamp,
        is_buyer_maker,
    };
    
    Ok(MarketEvent::Trade { trade })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Price, Size};

    #[test]
    fn test_high_performance_adapter() {
        let mut adapter = HighPerformanceBinanceAdapter::new();
        
        // Add market data channel
        let btc_market_rx = adapter.add_market_data_channel("BTCUSDT".to_string());
        let btc_execution_rx = adapter.add_execution_report_channel("BTCUSDT".to_string());
        
        // Add ETH channels
        let eth_market_rx = adapter.add_market_data_channel("ETHUSDT".to_string());
        let eth_execution_rx = adapter.add_execution_report_channel("ETHUSDT".to_string());
        
        // Test connection
        let symbols = vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()];
        adapter.connect(&mut adapter, symbols).await.unwrap();
        
        // Check connection count
        assert_eq!(adapter.get_connection_count(), 2);
        
        // Test market data reception
        let btc_market_event = MarketEvent::OrderBookSnapshot(crate::orderbook::OrderBookSnapshot::new(
            "BTCUSDT".to_string(),
            vec![
                crate::orderbook::OrderBookLevel::new(
                    crate::types::Price::from_str("100.0").unwrap(),
                    crate::types::Size::from_str("10.0").unwrap()
                )
            ],
            vec![
                crate::orderbook::OrderBookLevel::new(
                    crate::types::Price::from_str("101.0").unwrap(),
                    crate::types::Size::from_str("10.0").unwrap()
                )
            ],
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .as_millis() as u64,
        ));
        
        if let Ok(event) = btc_market_rx.try_recv() {
            assert_eq!(event, MarketEvent::OrderBookSnapshot(snapshot));
        }
        
        // Test execution report reception
        let btc_execution_report = crate::traits::ExecutionReport {
            order_id: crate::traits::OrderId::new("test_order".to_string()),
            client_order_id: Some("client_test".to_string()),
            symbol: "BTCUSDT".to_string(),
            status: crate::traits::OrderStatus::Filled,
            side: crate::traits::OrderSide::Buy,
            order_type: crate::traits::OrderType::Market,
            time_in_force: crate::traits::TimeInForce::GTC,
            quantity: crate::types::Size::from_str("1.0").unwrap(),
            price: Some(crate::types::Price::from_str("50000.0").unwrap()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .as_millis() as u64,
        };
        
        if let Ok(report) = btc_execution_rx.try_recv() {
            assert_eq!(report.order_id, btc_execution_report.order_id);
            assert_eq!(report.status, btc_execution_report.status);
        }
    }
}
