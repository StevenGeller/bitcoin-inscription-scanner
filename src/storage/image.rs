use super::Result;
use bitcoin::Txid;
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::Write;
use blake3::Hash;

pub struct ImageStorage {
    base_dir: PathBuf,
}

impl ImageStorage {
    pub fn new(base_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&base_dir)?;
        Ok(Self { base_dir })
    }

    pub fn store(&self, txid: Txid, mime_type: &str, data: &[u8]) -> Result<()> {
        let hash = blake3::hash(data);
        let filename = format!("{}-{}.bin", txid, hash);
        let path = self.base_dir.join(filename);
        
        let mut file = File::create(path)?;
        file.write_all(mime_type.as_bytes())?;
        file.write_all(b"\n")?;
        file.write_all(data)?;
        
        Ok(())
    }

    pub fn get(&self, txid: Txid, hash: Hash) -> Result<Option<(String, Vec<u8>)>> {
        let filename = format!("{}-{}.bin", txid, hash);
        let path = self.base_dir.join(filename);
        
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read(&path)?;
        let mut parts = content.splitn(2, |&b| b == b'\n');
        
        let mime_type = parts
            .next()
            .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
            .ok_or_else(|| super::StorageError::ImageError("Invalid mime type".to_string()))?;
            
        let data = parts
            .next()
            .ok_or_else(|| super::StorageError::ImageError("Invalid data".to_string()))?
            .to_vec();

        Ok(Some((mime_type, data)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_image_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ImageStorage::new(temp_dir.path().to_path_buf()).unwrap();

        let txid = Txid::default();
        let mime_type = "image/png";
        let data = vec![1, 2, 3, 4];
        
        storage.store(txid, mime_type, &data).unwrap();
        
        let hash = blake3::hash(&data);
        let (stored_mime_type, stored_data) = storage.get(txid, hash).unwrap().unwrap();
        
        assert_eq!(stored_mime_type, mime_type);
        assert_eq!(stored_data, data);
    }
}