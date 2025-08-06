use std::io;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RfgrepError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("Clipboard error: {0}")]
    Clipboard(#[from] arboard::Error),

    #[error("Walkdir error: {0}")]
    Walkdir(#[from] walkdir::Error),

    #[error("Memory map error: {0}")]
    #[allow(dead_code)]
    Mmap(io::Error),

    #[error("Configuration error: {0}")]
    #[allow(dead_code)]
    Config(String),

    #[error("Failed to process file '{path}': {source}")]
    FileProcessing {
        path: PathBuf,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Invalid file extension: {0}")]
    #[allow(dead_code)]
    InvalidExtension(String),

    #[error("Binary file detected: {0}")]
    #[allow(dead_code)]
    BinaryFile(String),

    #[error("File too large: {path} ({size} MB)")]
    #[allow(dead_code)]
    FileTooLarge { path: String, size: f64 },

    #[error("An unexpected error occurred: {0}")]
    Other(String),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, RfgrepError>;
