use std::fs::{File};
use std::path::{Path, PathBuf};

use anyhow::Result;
use flate2::{write::GzEncoder, Compression};
use tar::Builder;

pub struct SnapshotManager;

impl SnapshotManager {
    pub fn save<P: AsRef<Path>>(source: P, tag: &str) -> Result<PathBuf> {
        let archive_path = source.as_ref().with_extension(format!("{}.tar.gz", tag));
        let tar_gz = File::create(&archive_path)?;
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = Builder::new(enc);
        tar.append_path(source.as_ref())?;
        tar.finish()?;
        Ok(archive_path)
    }
}
