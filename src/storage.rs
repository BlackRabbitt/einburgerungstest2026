use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::fs;

pub async fn ensure_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
        .await
        .with_context(|| format!("failed to create directory {}", path.display()))
}

pub async fn read_json_file<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let text = fs::read_to_string(path)
        .await
        .with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))
}

pub async fn write_json_file<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent).await?;
    }

    let text = serde_json::to_string(value)?;
    fs::write(path, text)
        .await
        .with_context(|| format!("failed to write {}", path.display()))
}

pub async fn file_exists(path: &Path) -> bool {
    fs::metadata(path).await.is_ok()
}

pub fn root_data_dir() -> PathBuf {
    PathBuf::from("data")
}
