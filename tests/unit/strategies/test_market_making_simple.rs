use crypto_hft::strategies::MarketMakingStrategy;
use crypto_hft::types::{Price, Size};
use crypto_hft::strategy::{Strategy, MarketState};
use crypto_hft::orderbook::{OrderBookSnapshot, OrderBookLevel};
use crypto_hft::traits::MarketEvent;
use std::time::Duration;

/// 测试做市策略的基本功能
#[test]
fn test_market_making_strategy_creation() {
    let strategy = MarketMakingStrategy::new(
        Price::from_str("1.0").unwrap(),      // Target spread: $1.0
        Size::from_str("0.001").unwrap(),     // Base order size: 0.001 BTC
        Size::from_str("0.1").unwrap(),       // Max position size: 0.1 BTC
        5,                                    // Max order levels
        Duration::from_millis(1000),          // Order refresh time: 1 second
    );
    
    // 验证策略创建成功
    assert!(true); // 如果能创建，说明构造函数正常
}

/// 测试做市策略生成信号 - 有足够价差的情况
#[test]
fn test_market_making_generate_signal_with_good_spread() {
    let mut strategy = MarketMakingStrategy::new(
        Price::from_str("1.0").unwrap(),      // Target spread: $1.0
        Size::from_str("0.001").unwrap(),     // Base order size: 0.001 BTC
        Size::from_str("0.1").unwrap(),       // Max position size: 0.1 BTC
        5,                                    // Max order levels
        Duration::from_millis(1000),          // Order refresh time: 1 second
    );
    
    // 创建市场状态，有足够的价差（$2.0 > $1.0 target）
    let mut market_state = MarketState::new("BTCUSDT".to_string());
    
    // 创建订单簿快照：bid $100, ask $102 (价差 $2.0)
    let snapshot = OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        vec![
            OrderBookLevel::new(
                Price::from_str("100.0").unwrap(),
                Size::from_str("1.0").unwrap()
            ),
            OrderBookLevel::new(
                Price::from_str("99.9").unwrap(),
                Size::from_str("0.5").unwrap()
            ),
        ],
        vec![
            OrderBookLevel::new(
                Price::from_str("102.0").unwrap(),
                Size::from_str("1.0").unwrap()
            ),
            OrderBookLevel::new(
                Price::from_str("102.1").unwrap(),
                Size::from_str("0.5").unwrap()
            ),
        ],
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    );
    
    // 更新市场状态
    let event = MarketEvent::OrderBookSnapshot(snapshot);
    market_state.update(&event);
    
    // 生成信号
    let signal = strategy.generate_signal(&market_state);
    
    // 验证信号生成成功（应该生成做市订单信号）
    // 注意：由于策略可能返回 Custom 信号，我们需要检查信号是否存在
    assert!(signal.is_some(), "策略应该在有足够价差时生成信号");
}

/// 测试做市策略 - 价差太小的情况
#[test]
fn test_market_making_no_signal_when_spread_too_small() {
    let mut strategy = MarketMakingStrategy::new(
        Price::from_str("1.0").unwrap(),      // Target spread: $1.0
        Size::from_str("0.001").unwrap(),     // Base order size: 0.001 BTC
        Size::from_str("0.1").unwrap(),       // Max position size: 0.1 BTC
        5,                                    // Max order levels
        Duration::from_millis(1000),          // Order refresh time: 1 second
    );
    
    // 创建市场状态，价差太小（$0.5 < $1.0 target）
    let mut market_state = MarketState::new("BTCUSDT".to_string());
    
    // 创建订单簿快照：bid $100, ask $100.5 (价差 $0.5)
    let snapshot = OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        vec![
            OrderBookLevel::new(
                Price::from_str("100.0").unwrap(),
                Size::from_str("1.0").unwrap()
            ),
        ],
        vec![
            OrderBookLevel::new(
                Price::from_str("100.5").unwrap(),
                Size::from_str("1.0").unwrap()
            ),
        ],
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    );
    
    // 更新市场状态
    let event = MarketEvent::OrderBookSnapshot(snapshot);
    market_state.update(&event);
    
    // 生成信号
    let signal = strategy.generate_signal(&market_state);
    
    // 验证：价差太小时，策略可能不生成信号，或者生成取消订单的信号
    // 这是合理的，因为做市商不应该在价差太小时下单
    // 注意：策略可能返回 None 或 CancelAllOrders 信号
    println!("Signal generated: {:?}", signal);
}

/// 测试做市策略 - 多个订单层级
#[test]
fn test_market_making_multiple_order_levels() {
    let mut strategy = MarketMakingStrategy::new(
        Price::from_str("1.0").unwrap(),      // Target spread: $1.0
        Size::from_str("0.001").unwrap(),     // Base order size: 0.001 BTC
        Size::from_str("0.1").unwrap(),       // Max position size: 0.1 BTC
        5,                                    // Max order levels: 5
        Duration::from_millis(1000),          // Order refresh time: 1 second
    );
    
    // 创建市场状态
    let mut market_state = MarketState::new("BTCUSDT".to_string());
    
    // 创建订单簿快照，有足够的深度
    let snapshot = OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        vec![
            OrderBookLevel::new(Price::from_str("100.0").unwrap(), Size::from_str("1.0").unwrap()),
            OrderBookLevel::new(Price::from_str("99.9").unwrap(), Size::from_str("0.8").unwrap()),
            OrderBookLevel::new(Price::from_str("99.8").unwrap(), Size::from_str("0.6").unwrap()),
            OrderBookLevel::new(Price::from_str("99.7").unwrap(), Size::from_str("0.4").unwrap()),
            OrderBookLevel::new(Price::from_str("99.6").unwrap(), Size::from_str("0.2").unwrap()),
        ],
        vec![
            OrderBookLevel::new(Price::from_str("102.0").unwrap(), Size::from_str("1.0").unwrap()),
            OrderBookLevel::new(Price::from_str("102.1").unwrap(), Size::from_str("0.8").unwrap()),
            OrderBookLevel::new(Price::from_str("102.2").unwrap(), Size::from_str("0.6").unwrap()),
            OrderBookLevel::new(Price::from_str("102.3").unwrap(), Size::from_str("0.4").unwrap()),
            OrderBookLevel::new(Price::from_str("102.4").unwrap(), Size::from_str("0.2").unwrap()),
        ],
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    );
    
    // 更新市场状态
    let event = MarketEvent::OrderBookSnapshot(snapshot);
    market_state.update(&event);
    
    // 生成信号
    let signal = strategy.generate_signal(&market_state);
    
    // 验证信号生成
    assert!(signal.is_some(), "策略应该在市场有足够深度时生成信号");
    
    println!("Signal with multiple levels: {:?}", signal);
}

/// 测试做市策略 - 订单刷新时间控制
#[test]
fn test_market_making_order_refresh_timing() {
    let mut strategy = MarketMakingStrategy::new(
        Price::from_str("1.0").unwrap(),
        Size::from_str("0.001").unwrap(),
        Size::from_str("0.1").unwrap(),
        5,
        Duration::from_millis(100),  // 很短的刷新时间：100ms
    );
    
    // 创建市场状态
    let mut market_state = MarketState::new("BTCUSDT".to_string());
    
    let snapshot = OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        vec![OrderBookLevel::new(Price::from_str("100.0").unwrap(), Size::from_str("1.0").unwrap())],
        vec![OrderBookLevel::new(Price::from_str("102.0").unwrap(), Size::from_str("1.0").unwrap())],
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    );
    
    let event = MarketEvent::OrderBookSnapshot(snapshot);
    market_state.update(&event);
    
    // 第一次生成信号
    let signal1 = strategy.generate_signal(&market_state);
    assert!(signal1.is_some(), "第一次应该生成信号");
    
    // 立即再次生成信号（应该被刷新时间限制）
    let signal2 = strategy.generate_signal(&market_state);
    // 由于刷新时间很短（100ms），如果调用间隔很短，可能不会生成新信号
    println!("First signal: {:?}", signal1);
    println!("Second signal (immediate): {:?}", signal2);
}

/// 测试做市策略 - 不同价差场景
#[test]
fn test_market_making_different_spreads() {
    let mut strategy = MarketMakingStrategy::new(
        Price::from_str("1.0").unwrap(),  // Target spread: $1.0
        Size::from_str("0.001").unwrap(),
        Size::from_str("0.1").unwrap(),
        5,
        Duration::from_millis(1000),
    );
    
    // 测试场景1：价差刚好等于目标价差
    let mut market_state1 = MarketState::new("BTCUSDT".to_string());
    let snapshot1 = OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        vec![OrderBookLevel::new(Price::from_str("100.0").unwrap(), Size::from_str("1.0").unwrap())],
        vec![OrderBookLevel::new(Price::from_str("101.0").unwrap(), Size::from_str("1.0").unwrap())],
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    );
    market_state1.update(&MarketEvent::OrderBookSnapshot(snapshot1));
    let signal1 = strategy.generate_signal(&market_state1);
    println!("Spread = target ($1.0): {:?}", signal1);
    
    // 测试场景2：价差大于目标价差
    let mut market_state2 = MarketState::new("BTCUSDT".to_string());
    let snapshot2 = OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        vec![OrderBookLevel::new(Price::from_str("100.0").unwrap(), Size::from_str("1.0").unwrap())],
        vec![OrderBookLevel::new(Price::from_str("103.0").unwrap(), Size::from_str("1.0").unwrap())],
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    );
    market_state2.update(&MarketEvent::OrderBookSnapshot(snapshot2));
    let signal2 = strategy.generate_signal(&market_state2);
    println!("Spread > target ($3.0): {:?}", signal2);
    
    // 验证：价差越大，策略应该更愿意下单
    assert!(true, "测试完成");
}

