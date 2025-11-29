# Quickstart Guide: High-Frequency Market Making System

**Date**: 2025-11-27  
**Feature**: High-Frequency Market Making System

## Overview

This guide will help you get started with the high-frequency market making system for cryptocurrency markets. The system supports market making strategies with fixed order sizes, real-time calculation of high-frequency indicators, short-term price prediction, and cross-exchange arbitrage.

## Prerequisites

- Rust 1.75 or later
- Linux environment (recommended for production)
- API credentials for supported exchanges
- Basic understanding of cryptocurrency trading concepts

## Installation

1. Clone the repository:
```bash
git clone <repository-url>
cd MMRust
```

2. Build the project:
```bash
cargo build --release
```

3. Run tests to verify installation:
```bash
cargo test
```

## Configuration

1. Create a configuration file:
```bash
cp config/example.toml config/config.toml
```

2. Edit the configuration file with your exchange credentials:
```toml
[exchanges.binance]
exchange_id = "binance"
api_key = "your-api-key"
api_secret = "your-api-secret"
sandbox = true

[exchanges.okx]
exchange_id = "okx"
api_key = "your-api-key"
api_secret = "your-api-secret"
sandbox = true
```

3. Configure your strategies:
```toml
[strategies.market_making]
strategy_type = "market_making"
symbols = ["BTCUSDT", "ETHUSDT"]
exchanges = ["binance", "okx"]

[strategies.market_making.parameters]
spread_bps = 10
order_size = "0.01"
target_inventory_ratio = "0.5"
max_position = "1.0"

[strategies.arbitrage]
strategy_type = "arbitrage"
symbols = ["BTCUSDT"]
exchanges = ["binance", "okx"]

[strategies.arbitrage.parameters]
min_profit_bps = 5
order_size = "0.01"
max_position = "1.0"
```

4. Configure risk management:
```toml
[risk_rules.max_position_size]
type = "MaxPositionSize"
symbol = "BTCUSDT"
max_size = "1.0"

[risk_rules.max_order_size]
type = "MaxOrderSize"
symbol = "BTCUSDT"
max_size = "0.1"

[risk_rules.max_daily_loss]
type = "MaxDailyLoss"
max_loss = "100.0"
```

## Running the System

### In Test Mode

1. Start the system in test mode:
```bash
cargo run --bin crypto_hft -- --config config/config.toml --test-mode
```

2. Monitor the logs:
```bash
tail -f logs/crypto_hft.log
```

### In Production Mode

1. Start the system:
```bash
cargo run --release --bin crypto_hft -- --config config/config.toml
```

2. Monitor the system:
```bash
# Check status
curl http://localhost:8080/status

# View positions
curl http://localhost:8080/positions

# View recent trades
curl http://localhost:8080/trades
```

## Monitoring

The system provides several monitoring endpoints:

- `/status`: System status and health
- `/positions`: Current positions across all exchanges
- `/orders`: Active orders
- `/trades`: Recent trades
- `/performance`: Performance metrics
- `/risk`: Risk violations

## Common Use Cases

### Market Making on a Single Exchange

1. Configure a market making strategy for a single exchange:
```toml
[strategies.market_making_binance]
strategy_type = "market_making"
symbols = ["BTCUSDT"]
exchanges = ["binance"]

[strategies.market_making_binance.parameters]
spread_bps = 10
order_size = "0.01"
target_inventory_ratio = "0.5"
max_position = "1.0"
```

2. Run the system:
```bash
cargo run --release --bin crypto_hft -- --config config/config.toml
```

### Cross-Exchange Arbitrage

1. Configure an arbitrage strategy:
```toml
[strategies.arbitrage]
strategy_type = "arbitrage"
symbols = ["BTCUSDT"]
exchanges = ["binance", "okx"]

[strategies.arbitrage.parameters]
min_profit_bps = 5
order_size = "0.01"
max_position = "1.0"
```

2. Run the system:
```bash
cargo run --release --bin crypto_hft -- --config config/config.toml
```

### Combining Strategies

1. Configure multiple strategies:
```toml
[strategies.market_making]
strategy_type = "market_making"
symbols = ["BTCUSDT"]
exchanges = ["binance"]

[strategies.market_making.parameters]
spread_bps = 10
order_size = "0.01"
target_inventory_ratio = "0.5"
max_position = "1.0"

[strategies.arbitrage]
strategy_type = "arbitrage"
symbols = ["BTCUSDT"]
exchanges = ["binance", "okx"]

[strategies.arbitrage.parameters]
min_profit_bps = 5
order_size = "0.01"
max_position = "1.0"
```

2. Run the system:
```bash
cargo run --release --bin crypto_hft -- --config config/config.toml
```

## Troubleshooting

### Common Issues

1. **Authentication Errors**
   - Verify API credentials are correct
   - Check if API keys have required permissions
   - Ensure sandbox mode is enabled for testing

2. **Connection Issues**
   - Check network connectivity
   - Verify firewall settings
   - Ensure exchange APIs are accessible

3. **Order Rejections**
   - Check if account has sufficient balance
   - Verify order parameters meet exchange requirements
   - Check if trading is enabled for the account

4. **Performance Issues**
   - Monitor system resources (CPU, memory)
   - Check network latency to exchanges
   - Review log files for errors

### Debug Mode

Enable debug mode for detailed logging:
```bash
RUST_LOG=debug cargo run --bin crypto_hft -- --config config/config.toml
```

## Performance Tuning

### System Optimization

1. **CPU Optimization**
   - Use a dedicated server with multiple cores
   - Disable CPU frequency scaling
   - Use performance CPU governor

2. **Network Optimization**
   - Use colocation with exchange data centers
   - Optimize TCP settings
   - Use dedicated network connections

3. **Memory Optimization**
   - Use sufficient RAM for order book data
   - Optimize memory allocation patterns
   - Use memory-mapped files for large datasets

### Strategy Optimization

1. **Market Making**
   - Adjust spread based on market volatility
   - Optimize inventory targets
   - Use dynamic order sizing

2. **Arbitrage**
   - Optimize detection thresholds
   - Minimize execution latency
   - Consider transaction costs

## Security Considerations

1. **API Key Management**
   - Store API keys securely
   - Use environment variables for sensitive data
   - Rotate API keys regularly

2. **Network Security**
   - Use encrypted connections
   - Implement firewall rules
   - Monitor for unauthorized access

3. **System Security**
   - Keep system updated
   - Use minimal privileges
   - Implement audit logging

## Advanced Configuration

### Custom Strategies

1. Create a new strategy file:
```rust
// src/strategies/custom_strategy.rs
use crate::traits::*;
use crate::types::*;

pub struct CustomStrategy {
    // Strategy state
}

impl Strategy for CustomStrategy {
    // Implement required methods
}
```

2. Register the strategy:
```rust
// src/strategies/mod.rs
pub mod custom_strategy;
```

3. Configure the strategy:
```toml
[strategies.custom]
strategy_type = "custom"
symbols = ["BTCUSDT"]
exchanges = ["binance"]

[strategies.custom.parameters]
# Custom parameters
```

### Custom Indicators

1. Create a new indicator:
```rust
// src/indicators/custom_indicator.rs
use crate::types::*;

pub struct CustomIndicator {
    // Indicator state
}

impl CustomIndicator {
    pub fn calculate(&self, orderbook: &OrderBookSnapshot) -> Decimal {
        // Custom calculation
    }
}
```

2. Use the indicator in a strategy:
```rust
let indicator = CustomIndicator::new();
let value = indicator.calculate(&orderbook);
```

## Support

For support and questions:

1. Check the documentation
2. Review the log files
3. Search existing issues
4. Create a new issue with detailed information

## Contributing

To contribute to the project:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.
