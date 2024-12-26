use super::inscription::{Inscription, InscriptionParser, InscriptionType};
use bitcoin::Block;
use rayon::prelude::*;
use std::sync::Arc;
use log::info;
use num_cpus;

pub struct ParallelParser {
    parser: Arc<InscriptionParser>,
    batch_size: usize,
    thread_count: usize,
}

impl ParallelParser {
    pub fn new(batch_size: usize) -> Self {
        // Get the number of physical CPU cores
        // M1 has 8 cores (4 performance + 4 efficiency)
        let thread_count = num_cpus::get_physical();
        info!("Initializing parallel parser with {} threads", thread_count);
        
        Self {
            parser: Arc::new(InscriptionParser::new()),
            batch_size,
            thread_count,
        }
    }

    pub fn process_blocks(&self, blocks: Vec<Block>) -> Vec<String> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.thread_count)
            .build()
            .unwrap();

        info!("Processing {} blocks in parallel using {} threads", blocks.len(), self.thread_count);
        
        pool.install(|| {
            blocks
                .par_chunks(self.batch_size)
                .flat_map(|chunk| {
                    chunk.par_iter()
                        .flat_map(|block| self.process_block(block))
                        .collect::<Vec<_>>()
                })
                .collect()
        })
    }

    fn process_block(&self, block: &Block) -> Vec<String> {
        block.txdata
            .par_iter()
            .filter_map(|tx| {
                if let Some(inscription) = self.parser.parse_transaction(tx) {
                    if let InscriptionType::Text(text) = inscription.content {
                        // Only collect text inscriptions that might be interesting
                        if text.contains("Chancellor") || 
                           text.contains("bank") || 
                           text.contains("Times") ||
                           text.contains("bailout") {
                            info!("Found relevant inscription text: {}", text);
                            return Some(text);
                        }
                    }
                }
                None
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::{Transaction, locktime::absolute::LockTime};

    fn create_test_block(num_txs: usize) -> Block {
        // Create a dummy block with the specified number of transactions
        let txdata = (0..num_txs)
            .map(|_| Transaction {
                version: 1,
                lock_time: LockTime::ZERO,
                input: vec![],
                output: vec![],
            })
            .collect();

        // For testing purposes only - create a zeroed header since we only care about transaction parsing
        let header = unsafe { std::mem::zeroed() };
        Block { header, txdata }
    }

    #[test]
    fn test_parallel_processing() {
        let parser = ParallelParser::new(100);
        let blocks = vec![
            create_test_block(10),
            create_test_block(20),
            create_test_block(30),
        ];

        let inscriptions = parser.process_blocks(blocks);
        
        // In this test case, we don't expect any inscriptions since we used dummy transactions
        assert_eq!(inscriptions.len(), 0);
    }
}
