//! Optimized memory-mapped I/O with proper error handling and resource management
use crate::error::{Result as RfgrepResult, RfgrepError};
use crate::performance::memory_pool::{MemoryPool, MemoryPoolError};
use log::{debug, warn};
use memmap2::Mmap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

/// Configuration for memory-mapped I/O
#[derive(Debug, Clone)]
pub struct MmapConfig {
    /// Minimum file size to use memory mapping (in bytes)
    pub min_file_size: u64,
    /// Maximum file size to use memory mapping (in bytes)
    pub max_file_size: u64,
    /// Enable memory pool for caching
    pub enable_pool: bool,
    /// Memory pool size
    pub pool_size: usize,
    /// Memory pool age limit (seconds)
    pub pool_age_limit: u64,
    /// Memory pressure threshold (bytes)
    pub memory_threshold: usize,
}

impl Default for MmapConfig {
    fn default() -> Self {
        Self {
            min_file_size: 16 * 1024 * 1024,   // 16MB
            max_file_size: 1024 * 1024 * 1024, // 1GB
            enable_pool: true,
            pool_size: 100,
            pool_age_limit: 300,                  // 5 minutes
            memory_threshold: 1024 * 1024 * 1024, // 1GB
        }
    }
}

/// Optimized memory-mapped I/O handler
pub struct OptimizedMmapHandler {
    config: MmapConfig,
    memory_pool: Option<Arc<MemoryPool>>,
}

impl OptimizedMmapHandler {
    /// Create a new optimized memory-mapped I/O handler
    pub fn new(config: MmapConfig) -> Self {
        let memory_pool = if config.enable_pool {
            Some(Arc::new(MemoryPool::new(
                config.pool_size,
                config.pool_age_limit,
                config.memory_threshold,
            )))
        } else {
            None
        };

        Self {
            config,
            memory_pool,
        }
    }

    /// Read file content using the optimal strategy
    pub fn read_file(&self, path: &Path) -> RfgrepResult<FileContent> {
        let start = Instant::now();
        let metadata = std::fs::metadata(path).map_err(|e| {
            RfgrepError::Io(std::io::Error::other(format!(
                "Failed to get metadata for {:?}: {}",
                path, e
            )))
        })?;

        let file_size = metadata.len();
        debug!("Reading file {:?} ({} bytes)", path, file_size);

        // Determine the best strategy
        let strategy = self.choose_strategy(file_size);
        debug!("Chosen strategy for {:?}: {:?}", path, strategy);

        let content = match strategy {
            ReadStrategy::MemoryMapped => self.read_with_mmap(path, file_size)?,
            ReadStrategy::Buffered => self.read_with_buffered(path)?,
            ReadStrategy::Streaming => self.read_with_streaming(path)?,
        };

        let duration = start.elapsed();
        debug!("File read completed in {:?}", duration);

        Ok(content)
    }

    /// Choose the optimal reading strategy based on file characteristics
    fn choose_strategy(&self, file_size: u64) -> ReadStrategy {
        if file_size < self.config.min_file_size {
            ReadStrategy::Buffered
        } else if file_size > self.config.max_file_size {
            ReadStrategy::Streaming
        } else {
            ReadStrategy::MemoryMapped
        }
    }

    /// Read file using memory mapping
    fn read_with_mmap(&self, path: &Path, _file_size: u64) -> RfgrepResult<FileContent> {
        // Try to get from memory pool first
        if let Some(pool) = &self.memory_pool {
            match pool.get_mmap(&path.to_path_buf()) {
                Ok(mmap) => {
                    debug!("Using cached memory map for {:?}", path);
                    return Ok(FileContent::MemoryMapped(mmap));
                }
                Err(MemoryPoolError::MemoryPressure(_)) => {
                    warn!(
                        "Memory pressure detected, falling back to streaming for {:?}",
                        path
                    );
                    return self.read_with_streaming(path);
                }
                Err(e) => {
                    debug!(
                        "Memory pool error for {:?}: {}, falling back to direct mmap",
                        path, e
                    );
                }
            }
        }

        // Create new memory map
        let file = File::open(path).map_err(|e| {
            RfgrepError::Io(std::io::Error::other(format!(
                "Failed to open file {:?}: {}",
                path, e
            )))
        })?;

        let mmap = unsafe { Mmap::map(&file) }.map_err(|e| {
            RfgrepError::Io(std::io::Error::other(format!(
                "Failed to create memory map for {:?}: {}",
                path, e
            )))
        })?;

        debug!("Created new memory map for {:?}", path);
        Ok(FileContent::MemoryMapped(Arc::new(mmap)))
    }

    /// Read file using buffered I/O
    fn read_with_buffered(&self, path: &Path) -> RfgrepResult<FileContent> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            RfgrepError::Io(std::io::Error::other(format!(
                "Failed to read file {:?}: {}",
                path, e
            )))
        })?;

        debug!("Read file with buffered I/O: {:?}", path);
        Ok(FileContent::String(content))
    }

    /// Read file using streaming I/O
    fn read_with_streaming(&self, path: &Path) -> RfgrepResult<FileContent> {
        let file = File::open(path).map_err(|e| {
            RfgrepError::Io(std::io::Error::other(format!(
                "Failed to open file {:?}: {}",
                path, e
            )))
        })?;

        let reader = BufReader::new(file);
        debug!("Using streaming I/O for {:?}", path);
        Ok(FileContent::Streaming(reader))
    }

    /// Get memory pool statistics
    pub fn get_pool_stats(&self) -> Option<crate::performance::memory_pool::MemoryPoolStats> {
        self.memory_pool.as_ref().map(|pool| pool.get_stats())
    }

    /// Clean up memory pool
    pub fn cleanup(&self) {
        if let Some(pool) = &self.memory_pool {
            pool.cleanup();
        }
    }

    /// Remove file from memory pool
    pub fn remove_from_pool(&self, path: &Path) {
        if let Some(pool) = &self.memory_pool {
            pool.remove_file(&path.to_path_buf());
        }
    }
}

/// File content representation
pub enum FileContent {
    /// Memory-mapped content
    MemoryMapped(Arc<Mmap>),
    /// String content
    String(String),
    /// Streaming reader
    Streaming(BufReader<File>),
}

impl FileContent {
    /// Get content as bytes
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            FileContent::MemoryMapped(mmap) => mmap,
            FileContent::String(s) => s.as_bytes(),
            FileContent::Streaming(_) => {
                // For streaming, we can't provide bytes directly
                // This would need to be handled differently in practice
                &[]
            }
        }
    }

    /// Get content as string slice
    pub fn as_str(&self) -> RfgrepResult<&str> {
        match self {
            FileContent::MemoryMapped(mmap) => std::str::from_utf8(mmap)
                .map_err(|e| RfgrepError::Other(format!("Invalid UTF-8: {}", e))),
            FileContent::String(s) => Ok(s),
            FileContent::Streaming(_) => Err(RfgrepError::Other(
                "Cannot get string from streaming content".to_string(),
            )),
        }
    }

    /// Check if content is memory-mapped
    pub fn is_memory_mapped(&self) -> bool {
        matches!(self, FileContent::MemoryMapped(_))
    }

    /// Get content size
    pub fn len(&self) -> usize {
        match self {
            FileContent::MemoryMapped(mmap) => mmap.len(),
            FileContent::String(s) => s.len(),
            FileContent::Streaming(_) => 0, // Unknown for streaming
        }
    }

    /// Check if content is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Reading strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadStrategy {
    MemoryMapped,
    Buffered,
    Streaming,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_optimized_mmap_handler() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let config = MmapConfig::default();
        let handler = OptimizedMmapHandler::new(config);

        let content = handler.read_file(&test_file).unwrap();
        assert!(!content.is_memory_mapped()); // Small file should use buffered I/O
        assert_eq!(content.len(), 12);
    }

    #[test]
    fn test_memory_pool_integration() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let config = MmapConfig {
            min_file_size: 1, // Force memory mapping
            enable_pool: true,
            ..Default::default()
        };
        let handler = OptimizedMmapHandler::new(config);

        let content = handler.read_file(&test_file).unwrap();
        assert!(content.is_memory_mapped());

        let stats = handler.get_pool_stats().unwrap();
        assert_eq!(stats.pool_size, 1);
    }
}
