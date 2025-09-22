//! Parallel file processing utilities for rfgrep
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Configuration for parallel processing
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    pub max_threads: usize,
    pub chunk_size: usize,
    pub timeout_seconds: Option<u64>,
    pub adaptive_chunking: bool,
    pub memory_pressure_threshold: usize,
}

/// Memory pressure levels for adaptive chunking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPressure {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            max_threads: num_cpus::get(),
            chunk_size: 100,
            timeout_seconds: None,
            adaptive_chunking: true,
            memory_pressure_threshold: 1024 * 1024 * 1024, // 1GB
        }
    }
}

/// Parallel file processor
pub struct ParallelProcessor {
    config: ParallelConfig,
    memory_usage: Arc<AtomicUsize>,
}

impl ParallelProcessor {
    pub fn new(config: ParallelConfig) -> Self {
        Self {
            config,
            memory_usage: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Calculate adaptive chunk size based on system resources
    fn calculate_chunk_size(&self, total_files: usize) -> usize {
        if !self.config.adaptive_chunking {
            return self.config.chunk_size;
        }

        let cpu_cores = self.config.max_threads;
        let memory_pressure = self.get_memory_pressure();

        let base_chunk_size = std::cmp::max(total_files / (cpu_cores * 4), 50);

        let adjusted_chunk_size = match memory_pressure {
            MemoryPressure::Low => base_chunk_size,
            MemoryPressure::Medium => (base_chunk_size as f64 * 0.8) as usize,
            MemoryPressure::High => (base_chunk_size as f64 * 0.6) as usize,
            MemoryPressure::Critical => (base_chunk_size as f64 * 0.4) as usize,
        };

        adjusted_chunk_size.clamp(10, 1000)
    }

    /// Get current memory pressure level
    fn get_memory_pressure(&self) -> MemoryPressure {
        let current_usage = self.memory_usage.load(Ordering::Relaxed);
        let threshold = self.config.memory_pressure_threshold;

        if current_usage < threshold / 4 {
            MemoryPressure::Low
        } else if current_usage < threshold / 2 {
            MemoryPressure::Medium
        } else if current_usage < threshold * 3 / 4 {
            MemoryPressure::High
        } else {
            MemoryPressure::Critical
        }
    }

    /// Track memory usage
    pub fn track_memory_usage(&self, amount: usize) {
        self.memory_usage.fetch_add(amount, Ordering::Relaxed);
    }

    /// Release memory usage
    pub fn release_memory_usage(&self, amount: usize) {
        self.memory_usage.fetch_sub(amount, Ordering::Relaxed);
    }

    /// Process files in parallel
    pub fn process_files<F, R>(&self, files: Vec<PathBuf>, processor: F) -> Vec<R>
    where
        F: Fn(PathBuf) -> R + Send + Sync,
        R: Send,
    {
        let chunk_size = self.calculate_chunk_size(files.len());
        files
            .into_par_iter()
            .with_max_len(chunk_size)
            .map(processor)
            .collect()
    }

    /// Process files with error handling
    pub fn process_files_with_result<F, R, E>(
        &self,
        files: Vec<PathBuf>,
        processor: F,
    ) -> Vec<Result<R, E>>
    where
        F: Fn(PathBuf) -> Result<R, E> + Send + Sync,
        R: Send,
        E: Send,
    {
        let chunk_size = self.calculate_chunk_size(files.len());
        files
            .into_par_iter()
            .with_max_len(chunk_size)
            .map(processor)
            .collect()
    }

    /// Get processing statistics
    pub fn get_stats(&self) -> ProcessingStats {
        ProcessingStats {
            max_threads: self.config.max_threads,
            chunk_size: self.config.chunk_size,
            adaptive_chunking: self.config.adaptive_chunking,
            memory_usage: self.memory_usage.load(Ordering::Relaxed),
            memory_pressure: self.get_memory_pressure(),
        }
    }
}

/// Processing statistics
#[derive(Debug, Clone)]
pub struct ProcessingStats {
    pub max_threads: usize,
    pub chunk_size: usize,
    pub adaptive_chunking: bool,
    pub memory_usage: usize,
    pub memory_pressure: MemoryPressure,
}

/// Parallel search processor
pub struct ParallelSearchProcessor {
    processor: ParallelProcessor,
}

impl ParallelSearchProcessor {
    pub fn new(config: ParallelConfig) -> Self {
        Self {
            processor: ParallelProcessor::new(config),
        }
    }

    /// Process search operations in parallel
    pub fn search_files<F, R>(&self, files: Vec<PathBuf>, searcher: F) -> Vec<R>
    where
        F: Fn(PathBuf) -> R + Send + Sync,
        R: Send,
    {
        self.processor.process_files(files, searcher)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_parallel_processing() {
        let temp_dir = tempdir().unwrap();
        let files: Vec<PathBuf> = (0..10)
            .map(|i| temp_dir.path().join(format!("file_{}.txt", i)))
            .collect();

        for file in &files {
            fs::write(file, "test content").unwrap();
        }

        let config = ParallelConfig::default();
        let processor = ParallelProcessor::new(config);

        let results: Vec<String> =
            processor.process_files(files, |path| fs::read_to_string(path).unwrap());

        assert_eq!(results.len(), 10);
        assert!(results.iter().all(|content| content == "test content"));
    }

    #[test]
    fn test_adaptive_chunking() {
        let config = ParallelConfig {
            adaptive_chunking: true,
            max_threads: 4,
            ..Default::default()
        };
        let processor = ParallelProcessor::new(config);

        let small_files = (0..50)
            .map(|i| PathBuf::from(format!("file_{}.txt", i)))
            .collect();
        let large_files = (0..1000)
            .map(|i| PathBuf::from(format!("file_{}.txt", i)))
            .collect();

        let _results_small: Vec<()> = processor.process_files(small_files, |_| ());

        let _results_large: Vec<()> = processor.process_files(large_files, |_| ());

        let stats = processor.get_stats();
        assert!(stats.adaptive_chunking);
    }

    #[test]
    fn test_parallel_search() {
        let temp_dir = tempdir().unwrap();
        let files: Vec<PathBuf> = (0..5)
            .map(|i| temp_dir.path().join(format!("file_{}.txt", i)))
            .collect();

        for (i, file) in files.iter().enumerate() {
            fs::write(file, format!("content {}", i)).unwrap();
        }

        let config = ParallelConfig::default();
        let search_processor = ParallelSearchProcessor::new(config);

        let results: Vec<bool> = search_processor.search_files(files, |path| {
            let content = fs::read_to_string(path).unwrap();
            content.contains("content")
        });

        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|&found| found));
    }
}
