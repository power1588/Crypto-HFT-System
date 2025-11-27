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
*   **Context**: 防止将“价格”和“数量”混淆相加。
*   **TDD Cycle**:
    *   [ ] **Red**: 编写测试，试图将 `Price` 和 `Size` 相加，编译器应报错；测试 `Price` 的序列化精度。
    *   [ ] **Green**: 使用 `rust_decimal` 并通过 `NewType` 模式封装：
        ```rust
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub struct Price(pub Decimal);
        pub struct Size(pub Decimal);
        ```
    *   [ ] **Refactor**: 为 `Price` 实现 `serde::Deserialize`，确保能从字符串高效解析，避免浮点误差。

### 1.2 订单簿逻辑 (OrderBook Logic)
*   **Context**: 核心数据结构，要求极高的更新速度。
*   **TDD Cycle**:
    *   [ ] **Red**: 定义 `OrderBook` 结构。编写测试 `apply_snapshot` 和 `apply_delta`。
        *   Case 1: 接收 Snapshot，最好的 Bid 应该是 X。
        *   Case 2: 接收 Delta (Ask 价格 P 数量 0)，该档位应从 Book 中移除。
        *   Case 3: 接收 Delta (Bid 价格 P 数量 N)，该档位应更新或插入。
    *   [ ] **Green**: 实现基于 `BTreeMap<Price, Size>` 的基础版本。
    *   [ ] **Refactor (High Performance)**:
        *   引入 `SmallVec` 优化 `bids/asks` 列表（假设 HFT 只关心前 20 档，分配在栈上）。
        *   如果是极高频，考虑替换 `BTreeMap` 为固定数组（Fixed Array）+ 游标，消除堆分配。

---

## Phase 2: 抽象与模拟 (Abstraction & Mocking) - The Glue

**目标**: 定义各模块交互的 `Trait`，利用 `mockall` 库进行交互测试。

### 2.1 定义核心 Traits
*   **Context**: 策略层不应知道它是连接的 Binance 还是 OKX。
*   **Tasks**:
    *   [ ] 定义 `MarketDataStream`:
        ```rust
        #[async_trait]
        pub trait MarketDataStream {
            async fn subscribe(&mut self, symbols: &[&str]) -> Result<()>;
            async fn next(&mut self) -> Option<MarketEvent>;
        }
        ```
    *   [ ] 定义 `ExecutionClient`:
        ```rust
        #[async_trait]
        pub trait ExecutionClient {
            async fn place_order(&self, order: NewOrder) -> Result<OrderId>;
            async fn cancel_order(&self, order_id: OrderId) -> Result<()>;
        }
        ```

### 2.2 消息解析性能测试
*   **Context**: 交易所数据解析是 CPU 密集型操作。
*   **TDD Cycle**:
    *   [ ] **Red**: 准备一段真实的 Binance Depth Update JSON 字符串。编写 Benchmark 测试 (`criterion` crate)，断言解析时间 < 10us。
    *   [ ] **Green**: 使用 `serde_json::from_str` 实现标准解析。
    *   [ ] **Refactor**: 引入 `simd-json`。将输入改为 `&mut [u8]` 以启用原地解析（In-place parsing），消除 String Allocation。对比 Benchmark 提升幅度。

---

## Phase 3: 策略引擎 (Strategy Engine) - Pure Logic

**目标**: 确保策略行为是确定性的（Deterministic）。策略层不包含任何 `async` IO，只处理状态变化。

### 3.1 策略状态机测试
*   **Context**: 给定一个行情输入，策略必须输出确定的信号。
*   **TDD Cycle**:
    *   [ ] **Red (Fixture)**:
        *   输入: Binance OrderBook (Bid: 100), OKX OrderBook (Ask: 99).
        *   输入: 资金充足。
        *   期望: 收到 `Signal::Arbitrage { buy: "OKX", sell: "Binance" }`。
    *   [ ] **Green**: 实现简单的 `fn on_tick(&mut self, market: &MarketState) -> Vec<Signal>`。
    *   [ ] **Refactor**: 优化锁机制。策略内部不应有 `Mutex`。数据通过 `Channel` 进入策略线程，策略线程独占数据所有权（Single Writer Principle）。

### 3.2 信号去抖与冷却 (Debounce)
*   **Context**: 防止网络抖动导致重复下单。
*   **TDD Cycle**:
    *   [ ] **Red**: 模拟连续 10 个 tick 满足套利条件。断言：只产生 1 个 Signal，后续 9 个因冷却时间被丢弃。
    *   [ ] **Green**: 在策略结构体中引入 `last_signal_ts` 字段进行判断。

---

## Phase 4: 风控系统 (Risk Management) - The Gatekeeper

**目标**: 风控必须是**同步**且**无阻塞**的。

### 4.1 静态规则检查
*   **TDD Cycle**:
    *   [ ] **Red**: 构造一个金额为 1,000,000 USDT 的订单。调用 `RiskEngine::check(order)`，断言返回 `Err(RiskViolation::ExceedsLimit)`。
    *   [ ] **Green**: 实现简单的阈值检查。

### 4.2 影子账本 (Shadow Ledger)
*   **Context**: 我们不能每次下单都去查询交易所余额（太慢）。
*   **TDD Cycle**:
    *   [ ] **Red**:
        *   初始余额: 1000 USDT.
        *   Action: 下单买入花费 100 USDT。
        *   Check: 再次下单 950 USDT，断言失败（可用余额不足）。
    *   [ ] **Green**: 在本地内存维护 `Inventory` 结构，下单时扣减 `frozen`，成交/取消时回滚。

---

## Phase 5: 交易所适配器 (Connectors) - The I/O Layer

**目标**: 处理脏活累活（断连、鉴权），但对外提供干净的数据流。

### 5.1 Binance WebSocket 集成
*   **Context**: 真实网络测试不可靠，需使用 Mock Server。
*   **TDD Cycle**:
    *   [ ] **Red**: 使用 `warp` 或 `tokio-tungstenite` 启动一个本地 Mock WS Server。
        *   Mock Server 发送: `{"e": "depthUpdate", ...}`
        *   Client 行为: 连接 Mock Server，断言收到的 Channel 消息已转换为标准 `OrderBookUpdate`。
    *   [ ] **Green**: 实现 Binance Connector，连接到 `ws://127.0.0.1:xxxx`。
    *   [ ] **Refactor**: 增加重连逻辑测试。关闭 Mock Server，等待 1s 重启，断言 Client 自动重连成功。

### 5.2 签名与 REST API (Wiremock)
*   **Context**: 测试下单签名逻辑是否正确。
*   **TDD Cycle**:
    *   [ ] **Red**: 使用 `wiremock` crate 模拟 Binance 下单接口 `POST /api/v3/order`。
    *   [ ] **Green**: 实现 HMAC-SHA256 签名。发送请求。
    *   [ ] **Verify**: 检查 Mock Server 收到的 Header 中 `X-MBX-APIKEY` 是否存在，Query Param 中 `signature` 是否符合预期。

---

## Phase 6: 订单管理系统 (OMS) - Lifecycle Management

### 6.1 订单状态转换
*   **TDD Cycle**:
    *   [ ] **Red**:
        *   创建订单 -> 状态 `New`。
        *   模拟收到 WS 推送 `ExecutionReport (PartiallyFilled)` -> 状态变更为 `PartiallyFilled`。
        *   模拟收到 WS 推送 `ExecutionReport (Filled)` -> 状态变更为 `Filled`，并归档。
    *   [ ] **Green**: 实现 `OrderManager` 结构体，维护 `HashMap<ClientOrderId, OrderState>`。

### 6.2 速率限制 (Rate Limiting)
*   **TDD Cycle**:
    *   [ ] **Red**: 配置限频 10 req/s。循环发送 20 个请求。断言前 10 个成功，第 11 个需等待或被拒绝（取决于策略，HFT 通常选择丢弃或排队）。
    *   [ ] **Green**: 集成 `governor` crate 实现 Token Bucket 算法。

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

---

## 开发优先级排序 (Priority Queue)

1.  **P0**: `Core` 类型定义与 `OrderBook` 逻辑 (纯算法，基石)。
2.  **P0**: `Strategy` 简单套利逻辑测试 (核心价值)。
3.  **P1**: `Connectors` 的 WebSocket 解析与标准化 (数据源)。
4.  **P1**: `OMS` 的基础下单与签名 (执行能力)。
5.  **P2**: `Risk` 基础检查 (安全)。
6.  **P3**: 真实交易所对接调试。

## 关键 Rust Crates 推荐 (Refined)

*   **Testing**: `mockall` (Mocking), `proptest` (Property-based), `wiremock` (HTTP Mock), `criterion` (Benchmark).
*   **Async Runtime**: `tokio` (standard), `tokio-console` (调试异步死锁/性能瓶颈).
*   **Fast Parsing**: `simd-json` (Critical for WS).
*   **Data Structures**: `smallvec` (Stack allocation), `dashmap` (Concurrent Map for OMS, though prefer message passing).

通过严格遵循这个 TDD 流程，你将构建出一个**即使在没有真实网络连接时也能通过 100% 测试**的系统，上线时的 Bug 率将极低。