//! Performance benchmark suite demonstrating rfgrep optimization strategies
use crate::error::Result as RfgrepResult;
use crate::performance::{
    memory_pool::MemoryPool,
    optimized_mmap::{MmapConfig, OptimizedMmapHandler},
    parallel_processor::{ParallelConfig, ParallelProcessor},
    zero_copy::{ZeroCopyProcessor, ZeroCopyUtils},
};
use crate::test_utils::{performance_harness::PerformanceHarness, MemoryTracker, TestEnvironment};
use regex::Regex;
use std::path::PathBuf;
use std::time::Instant;

/// Performance benchmark suite for comprehensive testing
pub struct PerformanceBenchmark {
    test_env: TestEnvironment,
    memory_tracker: MemoryTracker,
    performance_harness: PerformanceHarness,
    parallel_processor: ParallelProcessor,
    mmap_handler: OptimizedMmapHandler,
    memory_pool: MemoryPool,
}

impl PerformanceBenchmark {
    /// Create a new performance benchmark suite
    pub fn new() -> RfgrepResult<Self> {
        let test_env = TestEnvironment::new();
        let memory_tracker = MemoryTracker::new();
        let performance_harness = PerformanceHarness::new();

        let parallel_config = ParallelConfig {
            adaptive_chunking: true,
            max_threads: num_cpus::get(),
            memory_pressure_threshold: 512 * 1024 * 1024, // 512MB
            ..Default::default()
        };
        let parallel_processor = ParallelProcessor::new(parallel_config);

        let mmap_config = MmapConfig {
            min_file_size: 1024 * 1024,       // 1MB
            max_file_size: 100 * 1024 * 1024, // 100MB
            enable_pool: true,
            pool_size: 50,
            pool_age_limit: 300,                 // 5 minutes
            memory_threshold: 512 * 1024 * 1024, // 512MB
        };
        let mmap_handler = OptimizedMmapHandler::new(mmap_config);

        let memory_pool = MemoryPool::new(50, 300, 512 * 1024 * 1024);

        Ok(Self {
            test_env,
            memory_tracker,
            performance_harness,
            parallel_processor,
            mmap_handler,
            memory_pool,
        })
    }

    /// Execute a comprehensive file search benchmark
    pub fn execute_search_benchmark(
        &mut self,
        num_files: usize,
        file_size_mb: usize,
        pattern: &str,
    ) -> RfgrepResult<BenchmarkResults> {
        let start_time = Instant::now();
        self.memory_tracker.update();

        let files = self.generate_test_files(num_files, file_size_mb)?;
        self.memory_tracker.update();

        let regex = Regex::new(pattern)?;
        self.memory_tracker.update();

        let results = self.test_search_strategies(&files, &regex)?;
        self.memory_tracker.update();

        let total_time = start_time.elapsed();
        let peak_memory = self.memory_tracker.peak_memory_usage();
        let memory_increase = self.memory_tracker.memory_increase();

        Ok(BenchmarkResults {
            total_time,
            peak_memory,
            memory_increase,
            files_processed: files.len(),
            search_results: results,
            performance_stats: self.performance_harness.get_stats(),
            memory_pool_stats: self.memory_pool.get_stats(),
        })
    }

    /// Generate test files with realistic content
    fn generate_test_files(
        &mut self,
        num_files: usize,
        file_size_mb: usize,
    ) -> RfgrepResult<Vec<PathBuf>> {
        let mut files = Vec::new();

        for i in 0..num_files {
            let filename = format!("file_{:04}.txt", i);
            let file_path = self.test_env.create_file(&filename, "");

            let content = self.generate_test_content(file_size_mb, i);
            std::fs::write(&file_path, content)?;

            files.push(file_path);

            if i % 10 == 0 {
                self.memory_tracker.update();
            }
        }

        Ok(files)
    }

    /// Generate representative test content
    fn generate_test_content(&self, size_mb: usize, file_index: usize) -> String {
        let mut content = String::with_capacity(size_mb * 1024 * 1024);

        content.push_str(&format!("// File {} - Generated content\n", file_index));
        content.push_str("// This is a realistic file with various content types\n");
        content.push_str("// Including code, comments, and data\n\n");

        let patterns = vec![
            "function processData() {",
            "  try {",
            "    const result = await fetch('/api/data');",
            "    if (!result.ok) {",
            "      throw new Error(`HTTP error! status: ${result.status}`);",
            "    }",
            "    return await result.json();",
            "  } catch (error) {",
            "    console.error('Failed to process data:', error);",
            "    throw new Error(`Data processing failed: ${error.message}`);",
            "  }",
            "}",
            "console.log('Processing file:', filename);",
            "const config = { timeout: 5000 };",
            "try {",
            "  const processor = new Processor(config);",
            "  await processor.process();",
            "} catch (error) {",
            "  console.error('Processing failed:', error);",
            "  process.exit(1);",
            "}",
            "export default class Processor {",
            "  constructor(options) {",
            "    this.options = options || {};",
            "    this.validateOptions();",
            "  }",
            "  validateOptions() {",
            "    if (!this.options.timeout || this.options.timeout < 0) {",
            "      throw new Error('Invalid timeout value');",
            "    }",
            "  }",
        ];

        let target_size = size_mb * 1024 * 1024;
        let mut current_size = content.len();

        while current_size < target_size {
            let pattern = &patterns[file_index % patterns.len()];
            content.push_str(pattern);
            content.push('\n');
            current_size = content.len();

            if file_index.is_multiple_of(3) {
                content.push_str(&format!("data_{} = [1, 2, 3, 4, 5];\n", file_index));
                current_size = content.len();
            }
        }

        content
    }

    /// Test different search strategies
    fn test_search_strategies(
        &mut self,
        files: &[PathBuf],
        pattern: &Regex,
    ) -> RfgrepResult<SearchResults> {
        let mut results = SearchResults::new();

        let parallel_start = Instant::now();
        let parallel_matches = self.test_parallel_search(files, pattern)?;
        results.parallel_time = parallel_start.elapsed();
        results.parallel_matches = parallel_matches;

        let mmap_start = Instant::now();
        let mmap_matches = self.test_mmap_search(files, pattern)?;
        results.mmap_time = mmap_start.elapsed();
        results.mmap_matches = mmap_matches;

        let zero_copy_start = Instant::now();
        let zero_copy_matches = self.test_zero_copy_search(files, pattern)?;
        results.zero_copy_time = zero_copy_start.elapsed();
        results.zero_copy_matches = zero_copy_matches;

        let combined_start = Instant::now();
        let combined_matches = self.test_combined_approach(files, pattern)?;
        results.combined_time = combined_start.elapsed();
        results.combined_matches = combined_matches;

        Ok(results)
    }

    /// Test parallel search with adaptive chunking
    fn test_parallel_search(&self, files: &[PathBuf], pattern: &Regex) -> RfgrepResult<usize> {
        let files_vec = files.to_vec();
        let pattern_clone = pattern.clone();

        let matches = self
            .parallel_processor
            .process_files(files_vec, |file_path| {
                match std::fs::read_to_string(&file_path) {
                    Ok(content) => pattern_clone.find_iter(&content).count(),
                    Err(_) => 0,
                }
            });

        Ok(matches.iter().sum())
    }

    /// Test memory-mapped I/O search
    fn test_mmap_search(&self, files: &[PathBuf], pattern: &Regex) -> RfgrepResult<usize> {
        let mut total_matches = 0;

        for file_path in files {
            match self.mmap_handler.read_file(file_path) {
                Ok(content) => {
                    if let Ok(text) = content.as_str() {
                        total_matches += pattern.find_iter(text).count();
                    }
                }
                Err(_) => continue,
            }
        }

        Ok(total_matches)
    }

    /// Test zero-copy search
    fn test_zero_copy_search(&self, files: &[PathBuf], pattern: &Regex) -> RfgrepResult<usize> {
        let mut total_matches = 0;

        for file_path in files {
            match std::fs::read_to_string(file_path) {
                Ok(content) => {
                    let _processor = ZeroCopyProcessor::new(&content);
                    let matches = ZeroCopyUtils::count_matches(&content, pattern);
                    total_matches += matches;
                }
                Err(_) => continue,
            }
        }

        Ok(total_matches)
    }

    /// Test combined approach
    fn test_combined_approach(
        &mut self,
        files: &[PathBuf],
        pattern: &Regex,
    ) -> RfgrepResult<usize> {
        let files_vec = files.to_vec();
        let pattern_clone = pattern.clone();
        let mmap_handler = &self.mmap_handler;

        let matches = self
            .parallel_processor
            .process_files(files_vec, |file_path| {
                match mmap_handler.read_file(&file_path) {
                    Ok(content) => {
                        if let Ok(text) = content.as_str() {
                            ZeroCopyUtils::count_matches(text, &pattern_clone)
                        } else {
                            0
                        }
                    }
                    Err(_) => 0,
                }
            });

        Ok(matches.iter().sum())
    }

    /// Generate comprehensive performance analysis report
    pub fn generate_performance_report(&self) -> String {
        let mut report = String::new();

        report.push_str("=== Performance Analysis Report ===\n\n");

        report.push_str(&format!(
            "Peak Memory Usage: {} MB\n",
            self.memory_tracker.peak_memory_usage() / 1024 / 1024
        ));
        report.push_str(&format!(
            "Memory Increase: {} MB\n",
            self.memory_tracker.memory_increase() / 1024 / 1024
        ));

        let perf_stats = self.performance_harness.get_stats();
        report.push_str(&format!(
            "Total Operations: {}\n",
            perf_stats.total_operations
        ));
        report.push_str(&format!(
            "Average Throughput: {:.2} ops/sec\n",
            perf_stats.average_throughput
        ));

        let pool_stats = self.memory_pool.get_stats();
        report.push_str(&format!("Memory Pool Size: {}\n", pool_stats.pool_size));
        report.push_str(&format!(
            "Pool Memory Usage: {} MB\n",
            pool_stats.memory_usage / 1024 / 1024
        ));

        report
    }
}

/// Results from a performance benchmark test
#[derive(Debug)]
pub struct BenchmarkResults {
    pub total_time: std::time::Duration,
    pub peak_memory: usize,
    pub memory_increase: usize,
    pub files_processed: usize,
    pub search_results: SearchResults,
    pub performance_stats: crate::test_utils::performance_harness::PerformanceStats,
    pub memory_pool_stats: crate::performance::memory_pool::MemoryPoolStats,
}

/// Search results for different strategies
#[derive(Debug)]
pub struct SearchResults {
    pub parallel_time: std::time::Duration,
    pub parallel_matches: usize,
    pub mmap_time: std::time::Duration,
    pub mmap_matches: usize,
    pub zero_copy_time: std::time::Duration,
    pub zero_copy_matches: usize,
    pub combined_time: std::time::Duration,
    pub combined_matches: usize,
}

impl SearchResults {
    fn new() -> Self {
        Self {
            parallel_time: std::time::Duration::ZERO,
            parallel_matches: 0,
            mmap_time: std::time::Duration::ZERO,
            mmap_matches: 0,
            zero_copy_time: std::time::Duration::ZERO,
            zero_copy_matches: 0,
            combined_time: std::time::Duration::ZERO,
            combined_matches: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_benchmark_creation() {
        let benchmark = PerformanceBenchmark::new().unwrap();
        assert!(benchmark.test_env.temp_dir.path().exists());
    }

    #[test]
    fn test_small_scale_benchmark() {
        let mut benchmark = PerformanceBenchmark::new().unwrap();
        let results = benchmark
            .execute_search_benchmark(5, 1, "function")
            .unwrap();

        assert_eq!(results.files_processed, 5);
        assert!(results.total_time.as_millis() > 0);
    }

    #[test]
    fn test_medium_scale_benchmark() {
        let mut benchmark = PerformanceBenchmark::new().unwrap();
        let results = benchmark.execute_search_benchmark(20, 2, "const").unwrap();

        assert_eq!(results.files_processed, 20);
        assert!(results.total_time.as_millis() > 0);

        let expected_matches = results.search_results.parallel_matches;
        assert_eq!(results.search_results.mmap_matches, expected_matches);
        assert_eq!(results.search_results.zero_copy_matches, expected_matches);
        assert_eq!(results.search_results.combined_matches, expected_matches);
    }

    #[test]
    fn test_performance_report_generation() {
        let mut benchmark = PerformanceBenchmark::new().unwrap();
        let _results = benchmark.execute_search_benchmark(3, 1, "TODO").unwrap();
        let report = benchmark.generate_performance_report();

        assert!(report.contains("Performance Analysis Report"));
        assert!(report.contains("Peak Memory Usage"));
        assert!(report.contains("Memory Increase"));
        assert!(report.contains("Total Operations"));
    }
}
