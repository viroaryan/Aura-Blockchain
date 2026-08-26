use prometheus::{
    register_histogram, register_int_counter, register_int_gauge, Encoder, Histogram,
    IntCounter, IntGauge, Registry, TextEncoder,
};
use std::sync::LazyLock;

pub static BLOCK_HEIGHT: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!("aura_block_height", "Current committed block height").unwrap()
});

pub static MEMPOOL_TXS: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!("aura_mempool_transactions", "Current transactions in mempool").unwrap()
});

pub static ACTIVE_PEERS: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!("aura_active_peers", "Current connected active peers").unwrap()
});

pub static TX_PROCESSED_TOTAL: LazyLock<IntCounter> = LazyLock::new(|| {
    register_int_counter!("aura_transactions_processed_total", "Total processed transactions").unwrap()
});

pub static CONSENSUS_ROUND: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!("aura_consensus_round", "Current active consensus round").unwrap()
});

pub static BLOCK_TIME_SECONDS: LazyLock<Histogram> = LazyLock::new(|| {
    register_histogram!(
        "aura_block_time_seconds",
        "Time taken to produce and commit a block"
    )
    .unwrap()
});

pub fn gather_metrics() -> String {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collection() {
        BLOCK_HEIGHT.set(42);
        MEMPOOL_TXS.set(10);
        TX_PROCESSED_TOTAL.inc_by(5);

        let output = gather_metrics();
        assert!(output.contains("aura_block_height 42"));
        assert!(output.contains("aura_mempool_transactions 10"));
    }
}
