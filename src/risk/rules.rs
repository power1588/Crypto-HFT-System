use crate::core::events::{NewOrder, OrderSide, Position, RiskViolation};
use crate::types::{Price, Size};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Trait for risk rules that can check orders
#[async_trait::async_trait]
pub trait RiskRule: Send + Sync {
    /// Check if an order violates this risk rule
    /// Returns Some(RiskViolation) if the order violates the rule, None otherwise
    async fn check_order(
        &self,
        order: &NewOrder,
        risk_engine: &RiskEngine,
    ) -> Option<RiskViolation>;
}

/// Risk engine that evaluates and enforces risk rules
pub struct RiskEngine {
    /// All risk rules
    rules: Arc<RwLock<Vec<Box<dyn RiskRule>>>>,
    /// Current positions by symbol
    positions: Arc<RwLock<HashMap<String, Position>>>,
    /// Account balances by asset
    balances: Arc<RwLock<HashMap<String, Size>>>,
    /// Maximum position size by symbol
    max_position_sizes: Arc<RwLock<HashMap<String, Size>>>,
    /// Maximum order size by symbol
    max_order_sizes: Arc<RwLock<HashMap<String, Size>>>,
    /// Maximum daily loss by symbol
    max_daily_losses: Arc<RwLock<HashMap<String, Price>>>,
    /// Daily losses by symbol
    daily_losses: Arc<RwLock<HashMap<String, Price>>>,
    /// Maximum total exposure
    max_total_exposure: Arc<RwLock<Price>>,
    /// Maximum number of open orders
    max_open_orders: Arc<RwLock<usize>>,
    /// Current number of open orders
    open_orders_count: Arc<RwLock<usize>>,
}

impl RiskEngine {
    /// Create a new risk engine
    pub fn new() -> Self {
        Self {
            rules: Arc::new(RwLock::new(Vec::new())),
            positions: Arc::new(RwLock::new(HashMap::new())),
            balances: Arc::new(RwLock::new(HashMap::new())),
            max_position_sizes: Arc::new(RwLock::new(HashMap::new())),
            max_order_sizes: Arc::new(RwLock::new(HashMap::new())),
            max_daily_losses: Arc::new(RwLock::new(HashMap::new())),
            daily_losses: Arc::new(RwLock::new(HashMap::new())),
            max_total_exposure: Arc::new(RwLock::new(Price::new(rust_decimal::Decimal::MAX))),
            max_open_orders: Arc::new(RwLock::new(100)),
            open_orders_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Add a risk rule
    pub async fn add_rule(&self, rule: Box<dyn RiskRule>) {
        let mut rules = self.rules.write().await;
        rules.push(rule);
    }

    /// Set maximum position size for a symbol
    pub async fn set_max_position_size(&self, symbol: &str, max_size: Size) {
        let mut max_sizes = self.max_position_sizes.write().await;
        max_sizes.insert(symbol.to_string(), max_size);
    }

    /// Set maximum order size for a symbol
    pub async fn set_max_order_size(&self, symbol: &str, max_size: Size) {
        let mut max_sizes = self.max_order_sizes.write().await;
        max_sizes.insert(symbol.to_string(), max_size);
    }

    /// Set maximum daily loss for a symbol
    pub async fn set_max_daily_loss(&self, symbol: &str, max_loss: Price) {
        let mut max_losses = self.max_daily_losses.write().await;
        max_losses.insert(symbol.to_string(), max_loss);
    }

    /// Set maximum total exposure
    pub async fn set_max_total_exposure(&self, max_exposure: Price) {
        let mut max_exp = self.max_total_exposure.write().await;
        *max_exp = max_exposure;
    }

    /// Set maximum number of open orders
    pub async fn set_max_open_orders(&self, max_orders: usize) {
        let mut max_orders_ref = self.max_open_orders.write().await;
        *max_orders_ref = max_orders;
    }

    /// Update account balance for an asset
    pub async fn update_balance(&self, asset: &str, balance: Size) {
        let mut balances = self.balances.write().await;
        balances.insert(asset.to_string(), balance);
    }

    /// Update position for a symbol
    pub async fn update_position(&self, symbol: &str, position: Position) {
        let mut positions = self.positions.write().await;
        positions.insert(symbol.to_string(), position);
    }

    /// Record a daily loss for a symbol
    pub async fn record_daily_loss(&self, symbol: &str, loss: Price) {
        let mut daily_losses = self.daily_losses.write().await;
        let current_loss = daily_losses
            .get(symbol)
            .cloned()
            .unwrap_or(Price::new(rust_decimal::Decimal::ZERO));
        daily_losses.insert(symbol.to_string(), current_loss + loss);
    }

    /// Reset daily losses (typically called at start of day)
    pub async fn reset_daily_losses(&self) {
        let mut daily_losses = self.daily_losses.write().await;
        daily_losses.clear();
    }

    /// Increment open orders count
    pub async fn increment_open_orders(&self) {
        let mut count = self.open_orders_count.write().await;
        *count += 1;
    }

    /// Decrement open orders count
    pub async fn decrement_open_orders(&self) {
        let mut count = self.open_orders_count.write().await;
        if *count > 0 {
            *count -= 1;
        }
    }

    /// Check if an order passes all risk rules
    pub async fn check_order(&self, order: &NewOrder) -> Result<(), RiskViolation> {
        let rules = self.rules.read().await;

        // Check against all rules
        for rule in rules.iter() {
            if let Some(violation) = rule.check_order(order, self).await {
                return Err(violation);
            }
        }

        Ok(())
    }

    /// Get current position for a symbol
    pub async fn get_position(&self, symbol: &str) -> Option<Position> {
        let positions = self.positions.read().await;
        positions.get(symbol).cloned()
    }

    /// Get current balance for an asset
    pub async fn get_balance(&self, asset: &str) -> Size {
        let balances = self.balances.read().await;
        balances
            .get(asset)
            .cloned()
            .unwrap_or(Size::new(rust_decimal::Decimal::ZERO))
    }

    /// Get total exposure across all positions
    pub async fn get_total_exposure(&self) -> Price {
        let positions = self.positions.read().await;
        let total_exposure = positions
            .values()
            .fold(rust_decimal::Decimal::ZERO, |acc, pos| {
                if let Some(avg_price) = pos.average_price {
                    acc + pos.size.value() * avg_price.value()
                } else {
                    acc
                }
            });

        Price::new(total_exposure)
    }

    /// Get current number of open orders
    pub async fn get_open_orders_count(&self) -> usize {
        let count = self.open_orders_count.read().await;
        *count
    }

    /// Get maximum position size for a symbol
    pub async fn get_max_position_size(&self, symbol: &str) -> Size {
        let max_sizes = self.max_position_sizes.read().await;
        max_sizes
            .get(symbol)
            .cloned()
            .unwrap_or(Size::new(rust_decimal::Decimal::MAX))
    }

    /// Get maximum order size for a symbol
    pub async fn get_max_order_size(&self, symbol: &str) -> Size {
        let max_sizes = self.max_order_sizes.read().await;
        max_sizes
            .get(symbol)
            .cloned()
            .unwrap_or(Size::new(rust_decimal::Decimal::MAX))
    }

    /// Get maximum daily loss for a symbol
    pub async fn get_max_daily_loss(&self, symbol: &str) -> Price {
        let max_losses = self.max_daily_losses.read().await;
        max_losses
            .get(symbol)
            .cloned()
            .unwrap_or(Price::new(rust_decimal::Decimal::MAX))
    }

    /// Get current daily loss for a symbol
    pub async fn get_daily_loss(&self, symbol: &str) -> Price {
        let daily_losses = self.daily_losses.read().await;
        daily_losses
            .get(symbol)
            .cloned()
            .unwrap_or(Price::new(rust_decimal::Decimal::ZERO))
    }

    /// Get maximum total exposure
    pub async fn get_max_total_exposure(&self) -> Price {
        let max_exp = self.max_total_exposure.read().await;
        *max_exp
    }

    /// Get maximum number of open orders
    pub async fn get_max_open_orders(&self) -> usize {
        let max_orders = self.max_open_orders.read().await;
        *max_orders
    }

    /// Get all positions
    pub async fn get_all_positions(&self) -> Vec<Position> {
        let positions = self.positions.read().await;
        positions.values().cloned().collect()
    }

    /// Get position statistics
    pub async fn get_position_stats(&self) -> PositionStats {
        let positions = self.positions.read().await;
        let total_positions = positions.len();
        let long_positions = positions
            .values()
            .filter(|p| p.size.value() > rust_decimal::Decimal::ZERO)
            .count();
        let short_positions = positions
            .values()
            .filter(|p| p.size.value() < rust_decimal::Decimal::ZERO)
            .count();
        let total_exposure = positions
            .values()
            .fold(rust_decimal::Decimal::ZERO, |acc, pos| {
                if let Some(avg_price) = pos.average_price {
                    acc + pos.size.value().abs() * avg_price.value()
                } else {
                    acc
                }
            });

        PositionStats {
            total_positions,
            long_positions,
            short_positions,
            total_exposure: Price::new(total_exposure),
        }
    }

    /// Cancel all orders for a symbol (stub - returns empty vector)
    /// In a real implementation, this would interact with order management
    pub async fn cancel_all_orders_for_symbol(&self, _symbol: &str) -> Vec<String> {
        // Stub implementation - would need OrderManager integration
        Vec::new()
    }
}

/// Position statistics
#[derive(Debug, Clone)]
pub struct PositionStats {
    /// Total number of positions
    pub total_positions: usize,
    /// Number of long positions
    pub long_positions: usize,
    /// Number of short positions
    pub short_positions: usize,
    /// Total exposure
    pub total_exposure: Price,
}

/// Position size limit rule
pub struct PositionSizeRule {
    /// Maximum position size by symbol
    max_positions: HashMap<String, Size>,
}

impl PositionSizeRule {
    /// Create a new position size rule
    pub fn new() -> Self {
        Self {
            max_positions: HashMap::new(),
        }
    }

    /// Set maximum position size for a symbol
    pub fn set_max_position(&mut self, symbol: &str, max_size: Size) {
        self.max_positions.insert(symbol.to_string(), max_size);
    }
}

#[async_trait::async_trait]
impl RiskRule for PositionSizeRule {
    async fn check_order(
        &self,
        order: &NewOrder,
        risk_engine: &RiskEngine,
    ) -> Option<RiskViolation> {
        // Get current position
        let current_position = risk_engine.get_position(order.symbol.as_str()).await;
        let current_size = current_position
            .map(|p| p.size)
            .unwrap_or(Size::new(rust_decimal::Decimal::ZERO));

        // Get maximum position size
        let max_size = risk_engine
            .get_max_position_size(order.symbol.as_str())
            .await;

        // Calculate new position size
        let new_size = match order.side {
            OrderSide::Buy => current_size + order.size,
            OrderSide::Sell => current_size - order.size,
        };

        // Check if new position would exceed limit
        if new_size.abs() > max_size {
            return Some(RiskViolation::new(
                "PositionSizeLimit".to_string(),
                format!(
                    "Order would exceed maximum position size for {}: current={}, new={}, max={}",
                    order.symbol, current_size, new_size, max_size
                ),
            ));
        }

        None
    }
}

/// Order size limit rule
pub struct OrderSizeRule {
    /// Maximum order size by symbol
    max_orders: HashMap<String, Size>,
}

impl OrderSizeRule {
    /// Create a new order size rule
    pub fn new() -> Self {
        Self {
            max_orders: HashMap::new(),
        }
    }

    /// Set maximum order size for a symbol
    pub fn set_max_order(&mut self, symbol: &str, max_size: Size) {
        self.max_orders.insert(symbol.to_string(), max_size);
    }
}

#[async_trait::async_trait]
impl RiskRule for OrderSizeRule {
    async fn check_order(
        &self,
        order: &NewOrder,
        risk_engine: &RiskEngine,
    ) -> Option<RiskViolation> {
        // Get maximum order size
        let max_size = risk_engine.get_max_order_size(order.symbol.as_str()).await;

        // Check if order size exceeds limit
        if order.size > max_size {
            return Some(RiskViolation::new(
                "OrderSizeLimit".to_string(),
                format!(
                    "Order size exceeds maximum for {}: order={}, max={}",
                    order.symbol, order.size, max_size
                ),
            ));
        }

        None
    }
}

/// Daily loss limit rule
pub struct DailyLossRule {
    /// Maximum daily loss by symbol
    max_losses: HashMap<String, Price>,
}

impl DailyLossRule {
    /// Create a new daily loss rule
    pub fn new() -> Self {
        Self {
            max_losses: HashMap::new(),
        }
    }

    /// Set maximum daily loss for a symbol
    pub fn set_max_loss(&mut self, symbol: &str, max_loss: Price) {
        self.max_losses.insert(symbol.to_string(), max_loss);
    }
}

#[async_trait::async_trait]
impl RiskRule for DailyLossRule {
    async fn check_order(
        &self,
        order: &NewOrder,
        risk_engine: &RiskEngine,
    ) -> Option<RiskViolation> {
        // For sell orders, check potential loss
        if order.side == OrderSide::Sell {
            if let Some(price) = order.price {
                // Get current position
                let current_position = risk_engine.get_position(order.symbol.as_str()).await;

                if let Some(pos) = current_position {
                    // Calculate potential loss if this order fills
                    let avg_price = pos.average_price.unwrap_or(price);
                    let potential_loss_per_unit = if price > avg_price {
                        price - avg_price // Selling at profit
                    } else {
                        avg_price - price // Selling at loss
                    };

                    let potential_loss = potential_loss_per_unit * order.size;

                    // Get current daily loss
                    let current_daily_loss =
                        risk_engine.get_daily_loss(order.symbol.as_str()).await;

                    // Get maximum daily loss
                    let max_daily_loss =
                        risk_engine.get_max_daily_loss(order.symbol.as_str()).await;

                    // Check if this would exceed daily loss limit
                    // Convert potential_loss (Decimal from Price * Size) to Price for comparison
                    let potential_loss_price = Price::new(potential_loss);
                    if current_daily_loss + potential_loss_price > max_daily_loss {
                        return Some(RiskViolation::new(
                            "DailyLossLimit".to_string(),
                            format!(
                        "Order would exceed maximum daily loss for {}: current={}, potential={}, max={}",
                        order.symbol, current_daily_loss, potential_loss_price, max_daily_loss
                            ),
                        ));
                    }
                }
            }
        }

        None
    }
}

/// Total exposure limit rule
pub struct TotalExposureRule {
    /// Maximum total exposure
    max_exposure: Price,
}

impl TotalExposureRule {
    /// Create a new total exposure rule
    pub fn new(max_exposure: Price) -> Self {
        Self { max_exposure }
    }
}

#[async_trait::async_trait]
impl RiskRule for TotalExposureRule {
    async fn check_order(
        &self,
        order: &NewOrder,
        risk_engine: &RiskEngine,
    ) -> Option<RiskViolation> {
        // Get current total exposure
        let current_exposure = risk_engine.get_total_exposure().await;

        // Calculate potential new exposure
        // Note: Price * Size returns Decimal, so we wrap in Price for arithmetic
        let potential_new_exposure = if let Some(price) = order.price {
            let order_value = Price::new(price * order.size);
            match order.side {
                OrderSide::Buy => current_exposure + order_value,
                OrderSide::Sell => current_exposure - order_value,
            }
        } else {
            current_exposure
        };

        // Check if new exposure would exceed limit
        if potential_new_exposure.abs() > self.max_exposure {
            return Some(RiskViolation::new(
                "TotalExposureLimit".to_string(),
                format!(
                    "Order would exceed maximum total exposure: current={}, potential={}, max={}",
                    current_exposure, potential_new_exposure, self.max_exposure
                ),
            ));
        }

        None
    }
}

/// Open orders count limit rule
pub struct OpenOrdersCountRule {
    /// Maximum number of open orders
    max_open_orders: usize,
}

impl OpenOrdersCountRule {
    /// Create a new open orders count rule
    pub fn new(max_open_orders: usize) -> Self {
        Self { max_open_orders }
    }
}

#[async_trait::async_trait]
impl RiskRule for OpenOrdersCountRule {
    async fn check_order(
        &self,
        _order: &NewOrder,
        risk_engine: &RiskEngine,
    ) -> Option<RiskViolation> {
        // Get current open orders count
        let current_count = risk_engine.get_open_orders_count().await;

        // Check if adding this order would exceed limit
        if current_count >= self.max_open_orders {
            return Some(RiskViolation::new(
                "OpenOrdersCountLimit".to_string(),
                format!(
                    "Would exceed maximum open orders: current={}, max={}",
                    current_count, self.max_open_orders
                ),
            ));
        }

        None
    }
}

/// Balance check rule
pub struct BalanceRule {
    /// Minimum required balance by asset
    min_balances: HashMap<String, Size>,
}

impl BalanceRule {
    /// Create a new balance rule
    pub fn new() -> Self {
        Self {
            min_balances: HashMap::new(),
        }
    }

    /// Set minimum required balance for an asset
    pub fn set_min_balance(&mut self, asset: &str, min_balance: Size) {
        self.min_balances.insert(asset.to_string(), min_balance);
    }
}

#[async_trait::async_trait]
impl RiskRule for BalanceRule {
    async fn check_order(
        &self,
        order: &NewOrder,
        risk_engine: &RiskEngine,
    ) -> Option<RiskViolation> {
        // For buy orders, check if we have enough balance
        if order.side == OrderSide::Buy {
            if let Some(price) = order.price {
                // Calculate required balance (Price * Size returns Decimal)
                let required_balance_decimal = price * order.size;
                let required_balance = Size::new(required_balance_decimal);

                // Extract base asset from symbol (e.g., BTC from BTCUSDT)
                // Use as_str() to get string slice for indexing
                let symbol_str = order.symbol.as_str();
                let base_asset = if symbol_str.len() >= 4 {
                    &symbol_str[..symbol_str.len() - 4] // Remove USDT suffix
                } else {
                    symbol_str
                };

                // Get current balance
                let current_balance = risk_engine.get_balance(base_asset).await;

                // Get minimum required balance
                let min_balance = self
                    .min_balances
                    .get(base_asset)
                    .cloned()
                    .unwrap_or(Size::new(rust_decimal::Decimal::ZERO));

                // Check if we have enough balance (all Size types now)
                if current_balance < min_balance + required_balance {
                    return Some(RiskViolation::new(
                        "InsufficientBalance".to_string(),
                        format!(
                            "Insufficient balance for {}: current={}, required={}, min={}",
                            base_asset, current_balance, required_balance, min_balance
                        ),
                    ));
                }
            }
        }

        None
    }
}

/// Maximum drawdown rule
/// Limits the maximum drawdown from peak equity
pub struct MaxDrawdownRule {
    /// Maximum drawdown percentage (e.g., 0.1 for 10%)
    max_drawdown_percent: rust_decimal::Decimal,
    /// Peak equity value
    peak_equity: rust_decimal::Decimal,
}

impl MaxDrawdownRule {
    /// Create a new max drawdown rule
    pub fn new(max_drawdown_percent: rust_decimal::Decimal) -> Self {
        Self {
            max_drawdown_percent,
            peak_equity: rust_decimal::Decimal::ZERO,
        }
    }

    /// Update peak equity
    pub fn update_peak_equity(&mut self, equity: rust_decimal::Decimal) {
        if equity > self.peak_equity {
            self.peak_equity = equity;
        }
    }

    /// Get current drawdown percentage
    pub fn get_drawdown_percent(
        &self,
        current_equity: rust_decimal::Decimal,
    ) -> rust_decimal::Decimal {
        if self.peak_equity.is_zero() {
            return rust_decimal::Decimal::ZERO;
        }

        let drawdown = self.peak_equity - current_equity;
        drawdown / self.peak_equity
    }
}

#[async_trait::async_trait]
impl RiskRule for MaxDrawdownRule {
    async fn check_order(
        &self,
        _order: &NewOrder,
        risk_engine: &RiskEngine,
    ) -> Option<RiskViolation> {
        // Get total exposure as proxy for equity
        let total_exposure = risk_engine.get_total_exposure().await;
        let current_equity = total_exposure.value();

        // Calculate drawdown
        let drawdown_percent = self.get_drawdown_percent(current_equity);

        if drawdown_percent > self.max_drawdown_percent {
            return Some(RiskViolation::new(
                "MaxDrawdownLimit".to_string(),
                format!(
                    "Drawdown exceeds maximum: current={}%, max={}%",
                    drawdown_percent * rust_decimal::Decimal::new(100, 0),
                    self.max_drawdown_percent * rust_decimal::Decimal::new(100, 0)
                ),
            ));
        }

        None
    }
}

/// Concentration limit rule
/// Limits the maximum percentage of total exposure in a single symbol
pub struct ConcentrationLimitRule {
    /// Maximum concentration percentage per symbol (e.g., 0.3 for 30%)
    max_concentration_percent: rust_decimal::Decimal,
}

impl ConcentrationLimitRule {
    /// Create a new concentration limit rule
    pub fn new(max_concentration_percent: rust_decimal::Decimal) -> Self {
        Self {
            max_concentration_percent,
        }
    }
}

#[async_trait::async_trait]
impl RiskRule for ConcentrationLimitRule {
    async fn check_order(
        &self,
        order: &NewOrder,
        risk_engine: &RiskEngine,
    ) -> Option<RiskViolation> {
        // Get total exposure
        let total_exposure = risk_engine.get_total_exposure().await;

        if total_exposure.value().is_zero() {
            return None; // No exposure yet, can't calculate concentration
        }

        // Get current position for this symbol
        let current_position = risk_engine.get_position(order.symbol.as_str()).await;

        // Calculate potential new exposure for this symbol
        let symbol_exposure = if let Some(price) = order.price {
            let current_exposure = current_position
                .as_ref()
                .and_then(|p| p.average_price.map(|ap| p.size.value() * ap.value()))
                .unwrap_or(rust_decimal::Decimal::ZERO);

            let order_value = order.size.value() * price.value();
            let new_exposure = match order.side {
                OrderSide::Buy => current_exposure + order_value,
                OrderSide::Sell => current_exposure - order_value,
            };

            new_exposure.abs()
        } else {
            return None; // Can't calculate without price
        };

        // Calculate concentration percentage
        let concentration_percent = symbol_exposure / total_exposure.value();

        if concentration_percent > self.max_concentration_percent {
            return Some(RiskViolation::new(
                "ConcentrationLimit".to_string(),
                format!(
                    "Symbol concentration exceeds limit for {}: current={}%, max={}%",
                    order.symbol,
                    concentration_percent * rust_decimal::Decimal::new(100, 0),
                    self.max_concentration_percent * rust_decimal::Decimal::new(100, 0)
                ),
            ));
        }

        None
    }
}

/// Minimum balance rule
/// Ensures minimum balance is maintained for an asset
pub struct MinimumBalanceRule {
    /// Minimum required balance by asset
    min_balances: HashMap<String, Size>,
}

impl MinimumBalanceRule {
    /// Create a new minimum balance rule
    pub fn new() -> Self {
        Self {
            min_balances: HashMap::new(),
        }
    }

    /// Set minimum balance for an asset
    pub fn set_min_balance(&mut self, asset: &str, min_balance: Size) {
        self.min_balances.insert(asset.to_string(), min_balance);
    }
}

#[async_trait::async_trait]
impl RiskRule for MinimumBalanceRule {
    async fn check_order(
        &self,
        order: &NewOrder,
        risk_engine: &RiskEngine,
    ) -> Option<RiskViolation> {
        // For buy orders, check if we'll have enough balance left
        if order.side == OrderSide::Buy {
            if let Some(price) = order.price {
                // Calculate required balance
                let required_balance = price.value() * order.size.value();

                // Extract quote asset from symbol (e.g., USDT from BTCUSDT)
                // Use as_str() to get string slice for indexing
                let symbol_str = order.symbol.as_str();
                let quote_asset = if symbol_str.len() >= 4 {
                    &symbol_str[symbol_str.len() - 4..] // Last 4 chars (USDT)
                } else {
                    "USDT" // Default
                };

                // Get current balance
                let current_balance = risk_engine.get_balance(quote_asset).await;

                // Get minimum required balance
                let min_balance = self
                    .min_balances
                    .get(quote_asset)
                    .cloned()
                    .unwrap_or(Size::new(rust_decimal::Decimal::ZERO));

                // Check if we'll have enough balance after this order
                let balance_after_order = current_balance.value() - required_balance;

                if balance_after_order < min_balance.value() {
                    return Some(RiskViolation::new(
                        "MinimumBalanceLimit".to_string(),
                        format!(
                            "Order would violate minimum balance for {}: current={}, required={}, min={}, after_order={}",
                            quote_asset, current_balance, required_balance, min_balance, balance_after_order
                        ),
                    ));
                }
            }
        }

        None
    }
}

/// Rate of change limit rule
/// Limits the rate at which position size can change
pub struct RateOfChangeLimitRule {
    /// Maximum position size change per time period
    max_change_per_period: Size,
    /// Time period in seconds
    period_seconds: u64,
    /// Last position sizes by symbol and timestamp
    last_positions: HashMap<String, (Size, std::time::Instant)>,
}

impl RateOfChangeLimitRule {
    /// Create a new rate of change limit rule
    pub fn new(max_change_per_period: Size, period_seconds: u64) -> Self {
        Self {
            max_change_per_period,
            period_seconds,
            last_positions: HashMap::new(),
        }
    }

    /// Update last position for a symbol
    pub fn update_last_position(&mut self, symbol: &str, size: Size) {
        self.last_positions
            .insert(symbol.to_string(), (size, std::time::Instant::now()));
    }
}

#[async_trait::async_trait]
impl RiskRule for RateOfChangeLimitRule {
    async fn check_order(
        &self,
        order: &NewOrder,
        risk_engine: &RiskEngine,
    ) -> Option<RiskViolation> {
        // Get current position
        let current_position = risk_engine.get_position(order.symbol.as_str()).await;
        let current_size = current_position
            .map(|p| p.size)
            .unwrap_or(Size::new(rust_decimal::Decimal::ZERO));

        // Calculate new position size
        let new_size = match order.side {
            OrderSide::Buy => current_size + order.size,
            OrderSide::Sell => current_size - order.size,
        };

        // Check if we have a previous position record
        if let Some((last_size, last_time)) = self.last_positions.get(order.symbol.as_str()) {
            // Check if enough time has passed
            let elapsed = last_time.elapsed().as_secs();
            if elapsed < self.period_seconds {
                // Calculate change
                let change = (new_size.value() - last_size.value()).abs();

                if change > self.max_change_per_period.value() {
                    return Some(RiskViolation::new(
                        "RateOfChangeLimit".to_string(),
                        format!(
                            "Position change rate exceeds limit for {}: change={}, max={}, elapsed={}s",
                            order.symbol, change, self.max_change_per_period, elapsed
                        ),
                    ));
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Symbol;
    use crate::TimeInForce;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_risk_engine_creation() {
        let risk_engine = RiskEngine::new();

        // Verify initial state
        assert_eq!(risk_engine.get_open_orders_count().await, 0);
        assert_eq!(risk_engine.get_max_open_orders().await, 100);
        assert_eq!(
            risk_engine.get_max_total_exposure().await,
            Price::new(rust_decimal::Decimal::MAX)
        );
    }

    #[tokio::test]
    async fn test_position_size_rule() {
        let risk_engine = RiskEngine::new();
        let mut rule = PositionSizeRule::new();

        // Set maximum position size
        risk_engine
            .set_max_position_size("BTCUSDT", Size::from_str("10.0").unwrap())
            .await;
        rule.set_max_position("BTCUSDT", Size::from_str("10.0").unwrap());

        // Add rule to engine
        risk_engine.add_rule(Box::new(rule)).await;

        // Create order that would exceed position limit
        let order = NewOrder::new_limit_buy(
            "BTCUSDT".to_string(),
            Size::from_str("15.0").unwrap(), // Exceeds max position
            Price::from_str("50000.0").unwrap(),
            TimeInForce::GoodTillCancelled,
        );

        // Check should fail
        let result = risk_engine.check_order(&order).await;
        assert!(result.is_err());

        let violation = result.unwrap_err();
        assert_eq!(violation.rule, "PositionSizeLimit");
    }

    #[tokio::test]
    async fn test_order_size_rule() {
        let risk_engine = RiskEngine::new();
        let mut rule = OrderSizeRule::new();

        // Set maximum order size
        risk_engine
            .set_max_order_size("BTCUSDT", Size::from_str("5.0").unwrap())
            .await;
        rule.set_max_order("BTCUSDT", Size::from_str("5.0").unwrap());

        // Add rule to engine
        risk_engine.add_rule(Box::new(rule)).await;

        // Create order that exceeds size limit
        let order = NewOrder::new_limit_buy(
            "BTCUSDT".to_string(),
            Size::from_str("10.0").unwrap(), // Exceeds max order size
            Price::from_str("50000.0").unwrap(),
            TimeInForce::GoodTillCancelled,
        );

        // Check should fail
        let result = risk_engine.check_order(&order).await;
        assert!(result.is_err());

        let violation = result.unwrap_err();
        assert_eq!(violation.rule, "OrderSizeLimit");
    }

    #[tokio::test]
    async fn test_daily_loss_rule() {
        let risk_engine = RiskEngine::new();
        let mut rule = DailyLossRule::new();

        // Set maximum daily loss
        risk_engine
            .set_max_daily_loss("BTCUSDT", Price::from_str("1000.0").unwrap())
            .await;
        rule.set_max_loss("BTCUSDT", Price::from_str("1000.0").unwrap());

        // Add rule to engine
        risk_engine.add_rule(Box::new(rule)).await;

        // Set up a position that would result in a loss
        let position = Position {
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "binance".to_string(),
            size: Size::from_str("1.0").unwrap(),
            average_price: Some(Price::from_str("50000.0").unwrap()),
            unrealized_pnl: Some(rust_decimal::Decimal::from_str("-500.0").unwrap()),
        };

        risk_engine.update_position("BTCUSDT", position).await;

        // Create sell order that would exceed daily loss limit
        let order = NewOrder::new_limit_sell(
            "BTCUSDT".to_string(),
            Size::from_str("1.0").unwrap(),
            Price::from_str("49000.0").unwrap(), // $1000 loss
            TimeInForce::GoodTillCancelled,
        );

        // Check should fail
        let result = risk_engine.check_order(&order).await;
        assert!(result.is_err());

        let violation = result.unwrap_err();
        assert_eq!(violation.rule, "DailyLossLimit");
    }

    #[tokio::test]
    async fn test_total_exposure_rule() {
        let risk_engine = RiskEngine::new();
        let rule = TotalExposureRule::new(Price::from_str("100000.0").unwrap());

        // Add rule to engine
        risk_engine.add_rule(Box::new(rule)).await;

        // Set up a position with high exposure
        let position = Position {
            symbol: Symbol::new("BTCUSDT"),
            exchange_id: "binance".to_string(),
            size: Size::from_str("2.0").unwrap(),
            average_price: Some(Price::from_str("50000.0").unwrap()),
            unrealized_pnl: Some(rust_decimal::Decimal::ZERO),
        };

        risk_engine.update_position("BTCUSDT", position).await;

        // Create buy order that would exceed total exposure limit
        let order = NewOrder::new_limit_buy(
            "BTCUSDT".to_string(),
            Size::from_str("1.0").unwrap(),
            Price::from_str("50000.0").unwrap(),
            TimeInForce::GoodTillCancelled,
        );

        // Check should fail
        let result = risk_engine.check_order(&order).await;
        assert!(result.is_err());

        let violation = result.unwrap_err();
        assert_eq!(violation.rule, "TotalExposureLimit");
    }

    #[tokio::test]
    async fn test_open_orders_count_rule() {
        let risk_engine = RiskEngine::new();
        let rule = OpenOrdersCountRule::new(2);

        // Add rule to engine
        risk_engine.add_rule(Box::new(rule)).await;

        // Set open orders count to limit
        risk_engine.increment_open_orders().await;
        risk_engine.increment_open_orders().await;

        assert_eq!(risk_engine.get_open_orders_count().await, 2);

        // Create order that would exceed open orders limit
        let order = NewOrder::new_limit_buy(
            "BTCUSDT".to_string(),
            Size::from_str("1.0").unwrap(),
            Price::from_str("50000.0").unwrap(),
            TimeInForce::GoodTillCancelled,
        );

        // Check should fail
        let result = risk_engine.check_order(&order).await;
        assert!(result.is_err());

        let violation = result.unwrap_err();
        assert_eq!(violation.rule, "OpenOrdersCountLimit");
    }

    #[tokio::test]
    async fn test_balance_rule() {
        let risk_engine = RiskEngine::new();
        let mut rule = BalanceRule::new();

        // Set minimum balance
        risk_engine
            .update_balance("BTC", Size::from_str("2.0").unwrap())
            .await;
        rule.set_min_balance("BTC", Size::from_str("1.0").unwrap());

        // Add rule to engine
        risk_engine.add_rule(Box::new(rule)).await;

        // Create buy order that requires more BTC than available
        let order = NewOrder::new_limit_buy(
            "BTCUSDT".to_string(),
            Size::from_str("2.0").unwrap(), // Requires 2 BTC but only 1 available
            Price::from_str("50000.0").unwrap(),
            TimeInForce::GoodTillCancelled,
        );

        // Check should fail
        let result = risk_engine.check_order(&order).await;
        assert!(result.is_err());

        let violation = result.unwrap_err();
        assert_eq!(violation.rule, "InsufficientBalance");
    }

    #[tokio::test]
    async fn test_multiple_rules() {
        let risk_engine = RiskEngine::new();

        // Add multiple rules
        let mut position_rule = PositionSizeRule::new();
        position_rule.set_max_position("BTCUSDT", Size::from_str("10.0").unwrap());
        risk_engine.add_rule(Box::new(position_rule)).await;

        let mut order_size_rule = OrderSizeRule::new();
        order_size_rule.set_max_order("BTCUSDT", Size::from_str("5.0").unwrap());
        risk_engine.add_rule(Box::new(order_size_rule)).await;

        // Create order that passes position rule but fails order size rule
        let order = NewOrder::new_limit_buy(
            "BTCUSDT".to_string(),
            Size::from_str("7.0").unwrap(), // Exceeds max order size but not max position
            Price::from_str("50000.0").unwrap(),
            TimeInForce::GoodTillCancelled,
        );

        // Check should fail due to order size rule
        let result = risk_engine.check_order(&order).await;
        assert!(result.is_err());

        let violation = result.unwrap_err();
        assert_eq!(violation.rule, "OrderSizeLimit");

        // Create order that passes both rules
        let order = NewOrder::new_limit_buy(
            "BTCUSDT".to_string(),
            Size::from_str("3.0").unwrap(), // Passes both rules
            Price::from_str("50000.0").unwrap(),
            TimeInForce::GoodTillCancelled,
        );

        // Check should pass
        let result = risk_engine.check_order(&order).await;
        assert!(result.is_ok());
    }
}
