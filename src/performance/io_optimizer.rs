//! I/O optimization module for efficient file processing
//! Provides buffered I/O, prefetching, and adaptive strategies

use crossbeam_channel::{bounded, Receiver, Sender};
use rayon::prelude::*;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::thread;

/// I/O optimization configuration
#[derive(Debug, Clone)]
pub struct IoConfig {
    pub buffer_size: usize,
    pub prefetch_enabled: bool,
    pub prefetch_queue_size: usize,
    pub mmap_threshold: usize,
    pub parallel_io: bool,
    pub max_parallel_files: usize,
}

impl Default for IoConfig {
    fn default() -> Self {
        Self {
            buffer_size: 64 * 1024,
            prefetch_enabled: true,
            prefetch_queue_size: 10,
            mmap_threshold: 16 * 1024 * 1024,
            parallel_io: true,
            max_parallel_files: 4,
        }
    }
}

/// Optimized file reader with buffering and prefetching
pub struct OptimizedFileReader {
    config: IoConfig,
    prefetch_sender: Option<Sender<PathBuf>>,
    prefetch_receiver: Option<Receiver<PathBuf>>,
}

impl OptimizedFileReader {
    pub fn new(config: IoConfig) -> Self {
        let (prefetch_sender, prefetch_receiver) = if config.prefetch_enabled {
            let (tx, rx) = bounded(config.prefetch_queue_size);
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };

        Self {
            config,
            prefetch_sender,
            prefetch_receiver,
        }
    }

    /// Start prefetching files
    pub fn start_prefetching(&self) {
        if let (Some(_sender), Some(_receiver)) = (&self.prefetch_sender, &self.prefetch_receiver) {
            let _config = self.config.clone();
            let receiver = _receiver.clone();

            thread::spawn(move || {
                let mut prefetched_files = VecDeque::new();

                while let Ok(path) = receiver.recv() {
                    if let Ok(file) = File::open(&path) {
                        prefetched_files.push_back((path, file));
                    }
                }
            });
        }
    }

    /// Read file with optimal strategy
    pub fn read_file(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        let metadata = std::fs::metadata(path)?;
        let file_size = metadata.len() as usize;

        if file_size > self.config.mmap_threshold {
            self.read_file_mmap(path)
        } else {
            self.read_file_buffered(path)
        }
    }

    /// Read file using memory mapping for large files
    fn read_file_mmap(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        use memmap2::Mmap;

        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        Ok(mmap.to_vec())
    }

    /// Read file using buffered I/O for smaller files
    fn read_file_buffered(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        let file = File::open(path)?;
        let mut reader = BufReader::with_capacity(self.config.buffer_size, file);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        Ok(buffer)
    }

    /// Read file line by line with buffering
    pub fn read_lines(&self, path: &Path) -> std::io::Result<Vec<String>> {
        let file = File::open(path)?;
        let reader = BufReader::with_capacity(self.config.buffer_size, file);
        let mut lines = Vec::new();

        for line in reader.lines() {
            lines.push(line?);
        }

        Ok(lines)
    }

    /// Check if file should be processed
    pub fn should_process_file(&self, path: &Path) -> bool {
        if let Ok(metadata) = std::fs::metadata(path) {
            let file_size = metadata.len() as usize;

            if file_size > 100 * 1024 * 1024 {
                return false;
            }
        }

        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                match ext_str.to_lowercase().as_str() {
                    "exe" | "dll" | "so" | "bin" => return false,
                    _ => {}
                }
            }
        }

        true
    }
}

/// Parallel file processor for concurrent I/O
pub struct ParallelFileProcessor {
    config: IoConfig,
    thread_pool: rayon::ThreadPool,
}

impl ParallelFileProcessor {
    pub fn new(config: IoConfig) -> Self {
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(config.max_parallel_files)
            .build()
            .unwrap();

        Self {
            config,
            thread_pool,
        }
    }

    /// Process multiple files in parallel
    pub fn process_files<F, R>(&self, paths: Vec<PathBuf>, processor: F) -> Vec<R>
    where
        F: Fn(PathBuf) -> R + Send + Sync,
        R: Send,
    {
        self.thread_pool
            .install(|| paths.into_par_iter().map(processor).collect())
    }

    /// Process files with error handling
    pub fn process_files_with_error<F, R, E>(
        &self,
        paths: Vec<PathBuf>,
        processor: F,
    ) -> Vec<Result<R, E>>
    where
        F: Fn(PathBuf) -> Result<R, E> + Send + Sync,
        R: Send,
        E: Send,
    {
        self.thread_pool
            .install(|| paths.into_par_iter().map(processor).collect())
    }
}

/// Adaptive I/O strategy that chooses the best approach
pub struct AdaptiveIoStrategy {
    config: IoConfig,
    file_reader: OptimizedFileReader,
    parallel_processor: ParallelFileProcessor,
}

impl AdaptiveIoStrategy {
    pub fn new(config: IoConfig) -> Self {
        let file_reader = OptimizedFileReader::new(config.clone());
        let parallel_processor = ParallelFileProcessor::new(config.clone());

        Self {
            config,
            file_reader,
            parallel_processor,
        }
    }

    /// Choose the best I/O strategy for the given file
    pub fn choose_strategy(&self, path: &Path) -> IoStrategy {
        if let Ok(metadata) = std::fs::metadata(path) {
            let file_size = metadata.len() as usize;

            if file_size > self.config.mmap_threshold {
                IoStrategy::MemoryMapped
            } else if file_size > self.config.buffer_size * 4 {
                IoStrategy::Buffered
            } else {
                IoStrategy::Direct
            }
        } else {
            IoStrategy::Direct
        }
    }

    /// Process a single file with the optimal strategy
    pub fn process_file<F, R>(&self, path: PathBuf, processor: F) -> Result<R, std::io::Error>
    where
        F: FnOnce(Vec<u8>) -> R,
    {
        let strategy = self.choose_strategy(&path);

        match strategy {
            IoStrategy::MemoryMapped => {
                let content = self.file_reader.read_file_mmap(&path)?;
                Ok(processor(content))
            }
            IoStrategy::Buffered => {
                let content = self.file_reader.read_file_buffered(&path)?;
                Ok(processor(content))
            }
            IoStrategy::Direct => {
                let content = std::fs::read(&path)?;
                Ok(processor(content))
            }
        }
    }

    /// Process multiple files in parallel
    pub fn process_files_parallel<F, R>(
        &self,
        paths: Vec<PathBuf>,
        processor: F,
    ) -> Vec<Result<R, std::io::Error>>
    where
        F: Fn(Vec<u8>) -> R + Send + Sync,
        R: Send,
    {
        self.parallel_processor
            .process_files_with_error(paths, |path| self.process_file(path, &processor))
    }
}

/// I/O strategy types
#[derive(Debug, Clone, Copy)]
pub enum IoStrategy {
    Direct,
    Buffered,
    MemoryMapped,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_io_config_default() {
        let config = IoConfig::default();
        assert_eq!(config.buffer_size, 64 * 1024);
        assert!(config.prefetch_enabled);
        assert_eq!(config.prefetch_queue_size, 10);
    }

    #[test]
    fn test_optimized_file_reader() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let config = IoConfig::default();
        let reader = OptimizedFileReader::new(config);

        let content = reader.read_file(&test_file).unwrap();
        assert_eq!(content, b"test content");
    }

    #[test]
    fn test_adaptive_io_strategy() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let config = IoConfig::default();
        let strategy = AdaptiveIoStrategy::new(config);

        let result = strategy
            .process_file(test_file, |content| String::from_utf8(content).unwrap())
            .unwrap();

        assert_eq!(result, "test content");
    }

    #[test]
    fn test_should_process_file() {
        let config = IoConfig::default();
        let reader = OptimizedFileReader::new(config);

        let temp_dir = TempDir::new().unwrap();
        let text_file = temp_dir.path().join("test.txt");
        let exe_file = temp_dir.path().join("test.exe");

        fs::write(&text_file, "content").unwrap();
        fs::write(&exe_file, b"\x00\x01\x02").unwrap();

        assert!(reader.should_process_file(&text_file));
        assert!(!reader.should_process_file(&exe_file));
    }
}
