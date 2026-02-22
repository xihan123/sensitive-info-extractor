use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
pub enum FileStatus {
    #[default]
    Pending,
    Processing(u8),
    Completed,
    Error(String),
}


impl FileStatus {
    pub fn processing(progress: u8) -> Self {
        Self::Processing(progress.min(100))
    }

    pub fn completed() -> Self {
        Self::Completed
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::Error(message.into())
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub file_path: PathBuf,
    pub file_name: String,
    pub columns: Vec<String>,
    pub row_count: u32,
    pub status: FileStatus,
    pub selected: bool,
}

impl FileInfo {
    pub fn from_path(path: PathBuf) -> Self {
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        Self {
            file_path: path,
            file_name,
            columns: Vec::new(),
            row_count: 0,
            status: FileStatus::Pending,
            selected: true,
        }
    }
}