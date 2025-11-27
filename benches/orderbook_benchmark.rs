use criterion::{black_box, criterion_group, criterion_main, Criterion};
use crypto_hft::orderbook::{OrderBook, OrderBookSnapshot, OrderBookDelta};
use crypto_hft::types::{Price, Size};

fn create_price(value: &str) -> Price {
    Price::from_str(value).unwrap()
}

fn create_size(value: &str) -> Size {
    Size::from_str(value).unwrap()
}

fn create_large_snapshot(levels: usize) -> OrderBookSnapshot {
    let mut bids = Vec::with_capacity(levels);
    let mut asks = Vec::with_capacity(levels);
    
    for i in 0..levels {
        // Create bids in descending order (100.00, 99.99, 99.98, ...)
        let bid_price = 10000 - i; // 100.00, 99.99, 99.98, ...
        let bid_price_str = format!("{}.{:02}", bid_price / 100, bid_price % 100);
        bids.push(crypto_hft::orderbook::types::OrderBookLevel::new(
            create_price(&bid_price_str),
            create_size(&format!("{}", i + 1))
        ));
        
        // Create asks in ascending order (100.01, 100.02, 100.03, ...)
        let ask_price = 10001 + i; // 100.01, 100.02, 100.03, ...
        let ask_price_str = format!("{}.{:02}", ask_price / 100, ask_price % 100);
        asks.push(crypto_hft::orderbook::types::OrderBookLevel::new(
            create_price(&ask_price_str),
            create_size(&format!("{}", i + 1))
        ));
    }
    
    OrderBookSnapshot::new(
        "BTCUSDT".to_string(),
        bids,
        asks,
        123456789,
    )
}

fn create_delta_updates(count: usize) -> Vec<OrderBookDelta> {
    let mut deltas = Vec::with_capacity(count);
    
    for i in 0..count {
        // Create delta with random updates
        let bid_price = 10000 - (i % 20); // Update top 20 bid levels
        let bid_price_str = format!("{}.{:02}", bid_price / 100, bid_price % 100);
        
        let ask_price = 10001 + (i % 20); // Update top 20 ask levels
        let ask_price_str = format!("{}.{:02}", ask_price / 100, ask_price % 100);
        
        deltas.push(OrderBookDelta::new(
            "BTCUSDT".to_string(),
            vec![
                crypto_hft::orderbook::types::OrderBookLevel::new(
                    create_price(&bid_price_str),
                    create_size(&format!("{}", (i + 1) % 10 + 1))
                ),
            ],
            vec![
                crypto_hft::orderbook::types::OrderBookLevel::new(
                    create_price(&ask_price_str),
                    create_size(&format!("{}", (i + 1) % 10 + 1))
                ),
            ],
            123456789 + i as u64,
        ));
    }
    
    deltas
}

fn bench_orderbook_creation(c: &mut Criterion) {
    c.bench_function("orderbook_creation", |b| {
        b.iter(|| {
            let book = OrderBook::new(black_box("BTCUSDT".to_string()));
            black_box(book)
        })
    });
}

fn bench_orderbook_apply_snapshot(c: &mut Criterion) {
    let mut group = c.benchmark_group("orderbook_apply_snapshot");
    
    for levels in [10, 100, 1000].iter() {
        let snapshot = create_large_snapshot(*levels);
        
        group.bench_with_input(format!("levels_{}", levels), levels, |b, _| {
            b.iter_with_setup(
                || OrderBook::new("BTCUSDT".to_string()),
                |mut book| {
                    book.apply_snapshot(black_box(snapshot.clone()));
                    black_box(book)
                }
            )
        });
    }
    
    group.finish();
}

fn bench_orderbook_apply_delta(c: &mut Criterion) {
    let mut group = c.benchmark_group("orderbook_apply_delta");
    
    for updates in [10, 100, 1000].iter() {
        let deltas = create_delta_updates(*updates);
        
        group.bench_with_input(format!("updates_{}", updates), updates, |b, _| {
            b.iter_with_setup(
                || {
                    let mut book = OrderBook::new("BTCUSDT".to_string());
                    book.apply_snapshot(create_large_snapshot(1000));
                    book
                },
                |mut book| {
                    for delta in &deltas {
                        book.apply_delta(black_box(delta.clone()));
                    }
                    black_box(book)
                }
            )
        });
    }
    
    group.finish();
}

fn bench_orderbook_top_levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("orderbook_top_levels");
    
    // Create a book with many levels
    let snapshot = create_large_snapshot(1000);
    let mut book = OrderBook::new("BTCUSDT".to_string());
    book.apply_snapshot(snapshot);
    
    for n in [1, 5, 10, 20].iter() {
        group.bench_with_input(format!("top_{}", n), n, |b, &n| {
            b.iter(|| {
                let bids = book.top_bids(black_box(n));
                let asks = book.top_asks(black_box(n));
                black_box((bids, asks))
            })
        });
    }
    
    group.finish();
}

fn bench_orderbook_best_prices(c: &mut Criterion) {
    // Create a book with many levels
    let snapshot = create_large_snapshot(1000);
    let mut book = OrderBook::new("BTCUSDT".to_string());
    book.apply_snapshot(snapshot);
    
    c.bench_function("orderbook_best_prices", |b| {
        b.iter(|| {
            let best_bid = book.best_bid();
            let best_ask = book.best_ask();
            let spread = book.spread();
            black_box((best_bid, best_ask, spread))
        })
    });
}

criterion_group!(
    benches,
    bench_orderbook_creation,
    bench_orderbook_apply_snapshot,
    bench_orderbook_apply_delta,
    bench_orderbook_top_levels,
    bench_orderbook_best_prices
);
criterion_main!(benches);
