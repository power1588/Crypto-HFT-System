这是一个基于 **Rust** 编写的高频交易（HFT）与做市（Market Making）系统架构设计文档。

该架构专为**跨交易所套利**和**高频做市**设计，重点关注低延迟（Low Latency）、内存安全（Memory Safety）和高并发处理能力（High Concurrency）。

---

# Rust Crypto HFT & Cross-Exchange Arbitrage System Architecture

## 1. 系统概述 (Overview)

本系统旨在构建一个高性能、模块化的加密货币交易基础设施。利用 Rust 的 `Zero-cost abstractions` 和 `Ownership` 模型，在保证内存安全的同时实现接近 C++ 的执行效率。

### 核心目标
*   **极低延迟**：Tick-to-Trade 延迟控制在微秒级（Microseconds）。
*   **高吞吐量**：能够处理成千上万的 WebSocket 推送和订单更新。
*   **高可用与可扩展**：支持动态添加新的交易所适配器和策略模块。
*   **安全性**：严格的风险控制层（Pre-trade Risk Checks）。

---

## 2. 系统架构图 (Architecture Diagram)

系统采用 **事件驱动 (Event-Driven)** 架构。核心链路在单一进程内通过内存通信（Channels/Ring Buffer）以减少上下文切换，周边服务通过 gRPC/ZeroMQ 解耦。

```mermaid
graph TD
    subgraph External["外部交易所 (Exchanges)"]
        Binance
        OKX
        Bybit
        Coinbase
    end

    subgraph Infrastructure["基础设施层"]
        DB[(TimescaleDB/ClickHouse)]
        Cache[(Redis Cluster)]
        Log[Logging Service]
    end

    subgraph Core_System["Rust Core Engine (Tokio Runtime)"]
        direction TB
        
        %% 模块定义
        MDG[Market Data Gateway<br/>(行情网关)]
        Normalizer[Data Normalizer<br/>(数据标准化)]
        Strategy[Strategy Engine<br/>(策略引擎)]
        Risk[Risk Management<br/>(风控系统)]
        OMS[Order Management System<br/>(订单管理)]
        
        %% 内部数据流
        MDG -->|Raw WS Msg| Normalizer
        Normalizer -->|Unified OrderBook/Trade| Strategy
        Strategy -->|Signal| Risk
        Risk -->|Approved Order| OMS
        OMS -->|Order Execution| External
        OMS -->|Execution Report| Strategy
        
        %% 异步持久化
        Normalizer -.->|Async Save| DB
        OMS -.->|Async Save| DB
    end

    External -->|WebSocket| MDG
    External <-->|REST API| OMS
```

---

## 3. 核心模块详解 (Core Modules)

### 3.1 行情网关 (Market Data Gateway - MDG)
负责维护与交易所的长连接，接收原始数据。

*   **技术栈**: `tokio-tungstenite`, `reqwest`.
*   **功能**:
    *   管理 WebSocket 连接（心跳、断线重连）。
    *   多路复用：一个连接处理多个交易对，或多个连接分片处理。
*   **优化**: 使用 `Epoll` 模型（Tokio默认）处理数千个并发连接。

### 3.2 数据标准化 (Data Normalizer)
将不同交易所的异构数据转换为系统内部统一格式。

*   **技术栈**: `serde`, `serde_json`, `simd-json` (高性能解析).
*   **数据结构**:
    ```rust
    pub struct OrderBookL2 {
        pub exchange: ExchangeId,
        pub symbol: SymbolId,
        pub bids: Vec<(Price, Size)>,
        pub asks: Vec<(Price, Size)>,
        pub timestamp: u64,
    }
    ```
*   **关键逻辑**: 维护本地 Orderbook Snapshot，增量更新（Delta updates）合并。

### 3.3 策略引擎 (Strategy Engine)
系统的核心大脑，处理业务逻辑。

*   **设计**: 采用 **Actor 模型** (基于 `tokio::sync::mpsc` 或 `actix`)。
*   **套利逻辑**:
    *   **Bellman-Ford / 负环检测**: 用于多角套利。
    *   **价差监控**: 用于两角搬砖 (Exchange A Buy, Exchange B Sell)。
*   **做市逻辑**:
    *   基于 AS (Avellaneda-Stoikov) 模型计算最优 Spread。
    *   库存倾斜（Inventory Skew）调整。

### 3.4 风险管理系统 (Risk Management System - RMS)
在订单发出前的最后一道防线。**同步执行，必须极快。**

*   **检查项**:
    *   **最大敞口限制 (Max Position Exposure)**。
    *   **Fat-finger 检查**: 防止价格偏离市场价过多。
    *   **Kill Switch**: 全局紧急熔断开关 (配合 Redis 标志位)。
    *   **资金检查**: 确保余额足够（本地维护 Shadow Balance）。

### 3.5 订单管理系统 (Order Management System - OMS)
负责订单生命周期管理和对外执行。

*   **功能**:
    *   **签名**: Ed25519 / HMAC-SHA256 高性能签名。
    *   **Rate Limiter**: 使用 `governor` 库实现 Token Bucket 算法，严格遵守交易所 API 限制。
    *   **Nonce Management**: 处理并发下的 Nonce 同步问题。
    *   **Smart Routing**: 选择延迟最低的网关节点发送订单。

---

## 4. Rust 技术选型与性能优化 (Tech Stack & Optimization)

### 4.1 核心 Crates 推荐

| 类别 | Crate | 用途 |
| :--- | :--- | :--- |
| **Runtime** | `tokio` | 异步运行时，开启 `full` features 和 `multi-thread`。 |
| **HTTP/WS** | `reqwest`, `tungstenite` | 网络通信。 |
| **Serialization** | `serde`, `simd-json` | `simd-json` 比标准 JSON 解析快 2-3 倍。 |
| **Numerical** | `rust_decimal`, `ndarray` | 高精度金额计算，避免浮点数误差。 |
| **Channels** | `flume` or `crossbeam` | 比标准库 mpsc 更快的 MPMC 通道。 |
| **Tracing** | `tracing`, `tracing-appender` | 结构化日志，异步写入避免阻塞热路径。 |
| **Time** | `chrono` | 时间处理。 |

### 4.2 性能优化策略 (Low Latency Tricks)

1.  **内存管理**:
    *   **Object Pooling**: 复用 Orderbook 和 Trade 对象，减少 `malloc` 和 `free` (使用 `object-pool` crate)。
    *   **Stack Allocation**: 尽可能在栈上分配小对象，使用 `SmallVec` 替代 `Vec`。

2.  **线程绑定 (Core Pinning)**:
    *   使用 `core_affinity` 将关键线程（如策略计算线程）绑定到特定 CPU 核，减少 Cache Miss 和上下文切换。

3.  **无锁编程 (Lock-free)**:
    *   在热路径（Hot Path）上避免使用 `Mutex`。使用 `Atomic` 类型或消息传递（Actor模型）来共享状态。
    *   如果必须共享读，使用 `RwLock` 甚至 `arc-swap` (RCU机制)。

4.  **网络层优化**:
    *   禁用 Nagle 算法 (`TCP_NODELAY`)。
    *   如果交易所支持，使用 UDP (QUIC) 协议（目前 Crypto 较少支持，主要还是 WS）。

---

## 5. 跨交易所套利特有设计 (Cross-Exchange Specifics)

### 5.1 统一账户与资金管理
为了实现跨交易所套利，必须维护一个**全局资金视图**。

*   **Shadow Ledger**: 在内存中实时维护各交易所各币种余额。
*   **Rebalancing (再平衡)**: 当某交易所资金耗尽时，自动触发划转脚本或通过交易调整持仓。

### 5.2 时钟同步与延迟测算
*   **Latency Monitoring**: 每个 Tick 到达时记录 `local_ts`，计算 `local_ts - exchange_ts`。
*   **Jitter Handling**: 策略需考虑网络抖动，只有价差 > (交易成本 + 预期滑点 + 风险溢价) 时才触发。

---

## 6. 数据存储与分析 (Persistence & Analytics)

虽然交易是实时的，但数据需要持久化用于回测和审计。

*   **热数据 (Redis)**: 存储当前的 Position, Open Orders, System State。
*   **冷数据 (TimescaleDB / ClickHouse)**:
    *   Orderbook Snapshots (每秒或每100ms采样)。
    *   Trade History。
    *   Log Files。
*   **写入策略**: 不要直接从主线程写入 DB。使用一个独立的 `PersistWorker` 通过 Channel 接收数据并批量写入。

---

## 7. 部署与运维 (Deployment)

*   **Co-location (服务器托管)**: 服务器应部署在 AWS (Tokyo) 等离交易所服务器最近的区域（对于 Binance/OKX/Bybit）。
*   **Rust Binary**: 编译为纯静态二进制文件 (`x86_64-unknown-linux-musl`)，基于 `Alpine` 或 `Distroless` 镜像构建 Docker。
*   **CI/CD**:
    *   `cargo check`, `cargo clippy`, `cargo audit` (安全检查)。
    *   单元测试覆盖核心算法，集成测试使用 Mock Server 模拟交易所。

---

## 8. 代码结构示例 (Project Structure)

```text
crypto-hft-system/
├── Cargo.toml
├── src/
│   ├── main.rs            # 入口点
│   ├── config.rs          # 配置加载
│   ├── core/
│   │   ├── types.rs       # 基础类型 (Side, OrderType)
│   │   ├── orderbook.rs   # 订单薄逻辑
│   │   └── ring_buffer.rs # 内部通信
│   ├── connectors/        # 交易所适配器
│   │   ├── binance.rs
│   │   ├── okx.rs
│   │   └── traits.rs      # 定义 Connector Trait
│   ├── strategy/
│   │   ├── maker.rs       # 做市策略
│   │   └── arb.rs         # 套利策略
│   ├── oms/
│   │   ├── execution.rs   # 执行逻辑
│   │   └── risk.rs        # 风控
│   └── utils/
│       ├── time.rs
│       └── logger.rs
```

---

此架构文档提供了一个高起点的 HFT 系统蓝图。使用 Rust 能够让你在处理高并发 IO 的同时，不必担心 C++ 中常见的段错误和内存泄漏问题，是现代量化交易系统的最佳选择。