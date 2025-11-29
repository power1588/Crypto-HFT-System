use async_trait::async_trait;
use crate::traits::{MarketDataStream, MarketEvent};
use crate::exchanges::error::ExchangeError;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Wrapper to convert MarketDataStream with ExchangeError to Box<dyn Error>
pub struct MarketStreamWrapper {
    inner: Arc<Mutex<dyn MarketDataStream<Error = ExchangeError> + Send + Sync>>,
}

impl MarketStreamWrapper {
    pub fn new(inner: Arc<Mutex<dyn MarketDataStream<Error = ExchangeError> + Send + Sync>>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl MarketDataStream for MarketStreamWrapper {
    type Error = ExchangeError;

    async fn subscribe(&mut self, symbols: &[&str]) -> Result<(), Self::Error> {
        let mut inner = self.inner.lock().await;
        inner.subscribe(symbols).await
    }

    async fn unsubscribe(&mut self, symbols: &[&str]) -> Result<(), Self::Error> {
        let mut inner = self.inner.lock().await;
        inner.unsubscribe(symbols).await
    }

    async fn next(&mut self) -> Option<Result<MarketEvent, Self::Error>> {
        let mut inner = self.inner.lock().await;
        inner.next().await
    }

    fn is_connected(&self) -> bool {
        // We can't easily check this without async, so return true
        true
    }

    fn last_update(&self, symbol: &str) -> Option<u64> {
        // We can't easily check this without async, so return None
        None
    }
}

