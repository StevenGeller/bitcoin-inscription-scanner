use super::Result;
use bloom::BloomFilter;
use std::sync::RwLock;

pub struct BloomCache {
    filter: RwLock<BloomFilter>,
    size: usize,
    fp_rate: f64,
}

impl BloomCache {
    pub fn new(size: usize, fp_rate: f64) -> Self {
        Self {
            filter: RwLock::new(BloomFilter::new(size, fp_rate)),
            size,
            fp_rate,
        }
    }

    pub fn insert(&self, key: &[u8]) -> Result<()> {
        let mut filter = self.filter.write().map_err(|_| {
            super::CacheError::DbError(rocksdb::Error::new(
                "Failed to acquire write lock for bloom filter"
            ))
        })?;
        filter.insert(key);
        Ok(())
    }

    pub fn contains(&self, key: &[u8]) -> Result<bool> {
        let filter = self.filter.read().map_err(|_| {
            super::CacheError::DbError(rocksdb::Error::new(
                "Failed to acquire read lock for bloom filter"
            ))
        })?;
        Ok(filter.contains(key))
    }

    pub fn clear(&self) -> Result<()> {
        let mut filter = self.filter.write().map_err(|_| {
            super::CacheError::DbError(rocksdb::Error::new(
                "Failed to acquire write lock for bloom filter"
            ))
        })?;
        *filter = BloomFilter::new(self.size, self.fp_rate);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bloom_cache() {
        let cache = BloomCache::new(1000, 0.01);
        
        // Test insert and contains
        cache.insert(b"test1").unwrap();
        cache.insert(b"test2").unwrap();
        
        assert!(cache.contains(b"test1").unwrap());
        assert!(cache.contains(b"test2").unwrap());
        assert!(!cache.contains(b"test3").unwrap());
        
        // Test clear
        cache.clear().unwrap();
        assert!(!cache.contains(b"test1").unwrap());
        assert!(!cache.contains(b"test2").unwrap());
    }
}