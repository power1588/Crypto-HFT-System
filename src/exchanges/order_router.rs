use crate::traits::{NewOrder, OrderId, ExecutionReport, OrderStatus, OrderSide, OrderType, TimeInForce};
use crate::types::{Price, Size};
use std::collections::HashMap;

/// Order routing configuration
#[derive(Debug, Clone)]
pub struct OrderRoutingConfig {
    /// Exchange-specific configurations
    pub exchanges: HashMap<String, ExchangeConfig>,
}

/// Exchange configuration
#[derive(Debug, Clone)]
pub struct ExchangeConfig {
    /// Exchange name
    pub name: String,
    /// Base URL for REST API
    pub base_url: String,
    /// WebSocket URL
    pub ws_url: String,
    /// API key
    pub api_key: Option<String>,
    /// Rate limits
    pub rate_limits: HashMap<String, u32>,
}

/// Order router that routes orders to appropriate exchanges
pub struct OrderRouter {
    /// Routing configuration
    config: OrderRoutingConfig,
    /// Exchange configurations
    exchanges: HashMap<String, ExchangeConfig>,
}

impl OrderRouter {
    /// Create a new order router
    pub fn new(config: OrderRoutingConfig) -> Self {
        Self {
            config,
            exchanges: HashMap::new(),
        }
    }

    /// Add an exchange configuration
    pub fn add_exchange(&mut self, name: String, config: ExchangeConfig) {
        self.exchanges.insert(name, config);
    }

    /// Get exchange configuration
    pub fn get_exchange(&self, name: &str) -> Option<&ExchangeConfig> {
        self.exchanges.get(name)
    }

    /// Route an order to the appropriate exchange
    pub fn route_order(&mut self, order: &NewOrder) -> Result<String, Box<dyn std::error::Error>> {
        // For this example, we'll route based on symbol prefix
        let exchange_name = if order.symbol.starts_with("BTC") {
            "binance"
        } else if order.symbol.starts_with("ETH") {
            "coinbase"
        } else {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Unsupported symbol: {}", order.symbol)
            )));
        };
        
        // Get exchange configuration
        let exchange_config = self.get_exchange(exchange_name)
            .ok_or_else(|| {
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Exchange not configured: {}", exchange_name)
                )))
            })?;
        
        // Route to exchange
        Ok(format!("{}:{}", exchange_config.base_url))
    }

    /// Get all exchange configurations
    pub fn get_all_exchanges(&self) -> &HashMap<String, ExchangeConfig> {
        &self.exchanges
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Price, Size};

    #[test]
    fn test_order_router() {
        let mut router = OrderRouter::new(OrderRoutingConfig {
            exchanges: HashMap::new(),
        });
        
        // Add exchanges
        router.add_exchange(
            "binance".to_string(),
            ExchangeConfig {
                name: "binance".to_string(),
                base_url: "https://api.binance.com".to_string(),
                ws_url: "wss://stream.binance.com:9443/ws".to_string(),
                api_key: Some("test_key".to_string()),
                rate_limits: HashMap::from([
                    ("BTCUSDT".to_string(), 100),
                    ("ETHUSDT".to_string(), 50),
                ]),
            }
        );
        
        router.add_exchange(
            "coinbase".to_string(),
            ExchangeConfig {
                name: "coinbase".to_string(),
                base_url: "https://api.coinbase.com".to_string(),
                ws_url: "wss://ws-feed.exchange.coinbase.com".to_string(),
                api_key: Some("test_key".to_string()),
                rate_limits: HashMap::from([
                    ("BTCUSDT".to_string(), 200),
                    ("ETHUSDT".to_string(), 100),
                ]),
            }
        );
        
        // Test routing
        let btc_order = NewOrder::new_market_buy(
            "BTCUSDT".to_string(),
            Size::from_str("1.0").unwrap()
        );
        
        let eth_order = NewOrder::new_market_buy(
            "ETHUSDT".to_string(),
            Size::from_str("1.0").unwrap()
        );
        
        // Route BTC order to Binance
        let btc_route = router.route_order(&btc_order);
        assert_eq!(btc_route.unwrap(), "https://api.binance.com");
        
        // Route ETH order to Coinbase
        let eth_route = router.route_order(&eth_order);
        assert_eq!(eth_route.unwrap(), "https://api.coinbase.com");
        
        // Test unsupported symbol
        let unsupported_order = NewOrder::new_market_buy(
            "DOGEUSDT".to_string(),
            Size::from_str("1.0").unwrap()
        );
        
        let unsupported_route = router.route_order(&unsupported_order);
        assert!(unsupported_route.is_err());
        assert!(matches!(unsupported_route.unwrap_err(), std::io::Error::ErrorKind::InvalidInput));
        
        // Test missing exchange
        let missing_exchange_order = NewOrder::new_market_buy(
            "LTCUSDT".to_string(),
            Size::from_str("1.0").unwrap()
        );
        
        let missing_exchange_route = router.route_order(&missing_exchange_order);
        assert!(missing_exchange_route.is_err());
        assert!(matches!(missing_exchange_route.unwrap_err(), std::io::Error::ErrorKind::NotFound));
        
        // Test all exchanges
        let all_exchanges = router.get_all_exchanges();
        assert_eq!(all_exchanges.len(), 2);
        assert!(all_exchanges.contains_key("binance"));
        assert!(all_exchanges.contains_key("coinbase"));
    }
}
