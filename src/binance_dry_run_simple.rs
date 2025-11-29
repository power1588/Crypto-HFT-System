use crypto_hft::{
    connectors::DryRunExecutionClient,
    core::events::OrderBookSnapshot,
    exchanges::binance::BinanceWebSocket,
    init_logging,
    orderbook::OrderBook,
    strategies::MarketMakingStrategy,
    strategy::{MarketState, Signal, Strategy},
    traits::{ExecutionClient, MarketDataStream, MarketEvent},
    types::{Price, Size},
};
use log::info;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Duration;

/// Print market data event
fn print_market_event(event: &MarketEvent) {
    match event {
        MarketEvent::OrderBookSnapshot(snapshot) => {
            println!("\n╔════════════════════════════════════════════════════════════╗");
            println!("║ 📊 ORDER BOOK SNAPSHOT (订单簿快照)                        ║");
            println!("╠════════════════════════════════════════════════════════════╣");
            println!("║ Symbol:      {:45} ║", snapshot.symbol);
            println!("║ Timestamp:   {:45} ║", snapshot.timestamp);

            if let Some(best_bid) = snapshot.bids.first() {
                println!(
                    "║ Best Bid:    {:45} ║",
                    format!("{} @ {}", best_bid.price, best_bid.size)
                );
            }
            if let Some(best_ask) = snapshot.asks.first() {
                println!(
                    "║ Best Ask:    {:45} ║",
                    format!("{} @ {}", best_ask.price, best_ask.size)
                );
            }

            if let (Some(best_bid), Some(best_ask)) = (snapshot.bids.first(), snapshot.asks.first())
            {
                let spread = best_ask.price - best_bid.price;
                println!("║ Spread:      {:45} ║", format!("{}", spread));
            }

            println!(
                "║ Bids:        {:45} ║",
                format!("{} levels", snapshot.bids.len())
            );
            println!(
                "║ Asks:        {:45} ║",
                format!("{} levels", snapshot.asks.len())
            );
            println!("╚════════════════════════════════════════════════════════════╝\n");
        }
        MarketEvent::OrderBookDelta(delta) => {
            println!("\n╔════════════════════════════════════════════════════════════╗");
            println!("║ 📈 ORDER BOOK UPDATE (订单簿更新)                            ║");
            println!("╠════════════════════════════════════════════════════════════╣");
            println!("║ Symbol:      {:45} ║", delta.symbol);
            println!("║ Timestamp:   {:45} ║", delta.timestamp);

            if let Some(best_bid) = delta.bids.first() {
                println!(
                    "║ Best Bid:    {:45} ║",
                    format!("{} @ {}", best_bid.price, best_bid.size)
                );
            }
            if let Some(best_ask) = delta.asks.first() {
                println!(
                    "║ Best Ask:    {:45} ║",
                    format!("{} @ {}", best_ask.price, best_ask.size)
                );
            }

            if let (Some(best_bid), Some(best_ask)) = (delta.bids.first(), delta.asks.first()) {
                let spread = best_ask.price - best_bid.price;
                println!("║ Spread:      {:45} ║", format!("{}", spread));
            }

            println!(
                "║ Bid Updates: {:45} ║",
                format!("{} levels", delta.bids.len())
            );
            println!(
                "║ Ask Updates: {:45} ║",
                format!("{} levels", delta.asks.len())
            );
            println!("╚════════════════════════════════════════════════════════════╝\n");
        }
        MarketEvent::Trade(trade) => {
            println!("\n╔════════════════════════════════════════════════════════════╗");
            println!("║ 💰 TRADE (成交)                                             ║");
            println!("╠════════════════════════════════════════════════════════════╣");
            println!("║ Symbol:      {:45} ║", trade.symbol);
            println!("║ Price:       {:45} ║", format!("{}", trade.price));
            println!("║ Size:        {:45} ║", format!("{}", trade.size));
            println!("║ Timestamp:   {:45} ║", trade.timestamp);
            println!("║ Side:        {:45} ║", format!("{:?}", trade.side));
            println!("╚════════════════════════════════════════════════════════════╝\n");
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_logging("info", None)?;

    info!("Starting Binance BTCUSDT Market Making Strategy (Dry-Run Mode)");
    info!("连接Binance BTCUSDT现货实时行情...");

    // Create Binance WebSocket directly
    let mut binance_ws = BinanceWebSocket::new();

    // Connect to BTCUSDT depth stream
    binance_ws.connect(&["BTCUSDT"]).await.map_err(|e| {
        eprintln!("Failed to connect to Binance WebSocket: {}", e);
        format!("WebSocket connection failed: {}", e)
    })?;

    info!("✅ Connected to Binance WebSocket for BTCUSDT");

    // Wrap WebSocket in Arc<Mutex> for concurrent access
    let market_stream: Arc<Mutex<BinanceWebSocket>> = Arc::new(Mutex::new(binance_ws));

    // Create dry-run execution client
    let execution_client = Arc::new(DryRunExecutionClient::new());

    // Create market making strategy
    let mut strategy = MarketMakingStrategy::new(
        Price::from_str("1.0").map_err(|e| format!("Invalid price: {}", e))?, // Target spread: $1.0
        Size::from_str("0.001").map_err(|e| format!("Invalid size: {}", e))?, // Base order size: 0.001 BTC
        Size::from_str("0.1").map_err(|e| format!("Invalid size: {}", e))?, // Max position size: 0.1 BTC
        5,                                                                  // Max order levels
        Duration::from_millis(1000), // Order refresh time: 1 second
    );

    // Create order book to track market state
    let order_book = Arc::new(tokio::sync::RwLock::new(OrderBook::new(
        "BTCUSDT".to_string(),
    )));

    // Track last signal generation time
    let last_signal_time = Arc::new(tokio::sync::RwLock::new(std::time::Instant::now()));

    info!("🚀 Starting market data stream processing...");
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║ 🎯 Binance BTCUSDT Market Making (Dry-Run Mode)            ║");
    println!("║ 📊 Real-time market data will be displayed below          ║");
    println!("║ 💡 Strategy orders will be printed but not executed       ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Main event loop
    loop {
        // Get next market event
        let event_result = {
            let mut ws = market_stream.lock().await;
            ws.next().await
        };

        if let Some(event_result) = event_result {
            match event_result {
                Ok(event) => {
                    // Print market event
                    print_market_event(&event);

                    // Update order book
                    {
                        let mut ob = order_book.write().await;
                        match &event {
                            MarketEvent::OrderBookSnapshot(snapshot) => {
                                ob.apply_snapshot(snapshot.clone());
                            }
                            MarketEvent::OrderBookDelta(delta) => {
                                ob.apply_delta(delta.clone());
                            }
                            _ => {}
                        }
                    }

                    // Generate signals from strategy periodically
                    let should_generate = {
                        let last_time = last_signal_time.read().await;
                        last_time.elapsed() >= Duration::from_millis(1000) // Generate signals every second
                    };

                    if should_generate {
                        // Get current market state from order book
                        let market_state = {
                            let ob = order_book.read().await;
                            let mut state = MarketState::new("BTCUSDT".to_string());

                            // Create snapshot from current order book state
                            let bids: Vec<_> = ob
                                .top_bids(10)
                                .iter()
                                .map(|(p, s)| crypto_hft::orderbook::OrderBookLevel::new(*p, *s))
                                .collect();
                            let asks: Vec<_> = ob
                                .top_asks(10)
                                .iter()
                                .map(|(p, s)| crypto_hft::orderbook::OrderBookLevel::new(*p, *s))
                                .collect();

                            let snapshot = OrderBookSnapshot::new(
                                "BTCUSDT".to_string(),
                                "binance".to_string(),
                                bids,
                                asks,
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis() as u64,
                            );

                            state.update(&MarketEvent::OrderBookSnapshot(snapshot));
                            state
                        };

                        // Generate signal from strategy
                        if let Some(signal) = strategy.generate_signal(&market_state) {
                            // Convert signal to order and execute (dry-run)
                            match signal {
                                Signal::PlaceOrder { order } => {
                                    // Execute order via dry-run client (will print but not actually place)
                                    if let Err(e) = execution_client.place_order(order).await {
                                        eprintln!("Error placing order: {}", e);
                                    }
                                }
                                _ => {
                                    // Handle other signal types if needed
                                }
                            }

                            // Update last signal time
                            {
                                let mut last_time = last_signal_time.write().await;
                                *last_time = std::time::Instant::now();
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error receiving market data: {}", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        } else {
            // No more events, wait a bit
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}
