use crate::core::events::{OrderSide, Trade};
use crate::types::{Price, Size};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use std::collections::VecDeque;

/// Trade flow indicator that tracks buy/sell pressure
pub struct TradeFlowIndicator {
    /// Historical trades
    trades: VecDeque<Trade>,
    /// Maximum number of trades to keep
    max_trades: usize,
    /// Time window in milliseconds
    time_window_ms: u64,
}

impl TradeFlowIndicator {
    /// Create a new trade flow indicator
    pub fn new(max_trades: usize, time_window_ms: u64) -> Self {
        Self {
            trades: VecDeque::with_capacity(max_trades),
            max_trades,
            time_window_ms,
        }
    }

    /// Add a new trade to the indicator
    pub fn add_trade(&mut self, trade: Trade) {
        // Remove old trades outside the time window
        self.remove_old_trades(trade.timestamp);

        // Add new trade
        self.trades.push_back(trade);

        // Remove oldest trades if we exceed max_trades
        while self.trades.len() > self.max_trades {
            self.trades.pop_front();
        }
    }

    /// Remove trades that are outside the time window
    fn remove_old_trades(&mut self, current_timestamp: u64) {
        let cutoff = current_timestamp.saturating_sub(self.time_window_ms);
        while let Some(front) = self.trades.front() {
            if front.timestamp < cutoff {
                self.trades.pop_front();
            } else {
                break;
            }
        }
    }

    /// Calculate buy pressure (volume-weighted buy trades)
    pub fn buy_pressure(&self) -> Size {
        self.trades
            .iter()
            .filter(|trade| trade.side == OrderSide::Buy)
            .fold(Size::new(Decimal::ZERO), |acc, trade| acc + trade.size)
    }

    /// Calculate sell pressure (volume-weighted sell trades)
    pub fn sell_pressure(&self) -> Size {
        self.trades
            .iter()
            .filter(|trade| trade.side == OrderSide::Sell)
            .fold(Size::new(Decimal::ZERO), |acc, trade| acc + trade.size)
    }

    /// Calculate net flow (buy_pressure - sell_pressure)
    pub fn net_flow(&self) -> Size {
        self.buy_pressure() - self.sell_pressure()
    }

    /// Calculate flow ratio: (buy_pressure - sell_pressure) / total_volume
    /// Returns a value between -1.0 (all sells) and 1.0 (all buys)
    pub fn flow_ratio(&self) -> Option<f64> {
        let buy = self.buy_pressure();
        let sell = self.sell_pressure();
        let total = buy + sell;

        if total.is_zero() {
            return Some(0.0);
        }

        let net = buy.value() - sell.value();
        let ratio = net / total.value();
        Some(ratio.to_f64().unwrap_or(0.0))
    }

    /// Calculate volume-weighted average price (VWAP) for buy trades
    pub fn buy_vwap(&self) -> Option<Price> {
        let buy_trades: Vec<_> = self
            .trades
            .iter()
            .filter(|trade| trade.side == OrderSide::Buy)
            .collect();

        if buy_trades.is_empty() {
            return None;
        }

        let total_value = buy_trades.iter().fold(Decimal::ZERO, |acc, trade| {
            acc + (trade.price.value() * trade.size.value())
        });

        let total_volume = buy_trades
            .iter()
            .fold(Size::new(Decimal::ZERO), |acc, trade| acc + trade.size);

        if total_volume.is_zero() {
            return None;
        }

        Some(Price::new(total_value / total_volume.value()))
    }

    /// Calculate volume-weighted average price (VWAP) for sell trades
    pub fn sell_vwap(&self) -> Option<Price> {
        let sell_trades: Vec<_> = self
            .trades
            .iter()
            .filter(|trade| trade.side == OrderSide::Sell)
            .collect();

        if sell_trades.is_empty() {
            return None;
        }

        let total_value = sell_trades.iter().fold(Decimal::ZERO, |acc, trade| {
            acc + (trade.price.value() * trade.size.value())
        });

        let total_volume = sell_trades
            .iter()
            .fold(Size::new(Decimal::ZERO), |acc, trade| acc + trade.size);

        if total_volume.is_zero() {
            return None;
        }

        Some(Price::new(total_value / total_volume.value()))
    }

    /// Calculate overall VWAP (all trades)
    pub fn overall_vwap(&self) -> Option<Price> {
        if self.trades.is_empty() {
            return None;
        }

        let total_value = self.trades.iter().fold(Decimal::ZERO, |acc, trade| {
            acc + (trade.price.value() * trade.size.value())
        });

        let total_volume = self
            .trades
            .iter()
            .fold(Size::new(Decimal::ZERO), |acc, trade| acc + trade.size);

        if total_volume.is_zero() {
            return None;
        }

        Some(Price::new(total_value / total_volume.value()))
    }

    /// Get the number of trades currently tracked
    pub fn trade_count(&self) -> usize {
        self.trades.len()
    }

    /// Clear all trades
    pub fn clear(&mut self) {
        self.trades.clear();
    }
}

/// Trade flow momentum indicator
/// Tracks the rate of change in trade flow
pub struct TradeFlowMomentum {
    /// Historical flow ratios
    flow_ratios: VecDeque<f64>,
    /// Window size for momentum calculation
    window_size: usize,
    /// Trade flow indicator
    flow_indicator: TradeFlowIndicator,
}

impl TradeFlowMomentum {
    /// Create a new trade flow momentum indicator
    pub fn new(max_trades: usize, time_window_ms: u64, window_size: usize) -> Self {
        Self {
            flow_ratios: VecDeque::with_capacity(window_size),
            window_size,
            flow_indicator: TradeFlowIndicator::new(max_trades, time_window_ms),
        }
    }

    /// Add a new trade and update momentum
    pub fn add_trade(&mut self, trade: Trade) {
        self.flow_indicator.add_trade(trade);

        // Update flow ratio history
        if let Some(ratio) = self.flow_indicator.flow_ratio() {
            self.flow_ratios.push_back(ratio);

            // Remove oldest ratio if we exceed window size
            if self.flow_ratios.len() > self.window_size {
                self.flow_ratios.pop_front();
            }
        }
    }

    /// Calculate momentum (rate of change in flow ratio)
    /// Returns the difference between current and previous flow ratio
    pub fn momentum(&self) -> Option<f64> {
        if self.flow_ratios.len() < 2 {
            return None;
        }

        let current = *self.flow_ratios.back()?;
        let previous = *self.flow_ratios.get(self.flow_ratios.len() - 2)?;

        Some(current - previous)
    }

    /// Calculate average momentum over the window
    pub fn average_momentum(&self) -> Option<f64> {
        if self.flow_ratios.len() < 2 {
            return None;
        }

        let mut sum = 0.0;
        for i in 1..self.flow_ratios.len() {
            sum += self.flow_ratios[i] - self.flow_ratios[i - 1];
        }

        Some(sum / (self.flow_ratios.len() - 1) as f64)
    }

    /// Get the current flow ratio
    pub fn current_flow_ratio(&self) -> Option<f64> {
        self.flow_indicator.flow_ratio()
    }

    /// Get the underlying trade flow indicator
    pub fn flow_indicator(&self) -> &TradeFlowIndicator {
        &self.flow_indicator
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Symbol;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn create_test_trade(side: OrderSide, price: &str, size: &str, timestamp: u64) -> Trade {
        Trade {
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "binance".to_string(),
            price: Price::from_str(price).unwrap(),
            size: Size::from_str(size).unwrap(),
            side,
            timestamp,
            trade_id: Some(format!("trade_{}", timestamp)),
        }
    }

    #[test]
    fn test_trade_flow_indicator_creation() {
        let indicator = TradeFlowIndicator::new(100, 60000);
        assert_eq!(indicator.trade_count(), 0);
        assert_eq!(
            indicator.buy_pressure(),
            Size::new(Decimal::ZERO)
        );
        assert_eq!(
            indicator.sell_pressure(),
            Size::new(Decimal::ZERO)
        );
    }

    #[test]
    fn test_buy_pressure() {
        let mut indicator = TradeFlowIndicator::new(100, 60000);

        indicator.add_trade(create_test_trade(OrderSide::Buy, "100.00", "1.0", 1000));
        indicator.add_trade(create_test_trade(OrderSide::Buy, "100.10", "2.0", 2000));
        indicator.add_trade(create_test_trade(OrderSide::Sell, "100.05", "0.5", 3000));

        assert_eq!(indicator.buy_pressure(), Size::from_str("3.0").unwrap());
        assert_eq!(indicator.sell_pressure(), Size::from_str("0.5").unwrap());
    }

    #[test]
    fn test_net_flow() {
        let mut indicator = TradeFlowIndicator::new(100, 60000);

        indicator.add_trade(create_test_trade(OrderSide::Buy, "100.00", "2.0", 1000));
        indicator.add_trade(create_test_trade(OrderSide::Sell, "100.05", "1.0", 2000));

        assert_eq!(indicator.net_flow(), Size::from_str("1.0").unwrap());
    }

    #[test]
    fn test_flow_ratio() {
        let mut indicator = TradeFlowIndicator::new(100, 60000);

        // All buys
        indicator.add_trade(create_test_trade(OrderSide::Buy, "100.00", "1.0", 1000));
        assert_eq!(indicator.flow_ratio(), Some(1.0));

        // All sells
        indicator.clear();
        indicator.add_trade(create_test_trade(OrderSide::Sell, "100.00", "1.0", 1000));
        assert_eq!(indicator.flow_ratio(), Some(-1.0));

        // Balanced
        indicator.clear();
        indicator.add_trade(create_test_trade(OrderSide::Buy, "100.00", "1.0", 1000));
        indicator.add_trade(create_test_trade(OrderSide::Sell, "100.00", "1.0", 2000));
        assert_eq!(indicator.flow_ratio(), Some(0.0));
    }

    #[test]
    fn test_vwap() {
        let mut indicator = TradeFlowIndicator::new(100, 60000);

        // Buy trades: 1.0 @ 100.00, 2.0 @ 100.10
        // VWAP = (1.0 * 100.00 + 2.0 * 100.10) / 3.0 = 300.20 / 3.0 = 100.066...
        indicator.add_trade(create_test_trade(OrderSide::Buy, "100.00", "1.0", 1000));
        indicator.add_trade(create_test_trade(OrderSide::Buy, "100.10", "2.0", 2000));

        let buy_vwap = indicator.buy_vwap();
        assert!(buy_vwap.is_some());
        let vwap_value = buy_vwap.unwrap().value();
        assert!(vwap_value > Decimal::from_str("100.06").unwrap());
        assert!(vwap_value < Decimal::from_str("100.07").unwrap());
    }

    #[test]
    fn test_time_window() {
        let mut indicator = TradeFlowIndicator::new(100, 5000); // 5 second window

        indicator.add_trade(create_test_trade(OrderSide::Buy, "100.00", "1.0", 1000));
        assert_eq!(indicator.trade_count(), 1);

        // Add trade within window
        indicator.add_trade(create_test_trade(OrderSide::Buy, "100.10", "1.0", 5000));
        assert_eq!(indicator.trade_count(), 2);

        // Add trade outside window (should remove first trade)
        indicator.add_trade(create_test_trade(OrderSide::Buy, "100.20", "1.0", 7000));
        assert_eq!(indicator.trade_count(), 2); // First trade removed
    }

    #[test]
    fn test_trade_flow_momentum() {
        let mut momentum = TradeFlowMomentum::new(100, 60000, 5);

        // Add trades with increasing buy pressure
        momentum.add_trade(create_test_trade(OrderSide::Buy, "100.00", "1.0", 1000));
        momentum.add_trade(create_test_trade(OrderSide::Sell, "100.00", "1.0", 2000));
        // Flow ratio should be 0.0

        momentum.add_trade(create_test_trade(OrderSide::Buy, "100.00", "2.0", 3000));
        // Flow ratio should be positive now

        let momentum_value = momentum.momentum();
        assert!(momentum_value.is_some());
    }

    #[test]
    fn test_clear() {
        let mut indicator = TradeFlowIndicator::new(100, 60000);

        indicator.add_trade(create_test_trade(OrderSide::Buy, "100.00", "1.0", 1000));
        assert_eq!(indicator.trade_count(), 1);

        indicator.clear();
        assert_eq!(indicator.trade_count(), 0);
    }
}
