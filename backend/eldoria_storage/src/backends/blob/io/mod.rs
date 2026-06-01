use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use async_trait::async_trait;
use bytes::Bytes;
use tokio::io::AsyncWriteExt;

use crate::{BlobStore, StorageError, models::BlobId};

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub struct FsBlobStore {
    blobs_dir: PathBuf, // {data_directory}/blobs
    tmp_dir: PathBuf,   // {data_directory}/tmp
}

impl FsBlobStore {
    /// Create the store and scaffold its directories under `data_directory`.
    pub async fn scaffold(data_directory: impl Into<PathBuf>) -> Result<Self, StorageError> {
        tracing::info!("Scaffolding data directories");
        let root = data_directory.into();
        let blobs_dir = root.join("blobs");
        let tmp_dir = root.join("tmp");
        tokio::fs::create_dir_all(&blobs_dir).await?;
        tokio::fs::create_dir_all(&tmp_dir).await?;

        let cleaned = Self::clean_tmp(&tmp_dir).await?;
        if cleaned > 0 {
            tracing::info!("Cleaned {cleaned} files from the tmp directory");
        }

        Ok(Self { blobs_dir, tmp_dir })
    }

    fn path_for(&self, id: &BlobId) -> PathBuf {
        let hex = id.as_hex();
        // sha256 hex is always 64 chars, so the 2-char shard prefix is safe.
        self.blobs_dir.join(&hex[..2]).join(hex)
    }

    async fn clean_tmp(tmp_dir: &PathBuf) -> Result<usize, StorageError> {
        let mut entries = match tokio::fs::read_dir(tmp_dir).await {
            Ok(entries) => entries,
            // No tmp/ yet (e.g. called before scaffold) just means nothing to clean.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(e) => return Err(e.into()),
        };

        let mut removed = 0;
        while let Some(entry) = entries.next_entry().await? {
            // Per-file failures are non-fatal: a straggler we can't unlink right now
            // (permissions, a racing process) just isn't counted, and the next sweep
            // catches it. Only the directory read itself propagates.
            if tokio::fs::remove_file(entry.path()).await.is_ok() {
                removed += 1;
            }
        }

        Ok(removed)
    }
}

#[async_trait]
impl BlobStore for FsBlobStore {
    async fn put(&self, bytes: &[u8]) -> Result<BlobId, StorageError> {
        let id = BlobId::from_content(bytes);

        // Dedup: identical bytes already on disk -> nothing to do.
        // The race against a concurrent put of the same content is benign:
        // both target the same destination with identical bytes, and rename is
        // atomic, so whoever lands second just replaces an identical file.
        if self.exists(&id).await? {
            return Ok(id);
        }

        let dest = self.path_for(&id);
        let shard_dir = dest.parent().expect("path_for always yields a parent");
        tokio::fs::create_dir_all(shard_dir).await?;

        // Stage in tmp/ — same filesystem as blobs/, so the rename stays atomic.
        let tmp_path = self.tmp_dir.join(format!(
            "{}-{}",
            std::process::id(),
            TMP_COUNTER.fetch_add(1, Ordering::Relaxed),
        ));

        let mut file = tokio::fs::File::create(&tmp_path).await?;
        file.write_all(bytes).await?;
        file.sync_all().await?; // bytes durable before we publish them
        drop(file); // close before rename (matters on Windows)

        tokio::fs::rename(&tmp_path, &dest).await?; // atomic publish

        // Best-effort: make the rename itself durable. Directory fsync isn't
        // supported on every platform (Windows), so don't fail the put on it.
        if let Ok(dir) = tokio::fs::File::open(shard_dir).await {
            let _ = dir.sync_all().await;
        }

        Ok(id)
    }

    async fn get(&self, id: &BlobId) -> Result<Option<Bytes>, StorageError> {
        match tokio::fs::read(self.path_for(id)).await {
            Ok(data) => Ok(Some(Bytes::from(data))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn exists(&self, id: &BlobId) -> Result<bool, StorageError> {
        Ok(tokio::fs::try_exists(self.path_for(id)).await?)
    }
}
