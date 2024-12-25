use super::Result;
use bitcoin::Txid;
use std::path::PathBuf;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write, BufRead, BufReader};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TextEntry {
    pub txid: String,
    pub content: String,
    pub timestamp: u64,
}

pub struct TextStorage {
    log_file: PathBuf,
}

impl TextStorage {
    pub fn new(log_file: PathBuf) -> Result<Self> {
        if let Some(parent) = log_file.parent() {
            fs::create_dir_all(parent)?;
        }
        
        if !log_file.exists() {
            File::create(&log_file)?;
        }
        
        Ok(Self { log_file })
    }

    pub fn store(&self, txid: Txid, content: &str) -> Result<()> {
        let entry = TextEntry {
            txid: txid.to_string(),
            content: content.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.log_file)?;
            
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &entry)?;
        writeln!(writer)?;
        writer.flush()?;
        
        Ok(())
    }

    pub fn read_entries(&self) -> Result<impl Iterator<Item = Result<TextEntry>>> {
        let file = File::open(&self.log_file)?;
        let reader = BufReader::new(file);
        
        Ok(reader.lines().map(|line| {
            line.map_err(|e| super::StorageError::IoError(e))
                .and_then(|l| {
                    serde_json::from_str(&l)
                        .map_err(|e| super::StorageError::TextError(e.to_string()))
                })
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_text_storage() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = TextStorage::new(temp_file.path().to_path_buf()).unwrap();

        let txid = Txid::default();
        let content = "Hello, Bitcoin!";
        
        storage.store(txid, content).unwrap();
        
        let entries: Vec<_> = storage.read_entries().unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content, content);
        assert_eq!(entries[0].txid, txid.to_string());
    }
}