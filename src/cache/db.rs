use super::Result;
use rocksdb::{DB, Options};
use std::path::Path;
use serde::{Serialize, de::DeserializeOwned};

pub struct CacheDb {
    db: DB,
}

impl CacheDb {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_compression_type(rocksdb::DBCompressionType::Snappy);
        opts.set_write_buffer_size(64 * 1024 * 1024); // 64MB write buffer
        
        let db = DB::open(&opts, path)?;
        Ok(Self { db })
    }

    pub fn get<T: DeserializeOwned>(&self, key: &[u8]) -> Result<Option<T>> {
        match self.db.get(key)? {
            Some(data) => Ok(Some(bincode::deserialize(&data)?)),
            None => Ok(None),
        }
    }

    pub fn put<T: Serialize>(&self, key: &[u8], value: &T) -> Result<()> {
        let data = bincode::serialize(value)?;
        self.db.put(key, data)?;
        Ok(())
    }

    pub fn delete(&self, key: &[u8]) -> Result<()> {
        self.db.delete(key)?;
        Ok(())
    }

    pub fn batch_put<T: Serialize>(&self, items: &[(Vec<u8>, T)]) -> Result<()> {
        let mut batch = rocksdb::WriteBatch::default();
        
        for (key, value) in items {
            let data = bincode::serialize(value)?;
            batch.put(key, data);
        }

        self.db.write(batch)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use serde::{Serialize, Deserialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestData {
        id: u32,
        value: String,
    }

    #[test]
    fn test_cache_operations() {
        let temp_dir = TempDir::new().unwrap();
        let cache = CacheDb::new(temp_dir.path()).unwrap();

        let test_data = TestData {
            id: 1,
            value: "test".to_string(),
        };

        // Test put and get
        cache.put(b"key1", &test_data).unwrap();
        let retrieved: TestData = cache.get(b"key1").unwrap().unwrap();
        assert_eq!(retrieved, test_data);

        // Test delete
        cache.delete(b"key1").unwrap();
        assert!(cache.get::<TestData>(b"key1").unwrap().is_none());

        // Test batch put
        let items = vec![
            (b"key2".to_vec(), TestData { id: 2, value: "test2".to_string() }),
            (b"key3".to_vec(), TestData { id: 3, value: "test3".to_string() }),
        ];
        cache.batch_put(&items).unwrap();

        let retrieved2: TestData = cache.get(b"key2").unwrap().unwrap();
        let retrieved3: TestData = cache.get(b"key3").unwrap().unwrap();
        assert_eq!(retrieved2.id, 2);
        assert_eq!(retrieved3.id, 3);
    }
}