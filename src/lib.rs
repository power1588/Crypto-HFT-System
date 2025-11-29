pub mod connectors;
pub mod core;
pub mod exchanges;
pub mod indicators;
pub mod monitoring;
pub mod oms;
pub mod orderbook;
pub mod realtime;
pub mod risk;
pub mod security;
pub mod strategies;
pub mod strategy;
pub mod traits;
pub mod types;

pub use core::events::{Signal, TradingEvent};
pub use exchanges::{
    BinanceAdapter,
    BinanceWebSocketAdapter,
    // Temporarily disabled
    // OkxAdapter,
    ConnectionManager,
    ConnectionStatus,
    ExchangeAdapter,
    MockExchangeAdapter,
};
pub use oms::RateLimiter;
pub use orderbook::{OrderBook, OrderBookDelta, OrderBookLevel, OrderBookSnapshot};
pub use realtime::{
    event_loop::EventLoop, order_executor::OrderExecutor, performance_monitor::PerformanceMonitor,
    risk_manager::RiskManager as RealtimeRiskManager, signal_generator::SignalGenerator,
};
pub use risk::{RiskEngine, RiskRule, RiskViolation, ShadowLedger};
pub use strategies::{
    ArbitrageStrategy, EventDrivenStrategy, MarketMakingStrategy as MMStrategy, PortfolioRebalancer,
};
pub use strategy::{MarketState, SimpleArbitrageStrategy, Strategy as SimpleStrategy};
pub use traits::strategy::{
    PositionManager, RiskManager, SignalValidator, Strategy, StrategyConfig, StrategyMetrics,
    StrategyState,
};
pub use traits::{
    Balance, ExecutionClient, ExecutionReport, MarketDataHistory, MarketDataStream, MarketEvent,
    NewOrder, OrderId, OrderManager, OrderSide, OrderStatus, OrderType, TimeInForce, Trade,
    TradingFees,
};
pub use types::{Price, Size, Symbol};

// Initialize logging
use log::info;

/// Initialize the logging system
pub fn init_logging(level: &str, log_file: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    // Set log level
    let log_level = match level {
        "error" => log::LevelFilter::Error,
        "warn" => log::LevelFilter::Warn,
        "info" => log::LevelFilter::Info,
        "debug" => log::LevelFilter::Debug,
        "trace" => log::LevelFilter::Trace,
        _ => return Err("Invalid log level".into()),
    };

    // Configure file logger if file path provided
    let mut builders = vec![fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} [{}] {} - {}",
                record.level(),
                record.target(),
                chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ"),
                message
            ))
        })
        .level(log_level)
        .chain(std::io::stdout())];

    // Add file logger if file path is provided
    if let Some(file_path) = log_file {
        let file_logger = fern::log_file(file_path)?;
        builders.push(
            fern::Dispatch::new()
                .format(|out, message, record| {
                    out.finish(format_args!(
                        "{} [{}] {} - {}",
                        record.level(),
                        record.target(),
                        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ"),
                        message
                    ))
                })
                .level(log_level)
                .chain(file_logger),
        );
    }

    // Apply all builders
    for builder in builders {
        builder.apply()?;
    }

    info!("Logging initialized with level: {}", level);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_initialization() {
        // Test that logging can be initialized
        // Note: This test might not work in all environments due to file permissions
        assert!(init_logging("info", None).is_ok());
    }
}
