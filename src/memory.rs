use crate::config::PerformanceConfig;
use log::debug;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[allow(dead_code)]
pub struct AdaptiveMemoryManager {
    config: PerformanceConfig,
    current_memory_usage: Arc<AtomicU64>,
}

impl AdaptiveMemoryManager {
    #[allow(dead_code)]
    pub fn new(config: PerformanceConfig) -> Self {
        Self {
            config,
            current_memory_usage: Arc::new(AtomicU64::new(0)),
        }
    }

    #[allow(dead_code)]
    pub fn get_mmap_threshold(&self) -> u64 {
        if !self.config.adaptive_memory {
            return self.config.mmap_threshold_mb * 1024 * 1024;
        }

        let available_memory = self.get_available_memory();
        let max_memory = self.config.max_memory_usage_mb * 1024 * 1024;

        let adaptive_threshold = (available_memory / 4).min(max_memory);
        let base_threshold = self.config.mmap_threshold_mb * 1024 * 1024;

        let threshold = adaptive_threshold
            .max(base_threshold)
            .min(1024 * 1024 * 1024);

        debug!("Adaptive mmap threshold: {}MB", threshold / 1024 / 1024);
        threshold
    }

    #[allow(dead_code)]
    pub fn get_chunk_size(&self, total_items: usize) -> usize {
        let base_chunk_size = 100;
        let multiplier = self.config.chunk_size_multiplier;
        let cpu_cores = num_cpus::get();

        let memory_factor = 1.0; 
        let cpu_factor = cpu_cores as f64;

        let adjusted_chunk_size =
            (base_chunk_size as f64 * multiplier * memory_factor / cpu_factor.sqrt()) as usize;

        debug!("Adaptive chunk size: {adjusted_chunk_size}");
        adjusted_chunk_size.max(1).min(total_items)
    }

    #[allow(dead_code)]
    pub fn should_use_mmap(&self, file_size: u64) -> bool {
        let threshold = self.get_mmap_threshold();
        file_size >= threshold
    }

    #[allow(dead_code)]
    pub fn update_memory_usage(&self, bytes: u64) {
        let current = self.current_memory_usage.load(Ordering::Relaxed);
        self.current_memory_usage
            .store(current + bytes, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    fn get_available_memory(&self) -> u64 {
        8 * 1024 * 1024 * 1024
    }

    #[allow(dead_code)]
    pub fn get_current_memory_usage(&self) -> u64 {
        self.current_memory_usage.load(Ordering::Relaxed)
    }

    #[allow(dead_code)]
    pub fn reset_memory_usage(&self) {
        self.current_memory_usage.store(0, Ordering::Relaxed);
    }
}

/// Memory usage statistics
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MemoryStats {
    pub current_usage: u64,
    pub peak_usage: u64,
    pub available_memory: u64,
    pub mmap_threshold: u64,
    pub chunk_size: usize,
}

impl AdaptiveMemoryManager {
    #[allow(dead_code)]
    pub fn get_stats(&self) -> MemoryStats {
        MemoryStats {
            current_usage: self.get_current_memory_usage(),
            peak_usage: self.get_current_memory_usage(), // TODO: track peak
            available_memory: self.get_available_memory(),
            mmap_threshold: self.get_mmap_threshold(),
            chunk_size: self.get_chunk_size(100),
        }
    }
}
