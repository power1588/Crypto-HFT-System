use criterion::{black_box, criterion_group, criterion_main, Criterion};
use crypto_hft::connectors::BinanceMessage;

fn bench_serde_json_parsing(c: &mut Criterion) {
    let json = r#"{
        "e": "depthUpdate",
        "E": 1672515782136,
        "s": "BNBBTC",
        "U": 157,
        "u": 160,
        "b": [
            ["0.0024", "10"],
            ["0.0023", "100"],
            ["0.0022", "50"],
            ["0.0021", "200"],
            ["0.0020", "150"],
            ["0.0019", "75"],
            ["0.0018", "300"],
            ["0.0017", "125"],
            ["0.0016", "80"],
            ["0.0015", "90"]
        ],
        "a": [
            ["0.0026", "100"],
            ["0.0027", "10"],
            ["0.0028", "50"],
            ["0.0029", "200"],
            ["0.0030", "150"],
            ["0.0031", "75"],
            ["0.0032", "300"],
            ["0.0033", "125"],
            ["0.0034", "80"],
            ["0.0035", "90"]
        ]
    }"#;

    c.bench_function("serde_json_parsing", |b| {
        b.iter(|| {
            let message = BinanceMessage::from_json(black_box(json)).unwrap();
            black_box(message)
        })
    });
}

fn bench_message_to_market_event(c: &mut Criterion) {
    let json = r#"{
        "e": "depthUpdate",
        "E": 1672515782136,
        "s": "BNBBTC",
        "U": 157,
        "u": 160,
        "b": [
            ["0.0024", "10"],
            ["0.0023", "100"],
            ["0.0022", "50"],
            ["0.0021", "200"],
            ["0.0020", "150"],
            ["0.0019", "75"],
            ["0.0018", "300"],
            ["0.0017", "125"],
            ["0.0016", "80"],
            ["0.0015", "90"]
        ],
        "a": [
            ["0.0026", "100"],
            ["0.0027", "10"],
            ["0.0028", "50"],
            ["0.0029", "200"],
            ["0.0030", "150"],
            ["0.0031", "75"],
            ["0.0032", "300"],
            ["0.0033", "125"],
            ["0.0034", "80"],
            ["0.0035", "90"]
        ]
    }"#;

    let message = BinanceMessage::from_json(json).unwrap();
    
    c.bench_function("message_to_market_event", |b| {
        b.iter(|| {
            let event = message.clone().to_market_event();
            black_box(event)
        })
    });
}

criterion_group!(
    benches,
    bench_serde_json_parsing,
    bench_message_to_market_event
);
criterion_main!(benches);
