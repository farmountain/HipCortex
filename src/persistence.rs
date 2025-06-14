use crate::memory_record::MemoryRecord;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm,
};
use anyhow::Result;
#[cfg(feature = "async-store")]
use async_trait::async_trait;
use base64::Engine as _;
use rand::RngCore;
use std::io::{BufRead, BufReader, Write};
#[cfg(feature = "async-store")]
use tokio::fs::File as AsyncFile;
#[cfg(feature = "async-store")]
use tokio::io::{
    AsyncBufReadExt, AsyncWriteExt, BufReader as AsyncBufReader, BufWriter as AsyncBufWriter,
};

pub trait MemoryBackend {
    fn load(&mut self) -> Result<Vec<MemoryRecord>>;
    fn append(&mut self, record: &MemoryRecord) -> Result<()>;
    fn flush(&mut self) -> Result<()>;
    fn clear(&mut self) -> Result<()>;
}

#[cfg(feature = "async-store")]
#[async_trait]
pub trait AsyncMemoryBackend {
    async fn load(&mut self) -> Result<Vec<MemoryRecord>>;
    async fn append(&mut self, record: &MemoryRecord) -> Result<()>;
    async fn flush(&mut self) -> Result<()>;
    async fn clear(&mut self) -> Result<()>;
}

pub struct FileBackend {
    path: std::path::PathBuf,
    wal: std::path::PathBuf,
    writer: Option<std::io::BufWriter<std::fs::File>>,
    cipher: Option<aes_gcm::Aes256Gcm>,
    envelope_path: Option<std::path::PathBuf>,
    compress: bool,
}

impl FileBackend {
    pub fn new<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let p = path.as_ref().to_path_buf();
        let wal = p.with_extension("wal");
        Ok(Self {
            path: p,
            wal,
            writer: None,
            cipher: None,
            envelope_path: None,
            compress: false,
        })
    }

    pub fn new_compressed<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let p = path.as_ref().to_path_buf();
        let wal = p.with_extension("wal");
        Ok(Self {
            path: p,
            wal,
            writer: None,
            cipher: None,
            envelope_path: None,
            compress: true,
        })
    }

    pub fn new_encrypted<P: AsRef<std::path::Path>>(path: P, key: [u8; 32]) -> Result<Self> {
        let cipher = aes_gcm::Aes256Gcm::new(
            &aes_gcm::aead::generic_array::GenericArray::clone_from_slice(&key),
        );
        let p = path.as_ref().to_path_buf();
        let wal = p.with_extension("wal");
        Ok(Self {
            path: p,
            wal,
            writer: None,
            cipher: Some(cipher),
            envelope_path: None,
            compress: true,
        })
    }

    pub fn new_encrypted_envelope<P: AsRef<std::path::Path>>(
        path: P,
        master_key: [u8; 32],
    ) -> Result<Self> {
        let p = path.as_ref().to_path_buf();
        let wal = p.with_extension("wal");
        let sk_path = p.with_extension("sk");
        let master = Aes256Gcm::new(
            &aes_gcm::aead::generic_array::GenericArray::clone_from_slice(&master_key),
        );
        let mut session_key = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut session_key);
        let mut nonce_bytes = [0u8; 12];
        rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);
        let ciphertext = master
            .encrypt(nonce, session_key.as_ref())
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let mut sk_file = std::fs::File::create(&sk_path)?;
        let encoded = base64::engine::general_purpose::STANDARD
            .encode([nonce_bytes.to_vec(), ciphertext].concat());
        sk_file.write_all(encoded.as_bytes())?;
        let cipher = Aes256Gcm::new(
            &aes_gcm::aead::generic_array::GenericArray::clone_from_slice(&session_key),
        );
        Ok(Self {
            path: p,
            wal,
            writer: None,
            cipher: Some(cipher),
            envelope_path: Some(sk_path),
            compress: true,
        })
    }

    pub fn open_encrypted_envelope<P: AsRef<std::path::Path>>(
        path: P,
        master_key: [u8; 32],
    ) -> Result<Self> {
        let p = path.as_ref().to_path_buf();
        let wal = p.with_extension("wal");
        let sk_path = p.with_extension("sk");
        let sk_data = std::fs::read_to_string(&sk_path)?;
        let bytes = base64::engine::general_purpose::STANDARD.decode(sk_data.trim())?;
        let (nonce_bytes, cipher_bytes) = bytes.split_at(12);
        let master = Aes256Gcm::new(
            &aes_gcm::aead::generic_array::GenericArray::clone_from_slice(&master_key),
        );
        let nonce = aes_gcm::Nonce::from_slice(nonce_bytes);
        let session_key = master
            .decrypt(nonce, cipher_bytes)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let cipher = Aes256Gcm::new(
            &aes_gcm::aead::generic_array::GenericArray::clone_from_slice(&session_key),
        );
        Ok(Self {
            path: p,
            wal,
            writer: None,
            cipher: Some(cipher),
            envelope_path: Some(sk_path),
            compress: true,
        })
    }
}

impl MemoryBackend for FileBackend {
    fn load(&mut self) -> Result<Vec<MemoryRecord>> {
        let mut records = Vec::new();
        if self.path.exists() {
            let file = std::fs::File::open(&self.path)?;
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                if let Some(cipher) = &self.cipher {
                    #[derive(serde::Deserialize)]
                    struct EncLine {
                        nonce: String,
                        data: String,
                    }
                    let enc: EncLine = serde_json::from_str(&line)?;
                    let nonce = base64::engine::general_purpose::STANDARD.decode(enc.nonce)?;
                    let data = base64::engine::general_purpose::STANDARD.decode(enc.data)?;
                    let nonce = aes_gcm::Nonce::from_slice(&nonce);
                    let plain = cipher
                        .decrypt(nonce, data.as_ref())
                        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                    let decompressed = zstd::stream::decode_all(&plain[..])?;
                    let rec: MemoryRecord = serde_json::from_slice(&decompressed)?;
                    records.push(rec);
                } else if self.compress {
                    let bytes = base64::engine::general_purpose::STANDARD.decode(line)?;
                    let decompressed = zstd::stream::decode_all(&bytes[..])?;
                    let rec: MemoryRecord = serde_json::from_slice(&decompressed)?;
                    records.push(rec);
                } else {
                    let rec: MemoryRecord = serde_json::from_str(&line)?;
                    records.push(rec);
                }
            }
        }
        if self.wal.exists() {
            let file = std::fs::File::open(&self.wal)?;
            let reader = std::io::BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                let rec: MemoryRecord = serde_json::from_str(&line)?;
                records.push(rec);
            }
            std::fs::remove_file(&self.wal)?;
        }
        Ok(records)
    }

    fn append(&mut self, record: &MemoryRecord) -> Result<()> {
        if self.writer.is_none() {
            self.writer = Some(std::io::BufWriter::new(
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.path)?,
            ));
        }
        let mut writer = self.writer.as_mut().unwrap();
        let data = serde_json::to_vec(record)?;
        if let Some(cipher) = &self.cipher {
            let compressed = zstd::stream::encode_all(&data[..], 0)?;
            let mut nonce_bytes = [0u8; 12];
            rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
            let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);
            let ciphertext = cipher
                .encrypt(nonce, compressed.as_ref())
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            #[derive(serde::Serialize)]
            struct EncLine<'a> {
                nonce: &'a str,
                data: &'a str,
            }
            let enc = EncLine {
                nonce: &base64::engine::general_purpose::STANDARD.encode(nonce_bytes),
                data: &base64::engine::general_purpose::STANDARD.encode(ciphertext),
            };
            serde_json::to_writer(&mut writer, &enc)?;
            writer.write_all(b"\n")?;
        } else if self.compress {
            let compressed = zstd::stream::encode_all(&data[..], 0)?;
            let encoded = base64::engine::general_purpose::STANDARD.encode(compressed);
            writer.write_all(encoded.as_bytes())?;
            writer.write_all(b"\n")?;
        } else {
            writer.write_all(&data)?;
            writer.write_all(b"\n")?;
        }
        // append to WAL
        let mut wal = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.wal)?;
        serde_json::to_writer(&mut wal, record)?;
        wal.write_all(b"\n")?;
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        if let Some(w) = self.writer.as_mut() {
            w.flush()?;
        }
        if self.wal.exists() {
            std::fs::remove_file(&self.wal)?;
        }
        Ok(())
    }

    fn clear(&mut self) -> Result<()> {
        if self.path.exists() {
            std::fs::remove_file(&self.path)?;
        }
        if self.wal.exists() {
            std::fs::remove_file(&self.wal)?;
        }
        if let Some(ref sk) = self.envelope_path {
            if sk.exists() {
                std::fs::remove_file(sk)?;
            }
        }
        self.writer = None;
        Ok(())
    }
}

#[cfg(feature = "async-store")]
pub struct AsyncFileBackend {
    path: std::path::PathBuf,
    wal: std::path::PathBuf,
    writer: Option<AsyncBufWriter<AsyncFile>>,
    cipher: Option<aes_gcm::Aes256Gcm>,
    envelope_path: Option<std::path::PathBuf>,
    compress: bool,
}

#[cfg(feature = "async-store")]
impl AsyncFileBackend {
    pub async fn new<P: AsRef<std::path::Path>>(path: P, compress: bool) -> Result<Self> {
        let p = path.as_ref().to_path_buf();
        let wal = p.with_extension("wal");
        Ok(Self {
            path: p,
            wal,
            writer: None,
            cipher: None,
            envelope_path: None,
            compress,
        })
    }

    pub async fn new_compressed<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        Self::new(path, true).await
    }

    pub async fn new_encrypted<P: AsRef<std::path::Path>>(path: P, key: [u8; 32]) -> Result<Self> {
        let cipher =
            Aes256Gcm::new(&aes_gcm::aead::generic_array::GenericArray::clone_from_slice(&key));
        let mut this = Self::new(path, true).await?;
        this.cipher = Some(cipher);
        Ok(this)
    }

    pub async fn new_encrypted_envelope<P: AsRef<std::path::Path>>(
        path: P,
        master_key: [u8; 32],
    ) -> Result<Self> {
        let p = path.as_ref().to_path_buf();
        let wal = p.with_extension("wal");
        let sk_path = p.with_extension("sk");
        let master = Aes256Gcm::new(
            &aes_gcm::aead::generic_array::GenericArray::clone_from_slice(&master_key),
        );
        let mut session_key = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut session_key);
        let mut nonce_bytes = [0u8; 12];
        rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);
        let ciphertext = master
            .encrypt(nonce, session_key.as_ref())
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        tokio::fs::write(
            &sk_path,
            base64::engine::general_purpose::STANDARD
                .encode([nonce_bytes.to_vec(), ciphertext].concat()),
        )
        .await?;
        let cipher = Aes256Gcm::new(
            &aes_gcm::aead::generic_array::GenericArray::clone_from_slice(&session_key),
        );
        Ok(Self {
            path: p,
            wal,
            writer: None,
            cipher: Some(cipher),
            envelope_path: Some(sk_path),
            compress: true,
        })
    }

    pub async fn open_encrypted_envelope<P: AsRef<std::path::Path>>(
        path: P,
        master_key: [u8; 32],
    ) -> Result<Self> {
        let p = path.as_ref().to_path_buf();
        let wal = p.with_extension("wal");
        let sk_path = p.with_extension("sk");
        let sk_data = tokio::fs::read_to_string(&sk_path).await?;
        let bytes = base64::engine::general_purpose::STANDARD.decode(sk_data.trim())?;
        let (nonce_bytes, cipher_bytes) = bytes.split_at(12);
        let master = Aes256Gcm::new(
            &aes_gcm::aead::generic_array::GenericArray::clone_from_slice(&master_key),
        );
        let nonce = aes_gcm::Nonce::from_slice(nonce_bytes);
        let session_key = master
            .decrypt(nonce, cipher_bytes)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let cipher = Aes256Gcm::new(
            &aes_gcm::aead::generic_array::GenericArray::clone_from_slice(&session_key),
        );
        Ok(Self {
            path: p,
            wal,
            writer: None,
            cipher: Some(cipher),
            envelope_path: Some(sk_path),
            compress: true,
        })
    }
}

#[cfg(feature = "async-store")]
#[async_trait]
impl AsyncMemoryBackend for AsyncFileBackend {
    async fn load(&mut self) -> Result<Vec<MemoryRecord>> {
        let mut records = Vec::new();
        if self.path.exists() {
            let file = AsyncFile::open(&self.path).await?;
            let mut reader = AsyncBufReader::new(file);
            let mut line = String::new();
            while reader.read_line(&mut line).await? > 0 {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    if let Some(cipher) = &self.cipher {
                        #[derive(serde::Deserialize)]
                        struct EncLine {
                            nonce: String,
                            data: String,
                        }
                        let enc: EncLine = serde_json::from_str(trimmed)?;
                        let nonce = base64::engine::general_purpose::STANDARD.decode(enc.nonce)?;
                        let data = base64::engine::general_purpose::STANDARD.decode(enc.data)?;
                        let nonce = aes_gcm::Nonce::from_slice(&nonce);
                        let plain = cipher
                            .decrypt(nonce, data.as_ref())
                            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                        let decompressed = zstd::stream::decode_all(&plain[..])?;
                        let rec: MemoryRecord = serde_json::from_slice(&decompressed)?;
                        records.push(rec);
                    } else if self.compress {
                        let bytes = base64::engine::general_purpose::STANDARD.decode(trimmed)?;
                        let decompressed = zstd::stream::decode_all(&bytes[..])?;
                        let rec: MemoryRecord = serde_json::from_slice(&decompressed)?;
                        records.push(rec);
                    } else {
                        let rec: MemoryRecord = serde_json::from_str(trimmed)?;
                        records.push(rec);
                    }
                }
                line.clear();
            }
        }
        if self.wal.exists() {
            let file = AsyncFile::open(&self.wal).await?;
            let mut reader = AsyncBufReader::new(file);
            let mut line = String::new();
            while reader.read_line(&mut line).await? > 0 {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    let rec: MemoryRecord = serde_json::from_str(trimmed)?;
                    records.push(rec);
                }
                line.clear();
            }
            tokio::fs::remove_file(&self.wal).await?;
        }
        Ok(records)
    }

    async fn append(&mut self, record: &MemoryRecord) -> Result<()> {
        if self.writer.is_none() {
            let file = AsyncFile::create(&self.path).await?;
            self.writer = Some(AsyncBufWriter::new(file));
        }
        let writer = self.writer.as_mut().unwrap();
        let data = serde_json::to_vec(record)?;
        if let Some(cipher) = &self.cipher {
            let compressed = zstd::stream::encode_all(&data[..], 0)?;
            let mut nonce_bytes = [0u8; 12];
            rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
            let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);
            let ciphertext = cipher
                .encrypt(nonce, compressed.as_ref())
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            #[derive(serde::Serialize)]
            struct EncLine<'a> {
                nonce: &'a str,
                data: &'a str,
            }
            let enc = EncLine {
                nonce: &base64::engine::general_purpose::STANDARD.encode(nonce_bytes),
                data: &base64::engine::general_purpose::STANDARD.encode(ciphertext),
            };
            let enc_bytes = serde_json::to_vec(&enc)?;
            writer.write_all(&enc_bytes).await?;
            writer.write_all(b"\n").await?;
        } else if self.compress {
            let compressed = zstd::stream::encode_all(&data[..], 0)?;
            let encoded = base64::engine::general_purpose::STANDARD.encode(compressed);
            writer.write_all(encoded.as_bytes()).await?;
            writer.write_all(b"\n").await?;
        } else {
            writer.write_all(&data).await?;
            writer.write_all(b"\n").await?;
        }
        let mut wal = AsyncFile::options()
            .create(true)
            .append(true)
            .open(&self.wal)
            .await?;
        wal.write_all(&data).await?;
        wal.write_all(b"\n").await?;
        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        if let Some(w) = self.writer.as_mut() {
            w.flush().await?;
        }
        if self.wal.exists() {
            tokio::fs::remove_file(&self.wal).await?;
        }
        Ok(())
    }

    async fn clear(&mut self) -> Result<()> {
        if self.path.exists() {
            tokio::fs::remove_file(&self.path).await?;
        }
        if self.wal.exists() {
            tokio::fs::remove_file(&self.wal).await?;
        }
        if let Some(ref sk) = self.envelope_path {
            if sk.exists() {
                tokio::fs::remove_file(sk).await?;
            }
        }
        self.writer = None;
        Ok(())
    }
}
