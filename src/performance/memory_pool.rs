//! Memory pool for efficient memory-mapped file handling
use log::debug;
use memmap2::Mmap;
use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

/// Memory pool for managing memory-mapped files
pub struct MemoryPool {
    /// Pool of available memory maps
    pool: Arc<RwLock<HashMap<PathBuf, Arc<Mmap>>>>,
    /// Metadata for pooled memory maps
    metadata: Arc<RwLock<HashMap<PathBuf, MmapMetadata>>>,
    /// Maximum number of memory maps to keep in pool
    max_pool_size: usize,
    /// Maximum age for cached memory maps
    max_age: Duration,
    /// Current memory usage in bytes
    memory_usage: Arc<Mutex<usize>>,
    /// Memory pressure threshold
    memory_threshold: usize,
}

/// Metadata for pooled memory maps
#[derive(Debug, Clone)]
struct MmapMetadata {
    last_accessed: Instant,
    file_size: u64,
    access_count: u64,
}

impl MemoryPool {
    /// Create a new memory pool
    pub fn new(max_pool_size: usize, max_age_seconds: u64, memory_threshold: usize) -> Self {
        Self {
            pool: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
            max_pool_size,
            max_age: Duration::from_secs(max_age_seconds),
            memory_usage: Arc::new(Mutex::new(0)),
            memory_threshold,
        }
    }

    /// Get a memory map for the given file, creating it if necessary
    pub fn get_mmap(&self, path: &PathBuf) -> Result<Arc<Mmap>, MemoryPoolError> {
        // Check if we already have this file in the pool
        {
            let pool = self.pool.read().unwrap();
            if let Some(mmap) = pool.get(path) {
                // Update metadata
                if let Some(meta) = self.metadata.write().unwrap().get_mut(path) {
                    meta.last_accessed = Instant::now();
                    meta.access_count += 1;
                }

                debug!("Memory pool hit for file: {:?}", path);
                return Ok(mmap.clone());
            }
        }

        // Create new memory map
        self.create_mmap(path)
    }

    /// Create a new memory map and add it to the pool
    fn create_mmap(&self, path: &PathBuf) -> Result<Arc<Mmap>, MemoryPoolError> {
        let file = File::open(path).map_err(|e| {
            MemoryPoolError::IoError(format!("Failed to open file {:?}: {}", path, e))
        })?;

        let metadata = file.metadata().map_err(|e| {
            MemoryPoolError::IoError(format!("Failed to get metadata for {:?}: {}", path, e))
        })?;

        let file_size = metadata.len();

        // Check memory pressure
        if self.is_memory_pressure_high() {
            return Err(MemoryPoolError::MemoryPressure(
                "Memory pressure too high".to_string(),
            ));
        }

        // Create memory map
        let mmap = unsafe { Mmap::map(&file) }.map_err(|e| {
            MemoryPoolError::MmapError(format!("Failed to create memory map for {:?}: {}", path, e))
        })?;

        // Update memory usage
        {
            let mut usage = self.memory_usage.lock().unwrap();
            *usage += file_size as usize;
        }

        // Add to pool
        let mmap_arc = Arc::new(mmap);
        let metadata = MmapMetadata {
            last_accessed: Instant::now(),
            file_size,
            access_count: 1,
        };

        {
            let mut pool = self.pool.write().unwrap();
            let mut meta_map = self.metadata.write().unwrap();

            // Clean up old entries if pool is full
            if pool.len() >= self.max_pool_size {
                self.cleanup_old_entries(&mut pool, &mut meta_map);
            }

            pool.insert(path.clone(), mmap_arc.clone());
            meta_map.insert(path.clone(), metadata);
        }

        debug!(
            "Created new memory map for file: {:?} ({} bytes)",
            path, file_size
        );
        Ok(mmap_arc)
    }

    /// Check if memory pressure is high
    fn is_memory_pressure_high(&self) -> bool {
        let usage = self.memory_usage.lock().unwrap();
        *usage > self.memory_threshold
    }

    /// Clean up old entries from the pool
    fn cleanup_old_entries(
        &self,
        pool: &mut HashMap<PathBuf, Arc<Mmap>>,
        meta_map: &mut HashMap<PathBuf, MmapMetadata>,
    ) {
        let now = Instant::now();
        let mut to_remove = Vec::new();

        for (path, metadata) in meta_map.iter() {
            if now.duration_since(metadata.last_accessed) > self.max_age {
                to_remove.push(path.clone());
            }
        }

        for path in to_remove {
            if let Some(metadata) = meta_map.remove(&path) {
                pool.remove(&path);
                let mut usage = self.memory_usage.lock().unwrap();
                *usage = usage.saturating_sub(metadata.file_size as usize);
                debug!("Removed old memory map from pool: {:?}", path);
            }
        }
    }

    /// Remove a specific file from the pool
    pub fn remove_file(&self, path: &PathBuf) {
        let mut pool = self.pool.write().unwrap();
        let mut meta_map = self.metadata.write().unwrap();

        if let Some(metadata) = meta_map.remove(path) {
            pool.remove(path);
            let mut usage = self.memory_usage.lock().unwrap();
            *usage = usage.saturating_sub(metadata.file_size as usize);
            debug!("Removed file from memory pool: {:?}", path);
        }
    }

    /// Clear all entries from the pool
    pub fn clear(&self) {
        let mut pool = self.pool.write().unwrap();
        let mut meta_map = self.metadata.write().unwrap();

        pool.clear();
        meta_map.clear();

        let mut usage = self.memory_usage.lock().unwrap();
        *usage = 0;
        debug!("Cleared memory pool");
    }

    /// Get current memory usage
    pub fn memory_usage(&self) -> usize {
        *self.memory_usage.lock().unwrap()
    }

    /// Get pool statistics
    pub fn get_stats(&self) -> MemoryPoolStats {
        let pool = self.pool.read().unwrap();
        let usage = self.memory_usage.lock().unwrap();

        MemoryPoolStats {
            pool_size: pool.len(),
            memory_usage: *usage,
            max_pool_size: self.max_pool_size,
            memory_threshold: self.memory_threshold,
        }
    }

    /// Clean up expired entries
    pub fn cleanup(&self) {
        let mut pool = self.pool.write().unwrap();
        let mut meta_map = self.metadata.write().unwrap();
        self.cleanup_old_entries(&mut pool, &mut meta_map);
    }
}

/// Memory pool statistics
#[derive(Debug, Clone)]
pub struct MemoryPoolStats {
    pub pool_size: usize,
    pub memory_usage: usize,
    pub max_pool_size: usize,
    pub memory_threshold: usize,
}

/// Memory pool errors
#[derive(Debug, thiserror::Error)]
pub enum MemoryPoolError {
    #[error("IO error: {0}")]
    IoError(String),

    #[error("Memory map error: {0}")]
    MmapError(String),

    #[error("Memory pressure: {0}")]
    MemoryPressure(String),
}

// Note: PooledMmap cannot implement Clone because Mmap doesn't implement Clone
// This is handled by the memory pool logic instead

impl Default for MemoryPool {
    fn default() -> Self {
        Self::new(100, 300, 1024 * 1024 * 1024) // 100 files, 5 minutes, 1GB threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_memory_pool_creation() {
        let pool = MemoryPool::new(10, 60, 1024);
        let stats = pool.get_stats();
        assert_eq!(stats.pool_size, 0);
        assert_eq!(stats.memory_usage, 0);
    }

    #[test]
    fn test_memory_pool_file_handling() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let pool = MemoryPool::new(10, 60, 1024);

        let mmap_result = pool.get_mmap(&test_file);
        assert!(mmap_result.is_ok());

        let stats = pool.get_stats();
        assert_eq!(stats.pool_size, 1);
        assert!(stats.memory_usage > 0);
    }

    #[test]
    fn test_memory_pool_cleanup() {
        let pool = MemoryPool::new(1, 1, 1024);
        pool.cleanup();

        let stats = pool.get_stats();
        assert_eq!(stats.pool_size, 0);
    }
}
