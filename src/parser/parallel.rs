use super::inscription::{Inscription, InscriptionParser};
use bitcoin::Block;
use rayon::prelude::*;
use std::sync::Arc;

pub struct ParallelParser {
    parser: Arc<InscriptionParser>,
    batch_size: usize,
}

impl ParallelParser {
    pub fn new(batch_size: usize) -> Self {
        Self {
            parser: Arc::new(InscriptionParser::new()),
            batch_size,
        }
    }

    pub fn process_blocks(&self, blocks: Vec<Block>) -> Vec<Inscription> {
        blocks
            .par_chunks(self.batch_size)
            .flat_map(|chunk| {
                chunk.par_iter()
                    .flat_map(|block| self.process_block(block))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn process_block(&self, block: &Block) -> Vec<Inscription> {
        block.txdata
            .par_iter()
            .filter_map(|tx| self.parser.parse_transaction(tx))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::{Transaction, TxOut, Script};

    fn create_test_block(num_txs: usize) -> Block {
        // Create a dummy block with the specified number of transactions
        let txdata = (0..num_txs)
            .map(|_| Transaction {
                version: 1,
                lock_time: 0,
                input: vec![],
                output: vec![],
            })
            .collect();

        Block {
            header: bitcoin::BlockHeader::default(),
            txdata,
        }
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