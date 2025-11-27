use crate::traits::{MarketDataStream, MarketEvent, ExecutionClient, OrderManager};
use std::collections::HashMap;
use std::time::Instant;

/// Connection manager for multiple exchange adapters
pub struct ConnectionManager {
    /// Active connections by exchange
    connections: HashMap<String, ConnectionInfo>,
    /// Connection statistics
    stats: ConnectionStats,
}

/// Connection information
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Exchange name
    pub exchange: String,
    /// Connection type
    pub connection_type: ConnectionType,
    /// Connected at
    pub connected_at: Option<Instant>,
    /// Last activity at
    pub last_activity: Option<Instant>,
    /// Message count
    pub message_count: u64,
    /// Error count
    pub error_count: u64,
}

/// Connection type
#[derive(Debug, Clone)]
pub enum ConnectionType {
    WebSocket,
    Rest,
}

/// Connection statistics
#[derive(Debug, Clone, Default)]
pub struct ConnectionStats {
    /// Total connections
    pub total: u32,
    /// Active connections
    pub active: u32,
    /// Failed connections
    pub failed: u32,
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Average latency
    pub avg_latency_ms: f64,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            stats: ConnectionStats::default(),
        }
    }

    /// Add a connection
    pub fn add_connection(&mut self, exchange: String, connection_type: ConnectionType) -> Result<(), Box<dyn std::error::Error>> {
        let connection_id = format!("{}:{}", exchange);
        let connection_info = ConnectionInfo {
            exchange: exchange.clone(),
            connection_type,
            connected_at: Some(std::time::Instant::now()),
            last_activity: Some(std::time::Instant::now()),
            message_count: 0,
            error_count: 0,
        };
        
        self.connections.insert(connection_id, connection_info);
        self.stats.total += 1;
        self.stats.active += 1;
        
        Ok(())
    }

    /// Remove a connection
    pub fn remove_connection(&mut self, exchange: String) -> Result<(), Box<dyn std::error::Error>> {
        let connection_id = format!("{}:{}", exchange);
        
        if let Some(connection_info) = self.connections.remove(&connection_id) {
            self.stats.active -= 1;
            self.stats.failed += 1;
        }
        
        Ok(())
    }

    /// Update connection statistics
    pub fn update_stats(&mut self, exchange: &str, message_count: u64, error_count: u64, latency_ms: f64) {
        if let Some(connection_info) = self.connections.get_mut(&exchange) {
            connection_info.message_count += message_count;
            connection_info.error_count += error_count;
            
            // Update average latency
            let total_messages = connection_info.message_count + connection_info.error_count;
            if total_messages > 0 {
                connection_info.avg_latency_ms = (connection_info.avg_latency_ms * (total_messages - 1) + latency_ms) / total_messages;
            }
        }
    }

    /// Get connection statistics
    pub fn get_stats(&self) -> &ConnectionStats {
        &self.stats
    }

    /// Get connection info
    pub fn get_connection_info(&self, exchange: &str) -> Option<&ConnectionInfo> {
        self.connections.get(&exchange)
    }

    /// Get all connection info
    pub fn get_all_connections(&self) -> &HashMap<String, ConnectionInfo> {
        &self.connections
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_connection_manager() {
        let mut manager = ConnectionManager::new();
        
        // Add connections
        manager.add_connection("binance".to_string(), ConnectionType::WebSocket).unwrap();
        manager.add_connection("coinbase".to_string(), ConnectionType::Rest).unwrap();
        
        // Check connections
        assert_eq!(manager.get_stats().total, 2);
        assert_eq!(manager.get_stats().active, 2);
        
        // Get connection info
        let binance_info = manager.get_connection_info("binance").unwrap();
        assert_eq!(binance_info.exchange, "binance");
        assert_eq!(binance_info.connection_type, ConnectionType::WebSocket);
        
        let coinbase_info = manager.get_connection_info("coinbase").unwrap();
        assert_eq!(coinbase_info.exchange, "coinbase");
        assert_eq!(coinbase_info.connection_type, ConnectionType::Rest);
        
        // Remove connection
        manager.remove_connection("binance").unwrap();
        assert_eq!(manager.get_stats().total, 2);
        assert_eq!(manager.get_stats().active, 1);
        assert_eq!(manager.get_stats().failed, 1);
        
        // Check connection is removed
        assert!(manager.get_connection_info("binance").is_none());
        
        // Update stats
        manager.update_stats("binance".to_string(), 10, 0, 50.0).unwrap();
        manager.update_stats("coinbase".to_string(), 5, 0, 100.0).unwrap();
        
        // Check updated stats
        let stats = manager.get_stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.active, 2);
        assert_eq!(stats.messages_sent, 15);
        assert_eq!(stats.messages_received, 5);
        assert_eq!(stats.avg_latency_ms, 75.0);
    }
}
