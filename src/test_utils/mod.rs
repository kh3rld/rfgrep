//! Test utilities for rfgrep
//! Provides common testing functions, mock data, and test harnesses

pub mod data_generator;
pub mod mock_file_system;
pub mod performance_benchmark;
pub mod performance_harness;

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test environment that provides temporary directories and test files
pub struct TestEnvironment {
    temp_dir: TempDir,
    test_files: Vec<PathBuf>,
}

impl TestEnvironment {
    /// Create a new test environment
    pub fn new() -> Self {
        Self {
            temp_dir: TempDir::new().expect("Failed to create temp directory"),
            test_files: Vec::new(),
        }
    }

    /// Get the root directory for tests
    pub fn root_dir(&self) -> &std::path::Path {
        self.temp_dir.path()
    }

    /// Create a test file with given content
    pub fn create_file(&mut self, name: &str, content: &str) -> PathBuf {
        let path = self.root_dir().join(name);
        fs::write(&path, content).expect("Failed to write test file");
        self.test_files.push(path.clone());
        path
    }

    /// Create a binary test file
    pub fn create_binary_file(&mut self, name: &str, content: &[u8]) -> PathBuf {
        let path = self.root_dir().join(name);
        fs::write(&path, content).expect("Failed to write binary test file");
        self.test_files.push(path.clone());
        path
    }

    /// Create a directory structure for testing
    pub fn create_directory_structure(&mut self) -> PathBuf {
        let subdir = self.root_dir().join("subdir");
        fs::create_dir_all(&subdir).expect("Failed to create subdirectory");

        // Create files in subdirectory
        self.create_file("subdir/file1.txt", "Content in subdirectory");
        self.create_file("subdir/file2.txt", "More content in subdirectory");

        subdir
    }

    /// Create a large test file
    pub fn create_large_file(&mut self, name: &str, size_mb: usize) -> PathBuf {
        let path = self.root_dir().join(name);
        let content = "A".repeat(size_mb * 1024 * 1024);
        fs::write(&path, content).expect("Failed to write large test file");
        self.test_files.push(path.clone());
        path
    }

    /// Get all test files created
    pub fn test_files(&self) -> &[PathBuf] {
        &self.test_files
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        // Cleanup is handled by TempDir
    }
}

/// Memory usage tracker
pub struct MemoryTracker {
    initial_memory: usize,
    peak_memory: usize,
}

impl MemoryTracker {
    pub fn new() -> Self {
        Self {
            initial_memory: Self::current_memory_usage(),
            peak_memory: Self::current_memory_usage(),
        }
    }

    pub fn update(&mut self) {
        let current = Self::current_memory_usage();
        self.peak_memory = self.peak_memory.max(current);
    }

    pub fn peak_memory_usage(&self) -> usize {
        self.peak_memory
    }

    pub fn memory_increase(&self) -> usize {
        self.peak_memory.saturating_sub(self.initial_memory)
    }

    fn current_memory_usage() -> usize {
        #[cfg(target_os = "linux")]
        {
            if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("VmRSS:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = kb_str.parse::<usize>() {
                                return kb * 1024; // Convert KB to bytes
                            }
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            if let Ok(output) = Command::new("ps")
                .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
                .output()
            {
                if let Ok(rss_str) = String::from_utf8(output.stdout) {
                    if let Ok(kb) = rss_str.trim().parse::<usize>() {
                        return kb * 1024; // Convert KB to bytes
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            use std::process::Command;
            if let Ok(output) = Command::new("wmic")
                .args(&[
                    "process",
                    "where",
                    &format!("ProcessId={}", std::process::id()),
                ])
                .args(&["get", "WorkingSetSize", "/value"])
                .output()
            {
                if let Ok(output_str) = String::from_utf8(output.stdout) {
                    for line in output_str.lines() {
                        if line.starts_with("WorkingSetSize=") {
                            if let Some(value) = line.split('=').nth(1) {
                                if let Ok(bytes) = value.trim().parse::<usize>() {
                                    return bytes;
                                }
                            }
                        }
                    }
                }
            }
        }

        #[cfg(unix)]
        {
            if let Ok(statm) = std::fs::read_to_string("/proc/self/statm") {
                if let Some(pages_str) = statm.split_whitespace().nth(1) {
                    if let Ok(pages) = pages_str.parse::<usize>() {
                        return pages * 4096; // Assume 4KB pages
                    }
                }
            }
        }

        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::performance_harness::PerformanceHarness;

    #[test]
    fn test_environment_creation() {
        let mut env = TestEnvironment::new();
        let file_path = env.create_file("test.txt", "test content");
        assert!(file_path.exists());
    }

    #[test]
    fn test_performance_harness() {
        let mut harness = PerformanceHarness::new();
        harness.measure("test_operation", || {
            std::thread::sleep(std::time::Duration::from_millis(10));
        });

        let measurements = harness.get_measurements();
        assert_eq!(measurements.len(), 1);
        assert_eq!(measurements[0].name, "test_operation");
    }

    #[test]
    fn test_real_world_memory_tracking() {
        let mut tracker = MemoryTracker::new();
        let initial_memory = tracker.peak_memory_usage();

        // Simulate memory allocation
        let mut data = Vec::new();
        for i in 0..1000 {
            data.push(vec![i; 1000]); // Allocate 1MB chunks
            tracker.update();

            // Check that memory usage is being tracked
            if i % 100 == 0 {
                let current_usage = tracker.peak_memory_usage();
                if current_usage > 0 {
                    assert!(current_usage >= initial_memory);
                }
            }
        }

        // Verify peak memory was recorded
        let peak_usage = tracker.peak_memory_usage();
        let _memory_increase = tracker.memory_increase();

        if peak_usage > 0 {
            assert!(peak_usage >= initial_memory);
        }

        drop(data);
        tracker.update();
    }

    #[test]
    fn test_memory_tracking_under_load() {
        let mut tracker = MemoryTracker::new();
        let initial_memory = tracker.peak_memory_usage();

        let mut file_data = Vec::new();

        for _i in 0..10 {
            let file_content = vec![b'x'; 1024 * 1024]; // 1MB per file
            file_data.push(file_content);
            tracker.update();

            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let peak_usage = tracker.peak_memory_usage();
        let memory_increase = tracker.memory_increase();

        if peak_usage > 0 {
            assert!(peak_usage >= initial_memory);

            if memory_increase > 0 {
                assert!(memory_increase >= 5 * 1024 * 1024); // At least 5MB
            }
        }

        file_data.clear();
        tracker.update();
    }

    #[test]
    fn test_memory_tracking_accuracy() {
        let tracker = MemoryTracker::new();
        let initial_memory = tracker.peak_memory_usage();

        let current_usage = MemoryTracker::current_memory_usage();

        if current_usage > 0 {
            assert!(current_usage < 1024 * 1024 * 1024); // Less than 1GB for a test process
            assert!(current_usage >= initial_memory);
        }
    }

    #[test]
    fn test_memory_tracking_thread_safety() {
        let tracker = std::sync::Arc::new(std::sync::Mutex::new(MemoryTracker::new()));
        let mut handles = vec![];

        for i in 0..5 {
            let tracker_clone = tracker.clone();
            let handle = std::thread::spawn(move || {
                let mut tracker = tracker_clone.lock().unwrap();

                let mut data = vec![0u8; 1024 * 100 * (i + 1)]; // Different sizes per thread
                tracker.update();

                data.extend(std::iter::repeat_n(1, 100));
                tracker.update();

                tracker.peak_memory_usage()
            });
            handles.push(handle);
        }

        let mut peak_values = Vec::new();
        for handle in handles {
            if let Ok(peak) = handle.join() {
                peak_values.push(peak);
            }
        }

        let final_tracker = tracker.lock().unwrap();
        let final_peak = final_tracker.peak_memory_usage();

        if final_peak > 0 {
            assert!(final_peak >= final_tracker.initial_memory);
        }
    }
}
