//! Intelligent caching system for search results and file metadata
use crate::processor::SearchMatch;
use lru::LruCache;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

/// Cache key for search results
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheKey {
    pub file_path: PathBuf,
    pub pattern: String,
    pub case_sensitive: bool,
    pub file_hash: u64,
}

impl Hash for CacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.file_path.hash(state);
        self.pattern.hash(state);
        self.case_sensitive.hash(state);
        self.file_hash.hash(state);
    }
}

/// Search result cache
pub struct SearchCache {
    results_cache: Mutex<LruCache<CacheKey, Vec<SearchMatch>>>,
    metadata_cache: Mutex<LruCache<PathBuf, FileMetadata>>,
    max_results_cache_size: usize,
    max_metadata_cache_size: usize,
}

/// File metadata for cache invalidation
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub size: u64,
    pub modified: u64,
    pub hash: u64,
}

impl SearchCache {
    /// Create a new search cache
    pub fn new() -> Self {
        Self {
            results_cache: Mutex::new(LruCache::new(std::num::NonZeroUsize::new(1000).unwrap())),
            metadata_cache: Mutex::new(LruCache::new(std::num::NonZeroUsize::new(10000).unwrap())),
            max_results_cache_size: 1000,
            max_metadata_cache_size: 10000,
        }
    }

    /// Get cached search results
    pub fn get(&self, file_path: &std::path::Path, pattern: &str, case_sensitive: bool) -> Option<Vec<SearchMatch>> {
        let metadata = self.get_file_metadata(file_path)?;
        let key = CacheKey {
            file_path: file_path.to_path_buf(),
            pattern: pattern.to_string(),
            case_sensitive,
            file_hash: metadata.hash,
        };

        self.results_cache.lock().unwrap().get(&key).cloned()
    }

    /// Insert search results into cache
    pub fn insert(&self, file_path: &std::path::Path, pattern: &str, case_sensitive: bool, matches: Vec<SearchMatch>) {
        let metadata = match self.get_file_metadata(file_path) {
            Some(meta) => meta,
            None => return, // Can't cache without metadata
        };

        let key = CacheKey {
            file_path: file_path.to_path_buf(),
            pattern: pattern.to_string(),
            case_sensitive,
            file_hash: metadata.hash,
        };

        self.results_cache.lock().unwrap().put(key, matches);
    }

    /// Get file metadata (cached)
    pub fn get_file_metadata(&self, file_path: &std::path::Path) -> Option<FileMetadata> {
        // Check cache first
        if let Some(metadata) = self.metadata_cache.lock().unwrap().get(file_path) {
            return Some(metadata.clone());
        }

        // Load from filesystem
        let metadata = self.load_file_metadata(file_path)?;

        // Cache it
        self.metadata_cache
            .lock()
            .unwrap()
            .put(file_path.to_path_buf(), metadata.clone());

        Some(metadata)
    }

    /// Load file metadata from filesystem
    fn load_file_metadata(&self, file_path: &std::path::Path) -> Option<FileMetadata> {
        let metadata = std::fs::metadata(file_path).ok()?;
        let modified = metadata
            .modified()
            .ok()?
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_secs();

        // Calculate file hash for cache invalidation
        let hash = self.calculate_file_hash(file_path)?;

        Some(FileMetadata {
            size: metadata.len(),
            modified,
            hash,
        })
    }

    /// Calculate a simple hash of the file for cache invalidation
    fn calculate_file_hash(&self, file_path: &std::path::Path) -> Option<u64> {
        let metadata = std::fs::metadata(file_path).ok()?;
        let mut hasher = DefaultHasher::new();

        // Use file size and modification time for a quick hash
        metadata.len().hash(&mut hasher);
        metadata
            .modified()
            .ok()?
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_secs()
            .hash(&mut hasher);

        Some(hasher.finish())
    }

    /// Check if file has been modified since last cache
    pub fn is_file_modified(&self, file_path: &std::path::Path) -> bool {
        let current_metadata = self.load_file_metadata(file_path);
        let mut cache_guard = self.metadata_cache.lock().unwrap();
        let cached_metadata = cache_guard.get(file_path);

        match (current_metadata, cached_metadata) {
            (Some(current), Some(cached)) => {
                current.size != cached.size || current.modified != cached.modified
            }
            (Some(_), None) => true, // File not in cache
            (None, _) => true,       // File doesn't exist
        }
    }

    /// Invalidate cache for a specific file
    pub fn invalidate_file(&self, file_path: &std::path::Path) {
        // Remove from metadata cache
        self.metadata_cache.lock().unwrap().pop(file_path);

        // Remove all search results for this file
        let mut results_cache = self.results_cache.lock().unwrap();
        let keys_to_remove: Vec<_> = results_cache
            .iter()
            .filter(|(key, _)| key.file_path == file_path)
            .map(|(key, _)| key.clone())
            .collect();

        for key in keys_to_remove {
            results_cache.pop(&key);
        }
    }

    /// Clear all caches
    pub fn clear(&self) {
        self.results_cache.lock().unwrap().clear();
        self.metadata_cache.lock().unwrap().clear();
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        let results_cache = self.results_cache.lock().unwrap();
        let metadata_cache = self.metadata_cache.lock().unwrap();

        CacheStats {
            results_cache_size: results_cache.len(),
            metadata_cache_size: metadata_cache.len(),
            max_results_cache_size: self.max_results_cache_size,
            max_metadata_cache_size: self.max_metadata_cache_size,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub results_cache_size: usize,
    pub metadata_cache_size: usize,
    pub max_results_cache_size: usize,
    pub max_metadata_cache_size: usize,
}

/// Cache warming utility
pub struct CacheWarmer {
    cache: std::sync::Arc<SearchCache>,
}

impl CacheWarmer {
    pub fn new(cache: std::sync::Arc<SearchCache>) -> Self {
        Self { cache }
    }

    /// Warm up cache by pre-loading metadata for common file types
    pub async fn warm_metadata(&self, paths: &[std::path::PathBuf]) {
        use rayon::prelude::*;

        paths.par_iter().for_each(|path| {
            let _ = self.cache.get_file_metadata(path);
        });
    }

    /// Warm up cache by pre-searching common patterns
    pub async fn warm_search_results(&self, paths: &[std::path::PathBuf], patterns: &[String]) {
        use rayon::prelude::*;

        for pattern in patterns {
            paths.par_iter().for_each(|path| {
                // This would trigger cache population (assume case-insensitive default)
                let _ = self.cache.get(path, pattern, false);
            });
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub enable_results_cache: bool,
    pub enable_metadata_cache: bool,
    pub max_results_cache_size: usize,
    pub max_metadata_cache_size: usize,
    pub cache_ttl_seconds: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enable_results_cache: true,
            enable_metadata_cache: true,
            max_results_cache_size: 1000,
            max_metadata_cache_size: 10000,
            cache_ttl_seconds: 3600, // 1 hour
        }
    }
}
