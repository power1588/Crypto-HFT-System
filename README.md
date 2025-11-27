# Crypto HFT System - Phase 1: Core Domain

## 概述

这是基于TDD（测试驱动开发）模式的高频加密货币交易系统的Phase 1实现，专注于核心领域模型的设计与实现。

## 核心特性

### 1. 类型安全 (Type Safety)

使用NewType模式实现了类型安全的`Price`和`Size`类型，防止了价格和数量的意外混用：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Price(pub Decimal);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Size(pub Decimal);
```

**特性**:
- 防止价格和数量之间的意外运算
- 支持精确的十进制运算，避免浮点误差
- 自定义序列化/反序列化，保持精度
- 支持与Decimal的标量运算

### 2. 高性能订单簿 (High-Performance OrderBook)

实现了基于BTreeMap的高性能订单簿数据结构：

```rust
pub struct OrderBook {
    symbol: String,
    bids: BTreeMap<Price, Size>, // 降序排列
    asks: BTreeMap<Price, Size>, // 升序排列
    last_update: u64,
}
```

**特性**:
- 高效的价格级别管理
- 支持快照和增量更新
- 使用SmallVec优化前N档价格查询（栈分配）
- 纳秒级查询性能

### 3. 测试驱动开发 (TDD)

所有核心功能都通过全面的单元测试验证：

- 16个单元测试，100%通过
- 覆盖所有边界条件和错误情况
- 使用属性测试验证不变量

## 性能基准

使用Criterion进行性能测试，结果如下：

| 操作 | 性能 |
|------|------|
| 创建OrderBook | ~27 ns |
| 获取最佳买卖价 | ~25 ns |
| 获取前20档价格 | ~322 ns |
| 应用1000档快照 | ~121 μs |
| 应用1000次增量更新 | ~182 μs |

## 项目结构

```
src/
├── lib.rs              # 库入口
├── types/              # 类型定义
│   ├── mod.rs
│   ├── price.rs        # Price类型实现
│   └── size.rs         # Size类型实现
└── orderbook/          # 订单簿实现
    ├── mod.rs
    ├── types.rs        # 订单簿相关类型
    └── orderbook.rs    # OrderBook核心实现
```

## 运行测试

```bash
# 运行所有单元测试
cargo test

# 运行性能基准测试
cargo bench --bench orderbook_benchmark
```

## 下一步计划

Phase 2将专注于：

1. 定义核心Traits（MarketDataStream, ExecutionClient）
2. 实现消息解析性能优化
3. 引入模拟测试框架

## 技术栈

- **Rust 2021**: 高性能系统编程语言
- **rust_decimal**: 精确的十进制运算
- **serde**: 序列化/反序列化
- **smallvec**: 栈分配优化
- **criterion**: 性能基准测试
- **mockall**: 模拟测试框架

## 设计原则

1. **类型安全**: 利用Rust类型系统防止逻辑错误
2. **零拷贝**: 尽可能避免不必要的内存分配
3. **高性能**: 针对高频交易场景优化
4. **可测试性**: 所有组件都易于测试和模拟
