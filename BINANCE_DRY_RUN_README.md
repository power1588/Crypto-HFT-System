# Binance BTCUSDT 做市策略 Dry-Run 模式

## 功能说明

这个程序实现了以下功能:

1. **接入Binance BTCUSDT现货实时公开市场行情**
   - 使用Binance WebSocket API获取实时订单簿数据
   - 不需要API密钥(使用公开数据流)

2. **Dry-Run模式运行做市策略**
   - 策略会生成买卖订单报价
   - 订单不会实际发送到交易所,只会在命令行打印

3. **实时行情和策略报价显示**
   - 在命令行实时显示订单簿快照和更新
   - 显示策略生成的买卖订单报价

## 使用方法

### 编译

```bash
cargo build --bin binance_dry_run --release
```

### 运行

```bash
cargo run --bin binance_dry_run
```

或者直接运行编译后的二进制文件:

```bash
./target/release/binance_dry_run
```

## 输出示例

程序会在命令行显示:

1. **订单簿快照/更新** - 显示最佳买卖价、价差等
2. **策略订单报价** - 显示策略生成的买卖订单(模拟下单,不会实际执行)

## 配置

可以在 `src/binance_dry_run.rs` 中修改策略参数:

- `target_spread`: 目标价差 (默认: $1.0)
- `base_order_size`: 基础订单大小 (默认: 0.001 BTC)
- `max_position_size`: 最大持仓大小 (默认: 0.1 BTC)
- `max_order_levels`: 最大订单层级 (默认: 5)
- `order_refresh_time`: 订单刷新时间 (默认: 1秒)

## 注意事项

1. 这是一个dry-run模式,不会实际下单
2. 需要网络连接以访问Binance WebSocket
3. 程序会持续运行直到手动停止(Ctrl+C)

## 依赖

程序需要以下依赖(已在Cargo.toml中配置):

- `reqwest` - HTTP客户端
- `tokio-tungstenite` - WebSocket客户端
- `futures-util` - 异步工具
- `hmac`, `sha2`, `base64` - 加密相关(用于API签名,公开数据流不需要)

