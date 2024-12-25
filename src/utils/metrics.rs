use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub struct Metrics {
    blocks_processed: AtomicU64,
    inscriptions_found: AtomicU64,
    processing_time: AtomicU64,
    start_time: Instant,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            blocks_processed: AtomicU64::new(0),
            inscriptions_found: AtomicU64::new(0),
            processing_time: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    pub fn increment_blocks(&self, count: u64) {
        self.blocks_processed.fetch_add(count, Ordering::Relaxed);
    }

    pub fn increment_inscriptions(&self, count: u64) {
        self.inscriptions_found.fetch_add(count, Ordering::Relaxed);
    }

    pub fn add_processing_time(&self, duration: Duration) {
        self.processing_time.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> MetricsSnapshot {
        let blocks = self.blocks_processed.load(Ordering::Relaxed);
        let inscriptions = self.inscriptions_found.load(Ordering::Relaxed);
        let processing_time = Duration::from_micros(
            self.processing_time.load(Ordering::Relaxed)
        );
        let total_time = self.start_time.elapsed();

        MetricsSnapshot {
            blocks_processed: blocks,
            inscriptions_found: inscriptions,
            processing_time,
            total_time,
            blocks_per_second: blocks as f64 / total_time.as_secs_f64(),
            inscriptions_per_block: if blocks > 0 {
                inscriptions as f64 / blocks as f64
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug)]
pub struct MetricsSnapshot {
    pub blocks_processed: u64,
    pub inscriptions_found: u64,
    pub processing_time: Duration,
    pub total_time: Duration,
    pub blocks_per_second: f64,
    pub inscriptions_per_block: f64,
}

impl std::fmt::Display for MetricsSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Performance Metrics:")?;
        writeln!(f, "  Blocks Processed: {}", self.blocks_processed)?;
        writeln!(f, "  Inscriptions Found: {}", self.inscriptions_found)?;
        writeln!(f, "  Processing Time: {:.2?}", self.processing_time)?;
        writeln!(f, "  Total Time: {:.2?}", self.total_time)?;
        writeln!(f, "  Blocks/Second: {:.2}", self.blocks_per_second)?;
        writeln!(f, "  Inscriptions/Block: {:.4}", self.inscriptions_per_block)?;
        Ok(())
    }
}