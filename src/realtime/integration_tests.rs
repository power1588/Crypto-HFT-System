use crate::realtime::{EventLoop, SignalGenerator, OrderExecutor, RiskManager};
use crate::traits::{MarketEvent, NewOrder, OrderId, ExecutionReport, OrderStatus, OrderSide, OrderType, TimeInForce};
use crate::types::{Price, Size};
use std::time::Duration;

/// Integration tests for real-time trading system
pub struct IntegrationTests {
    event_loop: EventLoop,
    signal_generator: SignalGenerator,
    order_executor: OrderExecutor,
    risk_manager: RiskManager,
}

impl IntegrationTests {
    /// Create new integration tests
    pub fn new() -> Self {
        Self {
            event_loop: EventLoop::new(),
            signal_generator: SignalGenerator::new(),
            order_executor: OrderExecutor::new(),
            risk_manager: RiskManager::new(),
        }
    }

    /// Test end-to-end real-time trading workflow
    pub async fn test_complete_workflow(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Setup components
        self.event_loop.add_event_processor(Box::new(SimpleEventProcessor::new()));
        self.signal_generator.add_strategy("BTCUSDT".to_string(), Box::new(Moving_average::MovingAverageSignalStrategy::new("BTCUSDT".to_string(), 10)));
        self.order_executor.add_execution_client("binance".to_string(), Box::new(MockExecutionClient::new()));
        self.risk_manager.add_rule(Box::new(PositionSizeLimitRule::new("BTCUSDT".to_string(), Size::from_str("10.0").unwrap())));
        
        // Start event loop
        self.event_loop.start().await?;
        
        // Simulate market data
        let market_event = MarketEvent::OrderBookSnapshot(crate::orderbook::OrderBookSnapshot::new(
            "BTCUSDT".to_string(),
            vec![
                crate::orderbook::OrderBookLevel::new(
                    Price::from_str("100.0").unwrap(),
                    Size::from_str("10.0").unwrap()
                )
            ],
            vec![
                crate::orderbook::OrderBookLevel::new(
                    Price::from_str("101.0").unwrap(),
                    Size::from_str("10.0").unwrap()
                )
            ],
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .as_millis() as u64,
        ));
        
        // Process event
        let signal = self.signal_generator.generate_signal(&self.event_loop.market_states.get("BTCUSDT").unwrap()).await?;
        
        // Check if signal was generated
        assert!(signal.is_some());
        
        // Simulate order submission
        let order = NewOrder::new_market_buy(
            "BTCUSDT".to_string(),
            Size::from_str("1.0").unwrap()
        );
        
        // Check if order passes risk check
        let risk_violation = self.risk_manager.check_order(&order);
        assert!(risk_violation.is_none());
        
        // Submit order
        let order_id = self.order_executor.submit_order(&order).await?;
        
        // Check order status
        let order_status = self.order_executor.get_order_status(&order_id).await?;
        assert_eq!(order_status, Some(OrderStatus::New));
        
        // Wait for order execution
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Check execution report
        let execution_report = self.order_executor.get_execution_report(&order_id).await?;
        assert!(execution_report.is_some());
        assert_eq!(execution_report.unwrap().order_id, order_id);
        assert_eq!(execution_report.unwrap().status, OrderStatus::Filled);
        
        Ok(())
    }

    /// Test high-frequency trading performance
    pub async fn test_high_frequency_trading(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Setup for high-frequency trading
        self.event_loop.add_event_processor(Box::new(SimpleEventProcessor::new()));
        self.signal_generator.add_strategy("BTCUSDT".to_string(), Box::new(moving_average::MovingAverageSignalStrategy::new("BTCUSDT".to_string(), 10)));
        self.order_executor.add_execution_client("binance".to_string(), Box::new(MockExecutionClient::new()));
        self.risk_manager.add_rule(Box::new(PositionSizeLimitRule::new("BTCUSDT".to_string(), Size::from_str("5.0").unwrap())));
        
        // Start event loop
        self.event_loop.start().await?;
        
        // Simulate high-frequency market data
        for i in 0..1000 {
            let market_event = MarketEvent::OrderBookSnapshot(crate::orderbook::OrderBookSnapshot::new(
                "BTCUSDT".to_string(),
                vec![
                    crate::orderbook::OrderBookLevel::new(
                        Price::from_str("100.0").unwrap(),
                        Size::from_str("10.0").unwrap()
                    )
                ],
                vec![
                    crate::orderbook::OrderBookLevel::new(
                        Price::from_str("101.0").unwrap(),
                        Size::from_str("10.0").unwrap()
                    )
                ],
                std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .as_millis() as u64,
            ));
            
            // Process event
            let _ = self.event_loop.process_event(market_event).await?;
        }
        
        // Check performance metrics
        let metrics = self.event_loop.get_metrics();
        assert!(metrics.events_processed, 1000);
        assert!(metrics.avg_processing_time_ms > 0.0);
        assert!(metrics.peak_processing_time_ms > 0.0);
        
        Ok(())
    }

    /// Test error handling and recovery
    pub async fn test_error_handling(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Setup error handling
        self.event_loop.add_event_processor(Box::new(SimpleEventProcessor::new()));
        self.signal_generator.add_strategy("BTCUSDT".to_string(), Box::new(moving_average::MovingAverageSignalStrategy::new("BTCUSDT".to_string(), 10)));
        self.order_executor.add_execution_client("binance".to_string(), Box::new(MockExecutionClient::new()));
        self.risk_manager.add_rule(Box::new(PositionSizeLimitRule::new("BTCUSDT".to_string(), Size::from_str("5.0").unwrap())));
        
        // Start event loop
        self.event_loop.start().await?;
        
        // Simulate order that violates risk rule
        let large_order = NewOrder::new_market_buy(
            "BTCUSDT".to_string(),
            Size::from_str("10.0").unwrap()
        );
        
        // Check if order is rejected
        let order_id = self.order_executor.submit_order(&large_order).await?;
        let order_status = self.order_executor.get_order_status(&order_id).await?;
        assert_eq!(order_status, Some(OrderStatus::Rejected));
        
        // Check risk violation
        let risk_violation = self.risk_manager.check_order(&large_order);
        assert!(risk_violation.is_some());
        
        Ok(())
    }

    /// Test system resilience under load
    pub async fn test_system_resilience(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Setup for resilience testing
        self.event_loop.add_event_processor(Box::new(SimpleEventProcessor::new()));
        self.signal_generator.add_strategy("BTCUSDT".to_string(), Box::new(moving_average::MovingAverageSignalStrategy::new("BTCUSDT".to_string(), 10)));
        self.order_executor.add_execution_client("binance".to_string(), Box::new(MockExecutionClient::new()));
        self.risk_manager.add_rule(Box::new(PositionSizeLimitRule::new("BTCUSDT".to_string(), Size::from_str("5.0").unwrap())));
        
        // Start event loop
        self.event_loop.start().await?;
        
        // Simulate high load
        for i in 0..10000 {
            let market_event = MarketEvent::OrderBookSnapshot(crate::orderbook::OrderBookSnapshot::new(
                "BTCUSDT".to_string(),
                vec![
                    crate::orderbook::OrderBookLevel::new(
                        Price::from_str("100.0").unwrap(),
                        Size::from_str("10.0").unwrap()
                    )
                ],
                vec![
                    crate::orderbook::OrderBookLevel::new(
                        Price::from_str("101.0").unwrap(),
                        Size::from_str("10.0").unwrap()
                    )
                ],
                std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .as_millis() as u64,
            ));
            
            // Process event
            let _ = self.event_loop.process_event(market_event).await?;
        }
        
        // Check performance metrics
        let metrics = self.event_loop.get_metrics();
        assert!(metrics.events_processed, 10000);
        assert!(metrics.avg_processing_time_ms > 0.0);
        assert!(metrics.peak_processing_time_ms > 0.0);
        
        Ok(())
    }
}

/// Simple event processor for testing
pub struct SimpleEventProcessor {
    processed_count: std::sync::atomic::AtomicUsize,
}

impl SimpleEventProcessor {
    /// Create a new simple event processor
    pub fn new() -> Self {
        Self {
            processed_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    impl crate::realtime::EventProcessor for SimpleEventProcessor {
        fn process_event(&mut self, event: MarketEvent) -> Result<(), Box<dyn std::error::Error>> {
            self.processed_count.fetch_add(1);
            
            match event {
                MarketEvent::OrderBookSnapshot(_) => {
                    // Process order book snapshot
                    eprintln!("Processed order book snapshot for symbol: {}", event.get_symbol().unwrap_or("unknown"));
                }
                MarketEvent::Trade(trade) => {
                    // Process trade
                    eprintln!("Processed trade: {} @ {} for {} units (ts: {})", 
                        trade.symbol,
                        trade.price,
                        trade.size,
                        trade.timestamp
                    );
                }
                _ => {
                    eprintln!("Unknown event type: {:?}", event);
                }
            }
            
            Ok(())
        }
    }
}

/// Mock execution client for testing
pub struct MockExecutionClient {
    orders: Vec<ExecutionReport>,
}

impl MockExecutionClient {
    /// Create a new mock execution client
    pub fn new() -> Self {
        Self {
            orders: Vec::new(),
        }
    }

    impl crate::traits::ExecutionClient for MockExecutionClient {
        async fn submit_order(&mut self, order: NewOrder) -> Result<OrderId, Box<dyn std::error::Error>> {
            let order_id = OrderId::new(format!("order_{}", self.orders.len()));
            let execution_report = ExecutionReport {
                order_id: order_id.clone(),
                client_order_id: Some(order.client_order_id.clone()),
                symbol: order.symbol.clone(),
                status: OrderStatus::New,
                side: order.side,
                order_type: order.order_type,
                time_in_force: order.time_in_force,
                quantity: order.quantity,
                price: order.price,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .as_millis() as u64,
            };
            
            self.orders.push(execution_report);
            Ok(order_id)
        }

        async fn get_order(&self, order_id: OrderId) -> Result<ExecutionReport, Box<dyn std::error::Error>> {
            self.orders
                .iter()
                .find(|order| order.order_id == order_id)
                .cloned()
                .ok_or_else(|| Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Order not found: {}", order_id.as_str())
                )))
        }

        async fn get_all_orders(&self) -> Result<Vec<ExecutionReport>, Box<dyn std::error::Error>> {
            Ok(self.orders.clone())
        }

        async fn get_orders_by_symbol(&self, symbol: &str) -> Result<Vec<ExecutionReport>, Box<dyn std::error::Error>> {
            Ok(self
                .orders
                .iter()
                .filter(|order| order.symbol == symbol)
                .cloned()
                .collect()
        }

        async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            Ok(())
        }

        async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            Ok(())
        }
    }
}

/// Mock order manager for testing
pub struct MockOrderManager {
    next_order_id: u64,
    orders: Vec<ExecutionReport>,
}

impl MockOrderManager {
    /// Create a new mock order manager
    pub fn new() -> Self {
        Self {
            next_order_id: 1,
            orders: Vec::new(),
        }
    }

    impl crate::traits::OrderManager for MockOrderManager {
        async fn generate_order_id(&mut self, _order: &NewOrder) -> OrderId {
            let order_id = OrderId::new(format!("order_{}", self.next_order_id));
            self.next_order_id += 1;
            order_id
        }

        async fn handle_execution_report(&mut self, report: ExecutionReport) -> Result<(), Box<dyn std::error::Error>> {
            self.orders.push(report);
            Ok(())
        }

        async fn get_order(&self, order_id: OrderId) -> Result<ExecutionReport, Box<dyn std::error::Error>> {
            self.orders
                .iter()
                .find(|order| order.order_id == order_id)
                .cloned()
                .ok_or_else(|| Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Order not found: {}", order_id.as_str())
                )))
        }

        async fn get_all_orders(&self) -> Result<Vec<ExecutionReport>, Box<dyn std::error::Error>> {
            Ok(self.orders.clone())
        }

        async fn get_orders_by_symbol(&self, symbol: &str) -> Result<Vec<ExecutionReport>, Box<dyn std::error::Error>> {
            Ok(self
                .orders
                .iter()
                .filter(|order| order.symbol == symbol)
                .cloned()
                .collect()
        }

        async fn get_order_status(&self, order_id: OrderId) -> Option<OrderStatus> {
            self.orders
                .iter()
                .find(|order| order.order_id == order_id)
                .map(|order| order.status.clone())
                .ok_or_else(|| OrderStatus::New)
        }

        async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            Ok(())
        }

        async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            Ok(())
        }
    }
}

/// Mock signal generator for testing
pub struct MockSignalGenerator {
    strategies: HashMap<String, Box<dyn crate::realtime::SignalStrategy>>,
}

impl MockSignalGenerator {
    /// Create a new mock signal generator
    pub fn new() -> Self {
        Self {
            strategies: HashMap::new(),
        }
    }

    /// Add a signal strategy
    pub fn add_strategy(&mut self, symbol: String, strategy: Box<dyn crate::realtime::SignalStrategy>) {
        self.strategies.insert(symbol, strategy);
    }

    /// Generate a signal based on market state
    pub fn generate_signal(&mut self, market_state: &crate::realtime::MarketState) -> Option<crate::realtime::Signal> {
        if let Some(strategy) = self.strategies.get(&market_state.symbol) {
            strategy.generate_signal(market_state)
        } else {
            None
        }
    }
}

/// Mock moving average signal strategy for testing
pub struct MockMovingAverageSignalStrategy {
    symbol: String,
    price_history: Vec<Price>,
    window_size: usize,
}

impl MockMovingAverageSignalStrategy {
    /// Create a new mock moving average signal strategy
    pub fn new(symbol: String, window_size: usize) -> Self {
        Self {
            symbol,
            price_history: Vec::new(),
            window_size,
        }
    }

    /// Add a price to history
    pub fn add_price(&mut self, price: Price) {
        self.price_history.push(price);
        
        // Keep only recent prices
        if self.price_history.len() > self.window_size {
            self.price_history.remove(0);
        }
    }

    /// Generate a signal based on moving average
    impl crate::realtime::SignalStrategy for MockMovingAverageSignalStrategy {
        fn name(&self) -> &str {
            "MovingAverage"
        }

        fn generate_signal(&mut self, market_state: &crate::realtime::MarketState) -> Option<crate::realtime::Signal> {
            // Get current price
            let current_price = market_state.best_bid()
                .map(|(bid_price, _)| bid_price)
                .or_else(|| market_state.best_ask()
                .map(|(ask_price, _)| ask_price);
            
            if current_price.is_none() {
                return None;
            }
            
            // Calculate moving average
            let avg_price = if self.price_history.len() > 0 {
                self.price_history.iter().sum::<Price>() / self.price_history.len() as f64
            } else {
                Price::from_str("0.0").unwrap()
            };
            
            // Generate signal based on moving average
            if let Some(bid_price) = current_price {
                if bid_price < avg_price {
                    Some(crate::realtime::Signal::Custom {
                        name: "buy_signal".to_string(),
                        data: {
                            symbol: self.symbol.clone(),
                            current_price: bid_price,
                            avg_price: avg_price,
                            signal_type: "below_average",
                        },
                    })
                } else if let Some(ask_price) = current_price {
                    if ask_price > avg_price {
                        Some(crate::realtime::Signal::Custom {
                            name: "sell_signal".to_string(),
                            data: {
                                symbol: self.symbol.clone(),
                                current_price: ask_price,
                                avg_price: avg_price,
                                signal_type: "above_average",
                            },
                        })
                    }
                }
            }
            
            None
        }
    }
}

/// Mock position size limit rule for testing
pub struct MockPositionSizeLimitRule {
    symbol: String,
    max_size: Size,
}

impl MockPositionSizeLimitRule {
    /// Create a new mock position size limit rule
    pub fn new(symbol: String, max_size: Size) -> Self {
        Self {
            symbol,
            max_size,
        }
    }

    impl crate::realtime::RiskRule for MockPositionSizeLimitRule {
        fn name(&self) -> &str {
            "PositionSizeLimit"
        }

        fn check_order(&self, order: &crate::traits::NewOrder) -> Option<crate::realtime::RiskViolation> {
            if let Some(max_size) = self.max_size {
                if order.quantity > *max_size {
                    Some(crate::realtime::RiskViolation::ExceedsPositionLimit {
                        symbol: self.symbol.clone(),
                        order_size: order.quantity,
                        max_size: *max_size,
                    })
                }
            }
            
            None
        }
    }
}

/// Mock exposure limit rule for testing
pub struct MockExposureLimitRule {
    symbol: String,
    max_exposure: Size,
}

impl MockExposureLimitRule {
    /// Create a new mock exposure limit rule
    pub fn new(symbol: String, max_exposure: Size) -> Self {
        Self {
            symbol,
            max_exposure,
        }
    }

    impl crate::realtime::RiskRule for MockExposureLimitRule {
        fn name(&self) -> &str {
            "ExposureLimit"
        }

        fn check_order(&self, order: &crate::traits::NewOrder) -> Option<crate::realtime::RiskViolation> {
            if let Some(max_exposure) = self.max_exposure {
                let order_value = order.quantity * order.price.unwrap_or(Price::from_str("0.0")).unwrap();
                if order_value > *max_exposure {
                    Some(crate::realtime::RiskViolation::ExceedsExposureLimit {
                        symbol: self.symbol.clone(),
                        order_value,
                        max_exposure: *max_exposure,
                    })
                }
            }
            
            None
        }
    }
}

/// Mock risk manager for testing
pub struct MockRiskManager {
    rules: Vec<Box<dyn crate::realtime::RiskRule>>,
}

impl MockRiskManager {
    /// Create a new mock risk manager
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
        }
    }

    /// Add a risk rule
    pub fn add_rule(&mut self, rule: Box<dyn crate::realtime::RiskRule>) {
        self.rules.push(rule);
    }

    /// Check if an order violates any risk rules
    pub fn check_order(&self, order: &crate::traits::NewOrder) -> Option<crate::realtime::RiskViolation> {
        for rule in &self.rules {
            if let Some(violation) = rule.check_order(order) {
                return Some(violation);
            }
        }
        
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Price, Size};
    use std::time::Duration;

    #[test]
    async fn test_complete_workflow() {
        let mut tests = IntegrationTests::new();
        
        // Test complete workflow
        tests.test_complete_workflow(&mut tests).await?;
        
        // Test high-frequency trading
        tests.test_high_frequency_trading(&mut tests).await?;
        
        // Test error handling
        tests.test_error_handling(&mut tests).await?;
        
        // Test system resilience
        tests.test_system_resilience(&mut tests).await?;
        
        Ok(())
    }

    #[test]
    async fn test_high_frequency_trading() {
        let mut event_loop = EventLoop::new();
        let mut signal_generator = SignalGenerator::new();
        let mut order_executor = OrderExecutor::new();
        let mut risk_manager = RiskManager::new();
        
        // Setup for high-frequency trading
        event_loop.add_event_processor(Box::new(SimpleEventProcessor::new()));
        signal_generator.add_strategy("BTCUSDT".to_string(), Box::new(MockMovingAverageSignalStrategy::new("BTCUSDT".to_string(), 100)));
        order_executor.add_execution_client("binance".to_string(), Box::new(MockExecutionClient::new()));
        risk_manager.add_rule(Box::new(MockPositionSizeLimitRule::new("BTCUSDT".to_string(), Size::from_str("5.0").unwrap())));
        
        // Start the event loop
        event_loop.start().await?;
        
        // Simulate high-frequency market data
        for i in 0..1000 {
            let market_event = MarketEvent::OrderBookSnapshot(crate::orderbook::OrderBookSnapshot::new(
                "BTCUSDT".to_string(),
                vec![
                    crate::orderbook::OrderBookLevel::new(
                        Price::from_str("100.0").unwrap(),
                        Size::from_str("10.0").unwrap()
                    )
                ],
                vec![
                    crate::orderbook::OrderBookLevel::new(
                        Price::from_str("101.0").unwrap(),
                        Size::from_str("10.0").unwrap()
                    )
                ],
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .as_millis() as u64,
            ));
            
            // Process the event
            let _ = event_loop.process_event(market_event).await?;
        }
        
        // Check performance metrics
        let metrics = event_loop.get_metrics();
        assert!(metrics.events_processed, 1000);
        assert!(metrics.avg_processing_time_ms > 0.0);
        assert!(metrics.peak_processing_time_ms > 0.0);
        
        Ok(())
    }

    #[test]
    async fn test_error_handling() {
        let mut event_loop = EventLoop::new();
        let mut signal_generator = SignalGenerator::new();
        let mut order_executor = OrderExecutor::new();
        let mut risk_manager = RiskManager::new();
        
        // Setup error handling
        event_loop.add_event_processor(Box::new(SimpleEventProcessor::new()));
        signal_generator.add_strategy("BTCUSDT".to_string(), Box::new(MockMovingAverageSignalStrategy::new("BTCUSDT".to_string(), 100)));
        order_executor.add_execution_client("binance".to_string(), Box::new(MockExecutionClient::new()));
        risk_manager.add_rule(Box::new(MockPositionSizeLimitRule::new("BTCUSDT".to_string(), Size::from_str("1.0").unwrap())));
        
        // Start the event loop
        event_loop.start().await?;
        
        // Simulate order that violates risk rule
        let large_order = NewOrder::new_market_buy(
            "BTCUSDT".to_string(),
            Size::from_str("10.0").unwrap()
        );
        
        // Check if order is rejected
        let order_id = order_executor.submit_order(&large_order).await?;
        let order_status = order_executor.get_order_status(&order_id).await?;
        assert_eq!(order_status, Some(crate::realtime::OrderStatus::Rejected));
        
        // Check risk violation
        let risk_violation = risk_manager.check_order(&large_order);
        assert!(risk_violation.is_some());
        
        Ok(())
    }

    #[test]
    async fn test_system_resilience() {
        let mut event_loop = EventLoop::new();
        let mut signal_generator = SignalGenerator::new();
        let mut order_executor = OrderExecutor::new();
        let mut risk_manager = RiskManager::new();
        
        // Setup for resilience testing
        event_loop.add_event_processor(Box::new(SimpleEventProcessor::new()));
        signal_generator.add_strategy("BTCUSDT".to_string(), Box::new(MockMovingAverageSignalStrategy::new("BTCUSDT".to_string(), 100)));
        order_executor.add_execution_client("binance".to_string(), Box::new(MockExecutionClient::new()));
        risk_manager.add_rule(Box::new(MockExposureLimitRule::new("BTCUSDT".to_string(), Size::from_str("100000.0").unwrap())));
        
        // Start the event loop
        event_loop.start().await?;
        
        // Simulate high load
        for i in 0..10000 {
            let market_event = MarketEvent::OrderBookSnapshot(crate::orderbook::OrderBookSnapshot::new(
                "BTCUSDT".to_string(),
                vec![
                    crate::orderbook::OrderBookLevel::new(
                        Price::from_str("100.0").unwrap(),
                        Size::from_str("10.0").unwrap()
                    )
                ],
                vec![
                    crate::orderbook::OrderBookLevel::new(
                        Price::from_str("101.0").unwrap(),
                        Size::from_str("10.0").unwrap()
                    )
                ],
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .as_millis() as u64,
            ));
            
            // Process the event
            let _ = event_loop.process_event(market_event).await?;
        }
        
        // Check performance metrics
        let metrics = event_loop.get_metrics();
        assert!(metrics.events_processed, 10000);
        assert!(metrics.avg_processing_time_ms > 0.0);
        assert!(metrics.peak_processing_time_ms > 0.0);
        
        Ok(())
    }
}
