use object_store::local::LocalFileSystem;
use slatedb::db::Db;
use slatedb::error::SlateDBError;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("SlateDB error: {0}")]
    SlateDb(#[from] SlateDBError),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Storage error: {0}")]
    Storage(String),
}

pub struct Database {
    inner: Db,
}

impl Database {
    pub async fn open(path: &str) -> Result<Self, DbError> {
        let path = Path::new(path);
        std::fs::create_dir_all(path).map_err(|e| DbError::Storage(e.to_string()))?;
        let object_store = Arc::new(
            LocalFileSystem::new_with_prefix(path)
                .map_err(|e| DbError::Storage(e.to_string()))?,
        );
        let db = Db::open("/", object_store).await?;
        Ok(Self { inner: db })
    }

    pub async fn put<T: serde::Serialize>(&self, key: &str, value: &T) -> Result<(), DbError> {
        let bytes = serde_json::to_vec(value)?;
        self.inner.put(key.as_bytes(), bytes.as_slice()).await?;
        Ok(())
    }

    pub async fn get<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
    ) -> Result<Option<T>, DbError> {
        match self.inner.get(key.as_bytes()).await? {
            Some(value) => {
                let item = serde_json::from_slice::<T>(&value)?;
                Ok(Some(item))
            }
            None => Ok(None),
        }
    }

    pub async fn delete(&self, key: &str) -> Result<(), DbError> {
        self.inner.delete(key.as_bytes()).await?;
        Ok(())
    }

    /// Scan all entries with a given prefix by maintaining an index key.
    ///
    /// This uses a convention where `"{prefix}__index"` stores a JSON array of
    /// all known keys for that prefix. Each key is stored individually.
    ///
    /// For example, prefix "rule:" uses index key "rule:__index".
    pub async fn scan_prefix<T: serde::de::DeserializeOwned>(
        &self,
        prefix: &str,
    ) -> Result<Vec<(String, T)>, DbError> {
        let index_key = format!("{}__index", prefix);
        let keys: Vec<String> = self.get(&index_key).await?.unwrap_or_default();

        let mut results = Vec::new();
        for key in keys {
            match self.get::<T>(&key).await? {
                Some(value) => results.push((key, value)),
                None => {
                    tracing::warn!("Index references missing key: {}", key);
                }
            }
        }

        Ok(results)
    }

    /// Add a key to the prefix index. Call this when inserting a new item.
    pub async fn add_to_index(&self, prefix: &str, key: &str) -> Result<(), DbError> {
        let index_key = format!("{}__index", prefix);
        let mut keys: Vec<String> = self.get(&index_key).await?.unwrap_or_default();
        if !keys.contains(&key.to_string()) {
            keys.push(key.to_string());
            self.put(&index_key, &keys).await?;
        }
        Ok(())
    }

    /// Remove a key from the prefix index. Call this when deleting an item.
    pub async fn remove_from_index(&self, prefix: &str, key: &str) -> Result<(), DbError> {
        let index_key = format!("{}__index", prefix);
        let mut keys: Vec<String> = self.get(&index_key).await?.unwrap_or_default();
        keys.retain(|k| k != key);
        self.put(&index_key, &keys).await?;
        Ok(())
    }

    pub async fn close(&self) -> Result<(), DbError> {
        self.inner.close().await?;
        Ok(())
    }
}
