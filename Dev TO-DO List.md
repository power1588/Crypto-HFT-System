这是一个基于 **TDD (测试驱动开发)** 模式深度细化后的开发待办文档。

为了确保**高可扩展性**和**高性能**，我们将采用 **Trait-Driven Design (接口驱动设计)**。在编写任何具体实现（如 Binance 连接器）之前，先定义接口和测试用例。这能强制解耦，并允许我们在不接触真实交易所的情况下验证核心逻辑。

---

# Crypto HFT System - TDD & High-Performance Roadmap

**核心原则：**
1.  **Red (红)**: 编写一个失败的测试（定义期望行为）。
2.  **Green (绿)**: 编写最简单的代码通过测试。
3.  **Refactor (重构)**: 优化代码结构和性能（消除 `clone`, 引入 Zero-copy, 使用 SIMD）。
4.  **Type Safety**: 利用 Rust 类型系统（NewType Pattern）防止逻辑错误。

---

## Phase 1: 核心领域模型 (Core Domain) - The Foundation

**目标**: 定义系统的通用语言，确保无 IO 依赖，纯内存操作。

### 1.1 类型安全基础 (Type Safety)
*   **Context**: 防止将"价格"和"数量"混淆相加。
*   **TDD Cycle**:
    *   [x] **Red**: 编写测试，试图将 `Price` 和 `Size` 相加，编译器应报错；测试 `Price` 的序列化精度。
    *   [x] **Green**: 使用 `rust_decimal` 并通过 `NewType` 模式封装：
        ```rust
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub struct Price(pub Decimal);
        pub struct Size(pub Decimal);
        ```
    *   [x] **Refactor**: 为 `Price` 实现 `serde::Deserialize`，确保能从字符串高效解析，避免浮点误差。

**实际实现状态**: ✅ 已完成
- 实现了 `Price` 和 `Size` 类型，使用 NewType 模式确保类型安全
- 添加了完整的序列化/反序列化支持
- 实现了算术运算，防止不同类型间的错误操作
- 添加了全面的单元测试

### 1.2 订单簿逻辑 (OrderBook Logic)
*   **Context**: 核心数据结构，要求极高的更新速度。
*   **TDD Cycle**:
    *   [x] **Red**: 定义 `OrderBook` 结构。编写测试 `apply_snapshot` 和 `apply_delta`。
        *   Case 1: 接收 Snapshot，最好的 Bid 应该是 X。
        *   Case 2: 接收 Delta (Ask 价格 P 数量 0)，该档位应从 Book 中移除。
        *   Case 3: 接收 Delta (Bid 价格 P 数量 N)，该档位应更新或插入。
    *   [x] **Green**: 实现基于 `BTreeMap<Price, Size>` 的基础版本。
    *   [x] **Refactor (High Performance)**:
        *   引入 `SmallVec` 优化 `bids/asks` 列表（假设 HFT 只关心前 20 档，分配在栈上）。
        *   如果是极高频，考虑替换 `BTreeMap` 为固定数组（Fixed Array）+ 游标，消除堆分配。

**实际实现状态**: ✅ 已完成
- 实现了基于 `BTreeMap` 的 `OrderBook` 结构
- 添加了 `apply_snapshot` 和 `apply_delta` 方法
- 使用 `SmallVec` 优化 `top_bids` 和 `top_asks` 方法
- 实现了完整的单元测试覆盖所有功能
- 添加了性能基准测试，测试不同规模下的订单簿操作

---

## Phase 2: 抽象与模拟 (Abstraction & Mocking) - The Glue

**目标**: 定义各模块交互的 `Trait`，利用 `mockall` 库进行交互测试。

### 2.1 定义核心 Traits
*   **Context**: 策略层不应知道它是连接的 Binance 还是 OKX。
*   **Tasks**:
    *   [x] 定义 `MarketDataStream`:
        ```rust
        #[async_trait]
        pub trait MarketDataStream {
            async fn subscribe(&mut self, symbols: &[&str]) -> Result<()>;
            async fn next(&mut self) -> Option<MarketEvent>;
        }
        ```
    *   [x] 定义 `ExecutionClient`:
        ```rust
        #[async_trait]
        pub trait ExecutionClient {
            async fn place_order(&self, order: NewOrder) -> Result<OrderId>;
            async fn cancel_order(&self, order_id: OrderId) -> Result<()>;
        }
        ```

**实际实现状态**: ✅ 已完成
- 定义了完整的 `MarketDataStream` 和 `ExecutionClient` traits
- 添加了 `MarketDataHistory` trait 用于历史数据访问
- 实现了 `MockMarketDataStream` 和 `MockExecutionClient` 用于测试
- 添加了所有必要的事件类型和订单状态定义

### 2.2 消息解析性能测试
*   **Context**: 交易所数据解析是 CPU 密集型操作。
*   **TDD Cycle**:
    *   [x] **Red**: 准备一段真实的 Binance Depth Update JSON 字符串。编写 Benchmark 测试 (`criterion` crate)，断言解析时间 < 10us。
    *   [x] **Green**: 使用 `serde_json::from_str` 实现标准解析。
    *   [ ] **Refactor**: 引入 `simd-json`。将输入改为 `&mut [u8]` 以启用原地解析（In-place parsing），消除 String Allocation。对比 Benchmark 提升幅度。

**实际实现状态**: 🟡 部分完成
- 实现了 `BinanceMessage` 解析，支持深度更新和交易消息
- 添加了消息解析的性能基准测试
- 已添加 `simd-json` 依赖，但尚未实现 SIMD 优化版本
- 当前使用标准 `serde_json` 进行解析

---

## Phase 3: 策略引擎 (Strategy Engine) - Pure Logic

**目标**: 确保策略行为是确定性的（Deterministic）。策略层不包含任何 `async` IO，只处理状态变化。

### 3.1 策略状态机测试
*   **Context**: 给定一个行情输入，策略必须输出确定的信号。
*   **TDD Cycle**:
    *   [x] **Red (Fixture)**:
        *   输入: Binance OrderBook (Bid: 100), OKX OrderBook (Ask: 99).
        *   输入: 资金充足。
        *   期望: 收到 `Signal::Arbitrage { buy: "OKX", sell: "Binance" }`。
    *   [x] **Green**: 实现简单的 `fn on_tick(&mut self, market: &MarketState) -> Vec<Signal>`。
    *   [x] **Refactor**: 优化锁机制。策略内部不应有 `Mutex`。数据通过 `Channel` 进入策略线程，策略线程独占数据所有权（Single Writer Principle）。

**实际实现状态**: ✅ 已完成
- 实现了 `StrategyEngine` 和 `Strategy` trait
- 添加了 `MarketState` 结构用于跟踪市场状态
- 实现了 `SimpleArbitrageStrategy` 作为示例策略
- 添加了信号去抖和冷却机制
- 实现了 `EventDrivenStrategy` 包装器，处理事件驱动的策略执行

### 3.2 信号去抖与冷却 (Debounce)
*   **Context**: 防止网络抖动导致重复下单。
*   **TDD Cycle**:
    *   [x] **Red**: 模拟连续 10 个 tick 满足套利条件。断言：只产生 1 个 Signal，后续 9 个因冷却时间被丢弃。
    *   [x] **Green**: 在策略结构体中引入 `last_signal_ts` 字段进行判断。

**实际实现状态**: ✅ 已完成
- 在 `StrategyEngine` 中实现了信号冷却机制
- 添加了可配置的冷却时间
- 实现了基于时间戳的信号去抖逻辑
- 添加了相应的单元测试验证功能

---

## Phase 4: 风控系统 (Risk Management) - The Gatekeeper

**目标**: 风控必须是**同步**且**无阻塞**的。

### 4.1 静态规则检查
*   **TDD Cycle**:
    *   [x] **Red**: 构造一个金额为 1,000,000 USDT 的订单。调用 `RiskEngine::check(order)`，断言返回 `Err(RiskViolation::ExceedsLimit)`。
    *   [x] **Green**: 实现简单的阈值检查。

**实际实现状态**: ✅ 已完成
- 实现了 `RiskEngine` 和 `RiskRule` trait
- 添加了 `MaxOrderSizeRule` 和 `MaxOrderValueRule` 作为示例规则
- 实现了 `RiskViolation` 枚举，定义了各种违规类型
- 添加了完整的单元测试验证风控规则

### 4.2 影子账本 (Shadow Ledger)
*   **Context**: 我们不能每次下单都去查询交易所余额（太慢）。
*   **TDD Cycle**:
    *   [x] **Red**:
        *   初始余额: 1000 USDT.
        *   Action: 下单买入花费 100 USDT。
        *   Check: 再次下单 950 USDT，断言失败（可用余额不足）。
    *   [x] **Green**: 在本地内存维护 `Inventory` 结构，下单时扣减 `frozen`，成交/取消时回滚。

**实际实现状态**: ✅ 已完成
- 实现了 `ShadowLedger` 和 `Inventory` 结构
- 添加了余额冻结/解冻机制
- 实现了执行报告处理，自动更新余额
- 添加了完整的单元测试验证影子账本功能

---

## Phase 5: 交易所适配器 (Connectors) - The I/O Layer

**目标**: 处理脏活累活（断连、鉴权），但对外提供干净的数据流。

### 5.1 Binance WebSocket 集成
*   **Context**: 真实网络测试不可靠，需使用 Mock Server。
*   **TDD Cycle**:
    *   [x] **Red**: 使用 `warp` 或 `tokio-tungstenite` 启动一个本地 Mock WS Server。
        *   Mock Server 发送: `{"e": "depthUpdate", ...}`
        *   Client 行为: 连接 Mock Server，断言收到的 Channel 消息已转换为标准 `OrderBookUpdate`。
    *   [x] **Green**: 实现 Binance Connector，连接到 `ws://127.0.0.1:xxxx`。
    *   [ ] **Refactor**: 增加重连逻辑测试。关闭 Mock Server，等待 1s 重启，断言 Client 自动重连成功。

**实际实现状态**: 🟡 部分完成
- 实现了 `BinanceMessage` 解析，支持深度更新和交易消息
- 添加了消息到 `MarketEvent` 的转换
- 实现了 `MockMarketDataStream` 用于测试
- 尚未实现实际的 WebSocket 连接和重连逻辑

### 5.2 签名与 REST API (Wiremock)
*   **Context**: 测试下单签名逻辑是否正确。
*   **TDD Cycle**:
    *   [ ] **Red**: 使用 `wiremock` crate 模拟 Binance 下单接口 `POST /api/v3/order`。
    *   [ ] **Green**: 实现 HMAC-SHA256 签名。发送请求。
    *   [ ] **Verify**: 检查 Mock Server 收到的 Header 中 `X-MBX-APIKEY` 是否存在，Query Param 中 `signature` 是否符合预期。

**实际实现状态**: 🔴 未开始
- 尚未实现 REST API 连接器
- 尚未实现签名逻辑
- 尚未添加 `wiremock` 测试

---

## Phase 6: 订单管理系统 (OMS) - Lifecycle Management

### 6.1 订单状态转换
*   **TDD Cycle**:
    *   [x] **Red**:
        *   创建订单 -> 状态 `New`。
        *   模拟收到 WS 推送 `ExecutionReport (PartiallyFilled)` -> 状态变更为 `PartiallyFilled`。
        *   模拟收到 WS 推送 `ExecutionReport (Filled)` -> 状态变更为 `Filled`，并归档。
    *   [x] **Green**: 实现 `OrderManager` 结构体，维护 `HashMap<ClientOrderId, OrderState>`。

**实际实现状态**: ✅ 已完成
- 实现了 `OrderManagerImpl` 和 `OrderManager` trait
- 添加了完整的订单状态跟踪
- 实现了订单生命周期管理
- 添加了按符号查询和历史记录功能
- 实现了完整的单元测试验证订单状态转换

### 6.2 速率限制 (Rate Limiting)
*   **TDD Cycle**:
    *   [x] **Red**: 配置限频 10 req/s。循环发送 20 个请求。断言前 10 个成功，第 11 个需等待或被拒绝（取决于策略，HFT 通常选择丢弃或排队）。
    *   [x] **Green**: 集成 `governor` crate 实现 Token Bucket 算法。

**实际实现状态**: ✅ 已完成
- 实现了 `RateLimiter` 结构，使用滑动窗口算法
- 添加了按符号的速率限制
- 实现了当前速率查询和下次可用时间计算
- 添加了完整的单元测试验证速率限制功能

---

## Phase 7: 集成测试 (Integration) - The End-to-End Flow

**目标**: 将所有 Mock 串联起来。

*   **Scenario**:
    1.  启动 `MockBinance` 和 `MockOkx` (WS + REST)。
    2.  启动 `Core System`。
    3.  向 Mock WS 推送制造价差的数据。
    4.  **断言**: Mock REST Server 收到了正确的下单请求（Buy Low, Sell High）。
    5.  向 Mock WS 推送成交回报。
    6.  **断言**: 系统内部 Log 显示套利成功，Shadow Ledger 余额更新。

**实际实现状态**: 🔴 未开始
- 尚未实现端到端集成测试
- 需要实现完整的系统集成测试场景

---

## 开发优先级排序 (Priority Queue)

1.  **P0**: `Core` 类型定义与 `OrderBook` 逻辑 (纯算法，基石) - ✅ 已完成
2.  **P0**: `Strategy` 简单套利逻辑测试 (核心价值) - ✅ 已完成
3.  **P1**: `Connectors` 的 WebSocket 解析与标准化 (数据源) - 🟡 部分完成
4.  **P1**: `OMS` 的基础下单与签名 (执行能力) - 🟡 部分完成
5.  **P2**: `Risk` 基础检查 (安全) - ✅ 已完成
6.  **P3**: 真实交易所对接调试 - 🔴 未开始

## 关键 Rust Crates 推荐 (Refined)

*   **Testing**: `mockall` (Mocking), `proptest` (Property-based), `wiremock` (HTTP Mock), `criterion` (Benchmark) - ✅ 已集成
*   **Async Runtime**: `tokio` (standard), `tokio-console` (调试异步死锁/性能瓶颈) - ✅ 已集成
*   **Fast Parsing**: `simd-json` (Critical for WS) - 🟡 已添加依赖但未实现
*   **Data Structures**: `smallvec` (Stack allocation), `dashmap` (Concurrent Map for OMS, though prefer message passing) - ✅ 已集成

## 当前项目状态总结

### 已完成模块 (100%)
1. **类型系统** - 完整的类型安全实现，包括 Price、Size 类型
2. **订单簿** - 高性能订单簿实现，支持快照和增量更新
3. **核心 Traits** - 完整的抽象接口定义
4. **策略引擎** - 事件驱动的策略执行框架
5. **风控系统** - 包括规则引擎和影子账本
6. **订单管理** - 完整的订单生命周期管理和速率限制

### 部分完成模块 (50-80%)
1. **连接器** - 消息解析已完成，但 WebSocket 连接和 REST API 尚未实现
2. **性能优化** - 基准测试已添加，但 SIMD 优化尚未实现

### 未开始模块 (0%)
1. **集成测试** - 端到端系统测试尚未实现
2. **真实交易所对接** - 实际的交易所连接尚未实现

### 下一步建议
1. 完成 WebSocket 连接器的实现
2. 添加 REST API 支持和签名逻辑
3. 实现 SIMD 优化的消息解析
4. 设计并实现端到端集成测试
5. 开始真实交易所的对接工作

通过严格遵循这个 TDD 流程，你将构建出一个**即使在没有真实网络连接时也能通过 100% 测试**的系统，上线时的 Bug 率将极低。