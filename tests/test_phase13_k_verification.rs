//! TDD tests for Phase 13 Group K: Performance Monitor Field Access
//!
//! Tests verify:
//! - T117: Async field access works correctly (no deadlocks)
//! - T118: get_fill_rate, get_cancellation_rate, get_rejection_rate methods exist and work

use crypto_hft::realtime::performance_monitor::{PerformanceMetrics, PerformanceMonitor};
use rust_decimal::Decimal;
use std::str::FromStr;

// =============================================================================
// T117: Async field access tests
// =============================================================================

/// Test T117-1: PerformanceMetrics default creation
#[test]
fn test_t117_1_performance_metrics_default() {
    let metrics = PerformanceMetrics::default();
    
    assert_eq!(metrics.market_data_events, 0);
    assert_eq!(metrics.signals_generated, 0);
    assert_eq!(metrics.orders_placed, 0);
    assert_eq!(metrics.orders_filled, 0);
    assert_eq!(metrics.orders_canceled, 0);
    assert_eq!(metrics.orders_rejected, 0);
    assert_eq!(metrics.risk_violations, 0);
    assert!(metrics.average_latency.is_none());
    assert!(metrics.total_pnl.is_none());
    assert!(metrics.sharpe_ratio.is_none());
    assert!(metrics.max_drawdown.is_none());
    assert!(metrics.win_rate.is_none());
    assert!(metrics.profit_factor.is_none());
}

/// Test T117-2: PerformanceMonitor creation and get_metrics async access
#[tokio::test]
async fn test_t117_2_performance_monitor_creation() {
    let monitor = PerformanceMonitor::new();
    
    // Await metrics - this should not deadlock
    let metrics = monitor.get_metrics().await;
    
    assert_eq!(metrics.market_data_events, 0);
    assert_eq!(metrics.orders_placed, 0);
}

/// Test T117-3: record_market_data_event async operation
#[tokio::test]
async fn test_t117_3_record_market_data_event() {
    let monitor = PerformanceMonitor::new();
    
    // Record events
    monitor.record_market_data_event().await;
    monitor.record_market_data_event().await;
    monitor.record_market_data_event().await;
    
    // Await metrics and access fields
    let metrics = monitor.get_metrics().await;
    assert_eq!(metrics.market_data_events, 3);
}

/// Test T117-4: record_signal async operation
#[tokio::test]
async fn test_t117_4_record_signal() {
    let monitor = PerformanceMonitor::new();
    
    monitor.record_signal().await;
    monitor.record_signal().await;
    
    let metrics = monitor.get_metrics().await;
    assert_eq!(metrics.signals_generated, 2);
}

/// Test T117-5: record_order_placement async operation
#[tokio::test]
async fn test_t117_5_record_order_placement() {
    let monitor = PerformanceMonitor::new();
    
    monitor.record_order_placement("order_1").await;
    monitor.record_order_placement("order_2").await;
    
    let metrics = monitor.get_metrics().await;
    assert_eq!(metrics.orders_placed, 2);
}

/// Test T117-6: record_order_fill async operation
#[tokio::test]
async fn test_t117_6_record_order_fill() {
    let monitor = PerformanceMonitor::new();
    
    monitor.record_order_placement("order_1").await;
    monitor.record_order_fill("order_1").await;
    
    let metrics = monitor.get_metrics().await;
    assert_eq!(metrics.orders_placed, 1);
    assert_eq!(metrics.orders_filled, 1);
}

/// Test T117-7: record_order_cancellation async operation
#[tokio::test]
async fn test_t117_7_record_order_cancellation() {
    let monitor = PerformanceMonitor::new();
    
    monitor.record_order_placement("order_1").await;
    monitor.record_order_cancellation("order_1").await;
    
    let metrics = monitor.get_metrics().await;
    assert_eq!(metrics.orders_placed, 1);
    assert_eq!(metrics.orders_canceled, 1);
}

/// Test T117-8: record_order_rejection async operation
#[tokio::test]
async fn test_t117_8_record_order_rejection() {
    let monitor = PerformanceMonitor::new();
    
    monitor.record_order_placement("order_1").await;
    monitor.record_order_rejection("order_1").await;
    
    let metrics = monitor.get_metrics().await;
    assert_eq!(metrics.orders_placed, 1);
    assert_eq!(metrics.orders_rejected, 1);
}

/// Test T117-9: record_pnl async operation (this is the critical test for deadlock fix)
#[tokio::test]
async fn test_t117_9_record_pnl_no_deadlock() {
    let monitor = PerformanceMonitor::new();
    
    // Record multiple P&L entries - this should NOT deadlock
    monitor.record_pnl(Decimal::from_str("100.0").unwrap()).await;
    monitor.record_pnl(Decimal::from_str("-50.0").unwrap()).await;
    monitor.record_pnl(Decimal::from_str("25.0").unwrap()).await;
    monitor.record_pnl(Decimal::from_str("-10.0").unwrap()).await;
    monitor.record_pnl(Decimal::from_str("15.0").unwrap()).await;
    
    // Await metrics - should have calculated P&L statistics
    let metrics = monitor.get_metrics().await;
    
    // Total P&L should be 80.0 (100 - 50 + 25 - 10 + 15)
    assert_eq!(metrics.total_pnl, Some(Decimal::from_str("80.0").unwrap()));
    
    // Win rate should be 60% (3 wins out of 5)
    assert!(metrics.win_rate.is_some());
    let win_rate = metrics.win_rate.unwrap();
    assert!((win_rate - 0.6).abs() < 0.01);
}

/// Test T117-10: reset_metrics async operation
#[tokio::test]
async fn test_t117_10_reset_metrics() {
    let monitor = PerformanceMonitor::new();
    
    // Record some data
    monitor.record_market_data_event().await;
    monitor.record_signal().await;
    monitor.record_order_placement("order_1").await;
    
    // Verify data was recorded
    let metrics = monitor.get_metrics().await;
    assert_eq!(metrics.market_data_events, 1);
    assert_eq!(metrics.signals_generated, 1);
    assert_eq!(metrics.orders_placed, 1);
    
    // Reset metrics
    monitor.reset_metrics().await;
    
    // Verify metrics were reset
    let metrics = monitor.get_metrics().await;
    assert_eq!(metrics.market_data_events, 0);
    assert_eq!(metrics.signals_generated, 0);
    assert_eq!(metrics.orders_placed, 0);
}

// =============================================================================
// T118: Rate calculation methods tests
// =============================================================================

/// Test T118-1: get_fill_rate method exists and works
#[tokio::test]
async fn test_t118_1_get_fill_rate() {
    let monitor = PerformanceMonitor::new();
    
    // Place 5 orders, fill 2
    for i in 0..5 {
        monitor.record_order_placement(&format!("order_{}", i)).await;
    }
    monitor.record_order_fill("order_0").await;
    monitor.record_order_fill("order_1").await;
    
    // Fill rate should be 40% (2/5 * 100)
    let fill_rate = monitor.get_fill_rate().await;
    assert!((fill_rate - 40.0).abs() < 0.01);
}

/// Test T118-2: get_cancellation_rate method exists and works
#[tokio::test]
async fn test_t118_2_get_cancellation_rate() {
    let monitor = PerformanceMonitor::new();
    
    // Place 5 orders, cancel 1
    for i in 0..5 {
        monitor.record_order_placement(&format!("order_{}", i)).await;
    }
    monitor.record_order_cancellation("order_2").await;
    
    // Cancellation rate should be 20% (1/5 * 100)
    let cancel_rate = monitor.get_cancellation_rate().await;
    assert!((cancel_rate - 20.0).abs() < 0.01);
}

/// Test T118-3: get_rejection_rate method exists and works
#[tokio::test]
async fn test_t118_3_get_rejection_rate() {
    let monitor = PerformanceMonitor::new();
    
    // Place 5 orders, reject 1
    for i in 0..5 {
        monitor.record_order_placement(&format!("order_{}", i)).await;
    }
    monitor.record_order_rejection("order_3").await;
    
    // Rejection rate should be 20% (1/5 * 100)
    let reject_rate = monitor.get_rejection_rate().await;
    assert!((reject_rate - 20.0).abs() < 0.01);
}

/// Test T118-4: Rate methods return 0 when no orders placed
#[tokio::test]
async fn test_t118_4_rate_methods_zero_orders() {
    let monitor = PerformanceMonitor::new();
    
    // No orders placed - all rates should be 0
    assert_eq!(monitor.get_fill_rate().await, 0.0);
    assert_eq!(monitor.get_cancellation_rate().await, 0.0);
    assert_eq!(monitor.get_rejection_rate().await, 0.0);
}

/// Test T118-5: Combined rate calculations
#[tokio::test]
async fn test_t118_5_combined_rates() {
    let monitor = PerformanceMonitor::new();
    
    // Place 10 orders
    for i in 0..10 {
        monitor.record_order_placement(&format!("order_{}", i)).await;
    }
    
    // Fill 5, cancel 2, reject 1 (2 still pending)
    for i in 0..5 {
        monitor.record_order_fill(&format!("order_{}", i)).await;
    }
    for i in 5..7 {
        monitor.record_order_cancellation(&format!("order_{}", i)).await;
    }
    monitor.record_order_rejection("order_7").await;
    
    let fill_rate = monitor.get_fill_rate().await;
    let cancel_rate = monitor.get_cancellation_rate().await;
    let reject_rate = monitor.get_rejection_rate().await;
    
    // Fill rate: 50% (5/10)
    assert!((fill_rate - 50.0).abs() < 0.01);
    
    // Cancel rate: 20% (2/10)
    assert!((cancel_rate - 20.0).abs() < 0.01);
    
    // Reject rate: 10% (1/10)
    assert!((reject_rate - 10.0).abs() < 0.01);
}

// =============================================================================
// Integration tests
// =============================================================================

/// Test integration: Full workflow without deadlocks
#[tokio::test]
async fn test_integration_full_workflow() {
    let monitor = PerformanceMonitor::new();
    
    // Simulate trading session
    for i in 0..10 {
        monitor.record_market_data_event().await;
        
        if i % 3 == 0 {
            monitor.record_signal().await;
            monitor.record_order_placement(&format!("order_{}", i)).await;
            
            // Simulate various outcomes
            match i % 9 {
                0 => monitor.record_order_fill(&format!("order_{}", i)).await,
                3 => monitor.record_order_cancellation(&format!("order_{}", i)).await,
                6 => monitor.record_order_rejection(&format!("order_{}", i)).await,
                _ => {}
            }
        }
    }
    
    // Record P&L (this was the deadlock source)
    monitor.record_pnl(Decimal::from_str("50.0").unwrap()).await;
    monitor.record_pnl(Decimal::from_str("-20.0").unwrap()).await;
    monitor.record_pnl(Decimal::from_str("30.0").unwrap()).await;
    
    // Verify we can access all metrics without issues
    let metrics = monitor.get_metrics().await;
    let _fill_rate = monitor.get_fill_rate().await;
    let _cancel_rate = monitor.get_cancellation_rate().await;
    let _reject_rate = monitor.get_rejection_rate().await;
    
    // Basic sanity checks
    assert!(metrics.market_data_events > 0);
    assert!(metrics.signals_generated > 0);
    assert!(metrics.total_pnl.is_some());
}

/// Test concurrent access (stress test for deadlocks)
#[tokio::test]
async fn test_concurrent_access() {
    use std::sync::Arc;
    
    let monitor = Arc::new(PerformanceMonitor::new());
    
    let mut handles = vec![];
    
    // Spawn multiple tasks that access the monitor concurrently
    for i in 0..5 {
        let monitor_clone = Arc::clone(&monitor);
        let handle = tokio::spawn(async move {
            for j in 0..10 {
                monitor_clone.record_market_data_event().await;
                monitor_clone.record_signal().await;
                monitor_clone.record_order_placement(&format!("order_{}_{}", i, j)).await;
                
                if j % 2 == 0 {
                    monitor_clone.record_order_fill(&format!("order_{}_{}", i, j)).await;
                }
                
                // This was previously causing deadlocks
                if j % 3 == 0 {
                    monitor_clone.record_pnl(Decimal::from(10 * (i as i32 + 1))).await;
                }
                
                // Access rates concurrently
                let _ = monitor_clone.get_fill_rate().await;
                let _ = monitor_clone.get_metrics().await;
            }
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete (would hang if deadlock)
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Verify final state
    let metrics = monitor.get_metrics().await;
    assert_eq!(metrics.market_data_events, 50); // 5 tasks * 10 events
    assert_eq!(metrics.signals_generated, 50);
    assert_eq!(metrics.orders_placed, 50);
}

