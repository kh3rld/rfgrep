//! Streaming search implementation for memory-efficient processing
use crate::processor::SearchMatch;
use crate::search::algorithms::SearchAlgorithmTrait;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

/// Streaming search engine that processes files in chunks
pub struct StreamingSearch {
    chunk_size: usize,
    buffer_pool: BufferPool,
}

/// Pool of reusable buffers to avoid allocations
struct BufferPool {
    buffers: Vec<Vec<u8>>,
    max_size: usize,
}

impl BufferPool {
    fn new(max_size: usize) -> Self {
        Self {
            buffers: Vec::new(),
            max_size,
        }
    }

    fn get_buffer(&mut self, size: usize) -> Vec<u8> {
        self.buffers
            .pop()
            .filter(|buf| buf.capacity() >= size)
            .unwrap_or_else(|| vec![0u8; size])
    }

    fn return_buffer(&mut self, mut buffer: Vec<u8>) {
        if self.buffers.len() < self.max_size {
            buffer.clear();
            self.buffers.push(buffer);
        }
    }
}

impl StreamingSearch {
    /// Create a new streaming search engine
    pub fn new(chunk_size: usize) -> Self {
        Self {
            chunk_size,
            buffer_pool: BufferPool::new(10), // Keep 10 buffers in pool
        }
    }

    /// Search a file using streaming approach
    pub fn search_file_streaming<A>(
        &mut self,
        path: &Path,
        pattern: &str,
        algorithm: &A,
        context_lines: usize,
    ) -> crate::error::Result<Vec<SearchMatch>>
    where
        A: SearchAlgorithmTrait,
    {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut matches = Vec::new();
        let mut _line_buffer: VecDeque<String> = VecDeque::new();
        let mut line_number = 0;
        let mut context_before = VecDeque::new();

        // Read file in chunks
        let mut buffer = self.buffer_pool.get_buffer(self.chunk_size);
        let mut remaining = String::new();

        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    let chunk = String::from_utf8_lossy(&buffer[..n]);
                    let full_text = remaining + &chunk;

                    // Process complete lines
                    let lines: Vec<&str> = full_text.lines().collect();
                    let last_line_incomplete = !full_text.ends_with('\n');

                    let lines_to_process = if last_line_incomplete {
                        lines.len() - 1
                    } else {
                        lines.len()
                    };

                    for i in 0..lines_to_process {
                        line_number += 1;
                        let line = lines[i];

                        // Add to context buffer
                        context_before.push_back((line_number, line.to_string()));
                        if context_before.len() > context_lines {
                            context_before.pop_front();
                        }

                        // Search in this line
                        let line_matches = algorithm.search(line, pattern);
                        for &match_pos in &line_matches {
                            let context_before_vec: Vec<(usize, String)> =
                                context_before.iter().take(context_lines).cloned().collect();

                            matches.push(SearchMatch {
                                path: path.to_path_buf(),
                                line_number,
                                line: line.to_string(),
                                context_before: context_before_vec,
                                context_after: Vec::new(), // Will be filled later
                                matched_text: pattern.to_string(),
                                column_start: match_pos,
                                column_end: match_pos + pattern.len(),
                            });
                        }
                    }

                    // Keep incomplete line for next iteration
                    remaining = if last_line_incomplete {
                        lines.last().unwrap().to_string()
                    } else {
                        String::new()
                    };
                }
                Err(e) => return Err(crate::error::RfgrepError::Io(e)),
            }
        }

        // Process remaining text
        if !remaining.is_empty() {
            line_number += 1;
            let line_matches = algorithm.search(&remaining, pattern);
            for &match_pos in &line_matches {
                let context_before_vec: Vec<(usize, String)> =
                    context_before.iter().take(context_lines).cloned().collect();

                matches.push(SearchMatch {
                    path: path.to_path_buf(),
                    line_number,
                    line: remaining.clone(),
                    context_before: context_before_vec,
                    context_after: Vec::new(),
                    matched_text: pattern.to_string(),
                    column_start: match_pos,
                    column_end: match_pos + pattern.len(),
                });
            }
        }

        // Fill context_after for all matches
        self.fill_context_after(&mut matches, path, context_lines)?;

        self.buffer_pool.return_buffer(buffer);
        Ok(matches)
    }

    /// Fill context_after for matches by reading ahead
    fn fill_context_after(
        &self,
        matches: &mut [SearchMatch],
        path: &Path,
        context_lines: usize,
    ) -> crate::error::Result<()> {
        if context_lines == 0 {
            return Ok(());
        }

        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().collect::<Result<Vec<_>, _>>()?;

        for match_ in matches.iter_mut() {
            let start_line = match_.line_number;
            let end_line = (start_line + context_lines).min(lines.len());

            match_.context_after = lines[start_line..end_line]
                .iter()
                .enumerate()
                .map(|(i, line)| (start_line + i + 1, line.clone()))
                .collect();
        }

        Ok(())
    }

    /// Search multiple files in parallel using streaming
    pub fn search_files_parallel<A>(
        &mut self,
        files: &[std::path::PathBuf],
        pattern: &str,
        algorithm: &A,
        context_lines: usize,
    ) -> crate::error::Result<Vec<SearchMatch>>
    where
        A: SearchAlgorithmTrait + Send + Sync,
    {
        use rayon::prelude::*;
        use std::sync::Mutex;

        let matches = Mutex::new(Vec::new());
        let errors = Mutex::new(Vec::new());

        files.par_iter().for_each(|path| {
            let mut local_search = StreamingSearch::new(self.chunk_size);
            match local_search.search_file_streaming(path, pattern, algorithm, context_lines) {
                Ok(file_matches) => {
                    if !file_matches.is_empty() {
                        matches.lock().unwrap().extend(file_matches);
                    }
                }
                Err(e) => {
                    errors.lock().unwrap().push(e);
                }
            }
        });

        let collected_errors = errors.into_inner().unwrap();
        if !collected_errors.is_empty() {
            eprintln!("Errors encountered during streaming search:");
            for err in collected_errors {
                eprintln!("  {}", err);
            }
        }

        Ok(matches.into_inner().unwrap())
    }
}

/// Memory-mapped search for very large files
pub struct MemoryMappedSearch {
    mmap_threshold: usize,
}

impl MemoryMappedSearch {
    pub fn new(mmap_threshold: usize) -> Self {
        Self { mmap_threshold }
    }

    /// Search using memory mapping for large files
    pub fn search_mmap<A>(
        &self,
        path: &Path,
        pattern: &str,
        algorithm: &A,
        context_lines: usize,
    ) -> crate::error::Result<Vec<SearchMatch>>
    where
        A: SearchAlgorithmTrait,
    {
        let metadata = std::fs::metadata(path)?;
        if metadata.len() as usize > self.mmap_threshold {
            self.search_with_mmap(path, pattern, algorithm, context_lines)
        } else {
            // Use regular file reading for smaller files
            let content = std::fs::read_to_string(path)?;
            let matches = algorithm.search_with_context(&content, pattern, context_lines);
            Ok(matches
                .into_iter()
                .map(|mut m| {
                    m.path = path.to_path_buf();
                    m
                })
                .collect())
        }
    }

    fn search_with_mmap<A>(
        &self,
        path: &Path,
        pattern: &str,
        algorithm: &A,
        context_lines: usize,
    ) -> crate::error::Result<Vec<SearchMatch>>
    where
        A: SearchAlgorithmTrait,
    {
        use memmap2::Mmap;

        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        // Convert to string safely
        let content = std::str::from_utf8(&mmap)
            .map_err(|e| crate::error::RfgrepError::Other(format!("Invalid UTF-8: {e}")))?;

        let matches = algorithm.search_with_context(content, pattern, context_lines);
        Ok(matches
            .into_iter()
            .map(|mut m| {
                m.path = path.to_path_buf();
                m
            })
            .collect())
    }
}

/// Adaptive search that chooses the best strategy based on file characteristics
pub struct AdaptiveSearch {
    streaming: StreamingSearch,
    mmap: MemoryMappedSearch,
    small_file_threshold: usize,
    large_file_threshold: usize,
}

impl AdaptiveSearch {
    pub fn new() -> Self {
        Self {
            streaming: StreamingSearch::new(64 * 1024), // 64KB chunks
            mmap: MemoryMappedSearch::new(100 * 1024 * 1024), // 100MB threshold
            small_file_threshold: 1024 * 1024,          // 1MB
            large_file_threshold: 100 * 1024 * 1024,    // 100MB
        }
    }

    /// Choose the best search strategy based on file size
    pub fn search_adaptive<A>(
        &mut self,
        path: &Path,
        pattern: &str,
        algorithm: &A,
        context_lines: usize,
    ) -> crate::error::Result<Vec<SearchMatch>>
    where
        A: SearchAlgorithmTrait,
    {
        let metadata = std::fs::metadata(path)?;
        let file_size = metadata.len() as usize;

        if file_size < self.small_file_threshold {
            // Small file: read entirely into memory
            let content = std::fs::read_to_string(path)?;
            let matches = algorithm.search_with_context(&content, pattern, context_lines);
            Ok(matches
                .into_iter()
                .map(|mut m| {
                    m.path = path.to_path_buf();
                    m
                })
                .collect())
        } else if file_size < self.large_file_threshold {
            // Medium file: use streaming
            self.streaming
                .search_file_streaming(path, pattern, algorithm, context_lines)
        } else {
            // Large file: use memory mapping
            self.mmap
                .search_mmap(path, pattern, algorithm, context_lines)
        }
    }
}
