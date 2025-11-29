pub mod binance;
pub mod mock;
// Temporarily disabled due to compilation errors - need to fix Error types
// pub mod okx;
// pub mod gate;
// pub mod bybit;
// pub mod hyperliquid;
// pub mod dydx;
// pub mod aster;
pub mod connection_manager;
pub mod error;

pub use binance::{BinanceAdapter, BinanceWebSocketAdapter};
pub use mock::MockExchangeAdapter;
// Temporarily disabled
// pub use okx::OkxAdapter;
// pub use gate::GateAdapter;
// pub use bybit::BybitAdapter;
// pub use hyperliquid::HyperliquidAdapter;
// pub use dydx::DydxAdapter;
// pub use aster::AsterAdapter;
pub use connection_manager::{ConnectionManager, ConnectionStatus, ExchangeAdapter};
pub use error::{BoxedError, ExchangeError};
