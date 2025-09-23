//! Performance optimization modules for rfgrep v0.3.1
//! Provides memory optimization, I/O optimization, and parallel processing improvements

pub mod cache_manager;
pub mod io_optimizer;
pub mod memory_pool;
pub mod optimized_mmap;
pub mod parallel_processor;
pub mod zero_copy;

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Performance metrics for monitoring and optimization
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub search_time: std::time::Duration,
    pub memory_usage: usize,
    pub files_processed: usize,
    pub matches_found: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

/// Thread-safe performance metrics using atomic operations
#[derive(Debug)]
pub struct AtomicPerformanceMetrics {
    search_time_nanos: AtomicU64,
    memory_usage: AtomicUsize,
    files_processed: AtomicUsize,
    matches_found: AtomicUsize,
    cache_hits: AtomicUsize,
    cache_misses: AtomicUsize,
}

impl AtomicPerformanceMetrics {
    pub fn new() -> Self {
        Self {
            search_time_nanos: AtomicU64::new(0),
            memory_usage: AtomicUsize::new(0),
            files_processed: AtomicUsize::new(0),
            matches_found: AtomicUsize::new(0),
            cache_hits: AtomicUsize::new(0),
            cache_misses: AtomicUsize::new(0),
        }
    }

    pub fn set_search_time(&self, duration: std::time::Duration) {
        self.search_time_nanos
            .store(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn add_memory_usage(&self, amount: usize) {
        self.memory_usage.fetch_add(amount, Ordering::Relaxed);
    }

    pub fn increment_files_processed(&self) {
        self.files_processed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_matches_found(&self, count: usize) {
        self.matches_found.fetch_add(count, Ordering::Relaxed);
    }

    pub fn increment_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn to_performance_metrics(&self) -> PerformanceMetrics {
        PerformanceMetrics {
            search_time: std::time::Duration::from_nanos(
                self.search_time_nanos.load(Ordering::Relaxed),
            ),
            memory_usage: self.memory_usage.load(Ordering::Relaxed),
            files_processed: self.files_processed.load(Ordering::Relaxed),
            matches_found: self.matches_found.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
        }
    }
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            search_time: std::time::Duration::ZERO,
            memory_usage: 0,
            files_processed: 0,
            matches_found: 0,
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    pub fn files_per_second(&self) -> f64 {
        if self.search_time.as_secs_f64() > 0.0 {
            self.files_processed as f64 / self.search_time.as_secs_f64()
        } else {
            0.0
        }
    }

    pub fn memory_per_file(&self) -> f64 {
        if self.files_processed > 0 {
            self.memory_usage as f64 / self.files_processed as f64
        } else {
            0.0
        }
    }

    pub fn cache_hit_rate(&self) -> f64 {
        let total_requests = self.cache_hits + self.cache_misses;
        if total_requests > 0 {
            self.cache_hits as f64 / total_requests as f64
        } else {
            0.0
        }
    }
}

/// Performance monitor for tracking and optimizing rfgrep performance
pub struct PerformanceMonitor {
    metrics: Arc<AtomicPerformanceMetrics>,
    start_time: Option<Instant>,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(AtomicPerformanceMetrics::new()),
            start_time: None,
        }
    }

    pub fn start_timing(&mut self) {
        self.start_time = Some(Instant::now());
    }

    pub fn stop_timing(&mut self) {
        if let Some(start) = self.start_time {
            let duration = start.elapsed();
            self.metrics.set_search_time(duration);
            self.start_time = None;
        }
    }

    pub fn record_file_processed(&self) {
        self.metrics.increment_files_processed();
    }

    pub fn record_matches_found(&self, count: usize) {
        self.metrics.add_matches_found(count);
    }

    pub fn record_cache_hit(&self) {
        self.metrics.increment_cache_hit();
    }

    pub fn record_cache_miss(&self) {
        self.metrics.increment_cache_miss();
    }

    pub fn record_memory_usage(&self, amount: usize) {
        self.metrics.add_memory_usage(amount);
    }

    pub fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.to_performance_metrics()
    }

    pub fn get_atomic_metrics(&self) -> Arc<AtomicPerformanceMetrics> {
        self.metrics.clone()
    }

    /// Reset all metrics to zero
    pub fn reset(&self) {
        self.metrics.search_time_nanos.store(0, Ordering::Relaxed);
        self.metrics.memory_usage.store(0, Ordering::Relaxed);
        self.metrics.files_processed.store(0, Ordering::Relaxed);
        self.metrics.matches_found.store(0, Ordering::Relaxed);
        self.metrics.cache_hits.store(0, Ordering::Relaxed);
        self.metrics.cache_misses.store(0, Ordering::Relaxed);
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory usage tracker for optimization
pub struct MemoryTracker {
    peak_usage: AtomicUsize,
    current_usage: AtomicUsize,
    allocations: AtomicUsize,
    deallocations: AtomicUsize,
}

impl MemoryTracker {
    pub fn new() -> Self {
        Self {
            peak_usage: AtomicUsize::new(0),
            current_usage: AtomicUsize::new(0),
            allocations: AtomicUsize::new(0),
            deallocations: AtomicUsize::new(0),
        }
    }

    pub fn track_allocation(&self, size: usize) {
        let current = self.current_usage.fetch_add(size, Ordering::Relaxed);
        let new_current = current + size;

        let mut peak = self.peak_usage.load(Ordering::Relaxed);
        while new_current > peak {
            match self.peak_usage.compare_exchange_weak(
                peak,
                new_current,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(current_peak) => peak = current_peak,
            }
        }

        self.allocations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn track_deallocation(&self, size: usize) {
        self.current_usage.fetch_sub(size, Ordering::Relaxed);
        self.deallocations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn peak_usage(&self) -> usize {
        self.peak_usage.load(Ordering::Relaxed)
    }

    pub fn current_usage(&self) -> usize {
        self.current_usage.load(Ordering::Relaxed)
    }

    pub fn allocation_count(&self) -> usize {
        self.allocations.load(Ordering::Relaxed)
    }

    pub fn deallocation_count(&self) -> usize {
        self.deallocations.load(Ordering::Relaxed)
    }

    /// Reset all counters to zero
    pub fn reset(&self) {
        self.peak_usage.store(0, Ordering::Relaxed);
        self.current_usage.store(0, Ordering::Relaxed);
        self.allocations.store(0, Ordering::Relaxed);
        self.deallocations.store(0, Ordering::Relaxed);
    }

    /// Get memory usage statistics
    pub fn get_stats(&self) -> MemoryStats {
        MemoryStats {
            peak_usage: self.peak_usage(),
            current_usage: self.current_usage(),
            allocations: self.allocation_count(),
            deallocations: self.deallocation_count(),
        }
    }
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub peak_usage: usize,
    pub current_usage: usize,
    pub allocations: usize,
    pub deallocations: usize,
}

impl Default for MemoryTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_performance_metrics() {
        let metrics = PerformanceMetrics::new();
        assert_eq!(metrics.files_per_second(), 0.0);
        assert_eq!(metrics.memory_per_file(), 0.0);
        assert_eq!(metrics.cache_hit_rate(), 0.0);
    }

    #[test]
    fn test_atomic_performance_metrics() {
        let metrics = AtomicPerformanceMetrics::new();

        metrics.set_search_time(Duration::from_millis(1000));
        metrics.add_memory_usage(1024);
        metrics.increment_files_processed();
        metrics.add_matches_found(5);
        metrics.increment_cache_hit();
        metrics.increment_cache_miss();

        let perf_metrics = metrics.to_performance_metrics();
        assert_eq!(perf_metrics.search_time, Duration::from_millis(1000));
        assert_eq!(perf_metrics.memory_usage, 1024);
        assert_eq!(perf_metrics.files_processed, 1);
        assert_eq!(perf_metrics.matches_found, 5);
        assert_eq!(perf_metrics.cache_hits, 1);
        assert_eq!(perf_metrics.cache_misses, 1);
    }

    #[test]
    fn test_memory_tracker() {
        let tracker = MemoryTracker::new();
        assert_eq!(tracker.peak_usage(), 0);
        assert_eq!(tracker.current_usage(), 0);

        tracker.track_allocation(1000);
        assert_eq!(tracker.current_usage(), 1000);
        assert_eq!(tracker.peak_usage(), 1000);

        tracker.track_deallocation(500);
        assert_eq!(tracker.current_usage(), 500);
        assert_eq!(tracker.peak_usage(), 1000);
    }

    #[test]
    fn test_performance_monitor() {
        let mut monitor = PerformanceMonitor::new();

        monitor.start_timing();
        thread::sleep(Duration::from_millis(10));
        monitor.stop_timing();

        monitor.record_file_processed();
        monitor.record_matches_found(3);
        monitor.record_cache_hit();
        monitor.record_memory_usage(512);

        let metrics = monitor.get_metrics();
        assert!(metrics.search_time >= Duration::from_millis(10));
        assert_eq!(metrics.files_processed, 1);
        assert_eq!(metrics.matches_found, 3);
        assert_eq!(metrics.cache_hits, 1);
        assert_eq!(metrics.memory_usage, 512);
    }

    #[test]
    fn test_concurrent_performance_monitoring() {
        let monitor = Arc::new(std::sync::Mutex::new(PerformanceMonitor::new()));
        let mut handles = vec![];

        for i in 0..10 {
            let monitor_clone = monitor.clone();
            let handle = thread::spawn(move || {
                let monitor = monitor_clone.lock().unwrap();
                monitor.record_file_processed();
                monitor.record_matches_found(i);
                if i % 2 == 0 {
                    monitor.record_cache_hit();
                } else {
                    monitor.record_cache_miss();
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let metrics = monitor.lock().unwrap().get_metrics();
        assert_eq!(metrics.files_processed, 10);
        assert_eq!(metrics.cache_hits + metrics.cache_misses, 10);
    }
}
