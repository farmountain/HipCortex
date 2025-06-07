use crate::memory_record::MemoryRecord;
use anyhow::Result;
use rocksdb::{IteratorMode, Options, DB};
use std::path::{Path, PathBuf};

pub struct RocksDbBackend {
    path: PathBuf,
    db: DB,
}

impl RocksDbBackend {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, &path)?;
        Ok(Self {
            path: path.as_ref().to_path_buf(),
            db,
        })
    }

    fn next_key(&self) -> Result<u64> {
        if let Some(value) = self.db.get(b"__counter__")? {
            let s = std::str::from_utf8(&value)?;
            Ok(s.parse().unwrap_or(0))
        } else {
            Ok(0)
        }
    }
}

impl crate::persistence::MemoryBackend for RocksDbBackend {
    fn load(&mut self) -> Result<Vec<MemoryRecord>> {
        let mut vec = Vec::new();
        for (key, value) in self.db.iterator(IteratorMode::Start) {
            if key.as_ref() == b"__counter__" {
                continue;
            }
            let rec: MemoryRecord = serde_json::from_slice(&value)?;
            vec.push(rec);
        }
        Ok(vec)
    }

    fn append(&mut self, record: &MemoryRecord) -> Result<()> {
        let mut counter = self.next_key()?;
        let key = counter.to_le_bytes();
        let value = serde_json::to_vec(record)?;
        self.db.put(key, value)?;
        counter += 1;
        self.db.put(b"__counter__", counter.to_string())?;
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }

    fn clear(&mut self) -> Result<()> {
        self.db.flush()?;
        drop(std::mem::take(&mut self.db));
        DB::destroy(&Options::default(), &self.path)?;
        self.db = DB::open_default(&self.path)?;
        Ok(())
    }
}
