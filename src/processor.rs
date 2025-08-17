use crate::error::{Result as RfgrepResult, RfgrepError};
use anyhow::Context;
use colored::*;
use lazy_static::lazy_static;
use log::{debug, info, warn};
use lru::LruCache;
use memmap2::Mmap;
use regex::Regex;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;
const CONTEXT_LINES: usize = 2;
const BINARY_CHECK_SIZE: usize = 8000;
const MMAP_THRESHOLD: u64 = 16 * 1024 * 1024; 
const REGEX_CACHE_SIZE: usize = 100; 

#[derive(Debug)]
pub struct SearchMatch {
    pub line_number: usize,
    pub line: String,
    pub context_before: Vec<(usize, String)>,
    pub context_after: Vec<(usize, String)>,
    pub matched_text: String,
    pub column_start: usize,
    pub column_end: usize,
}

lazy_static! {
    static ref REGEX_CACHE: Mutex<LruCache<String, Regex>> =
        Mutex::new(LruCache::new(REGEX_CACHE_SIZE.try_into().unwrap()));
}

pub fn is_binary(file: &Path) -> bool {
    // Use infer for initial magic number detection
    if let Ok(Some(k)) = infer::get_from_path(file) {
        // Todo: Implement a more robust check, involve a list of known text MIME types
        if !k.mime_type().starts_with("text/") {
            debug!(
                "Infer detected binary file type for {}: {}",
                file.display(),
                k.mime_type()
            );
            return true;
        }
    }

    // Fallback to null byte check for unknown or ambiguous types
    if let Ok(mut file) = File::open(file) {
        let mut buffer = vec![0u8; BINARY_CHECK_SIZE];
        if let Ok(n) = file.read(&mut buffer) {
            if n > 0 {
                let null_bytes = buffer[..n].iter().filter(|&&b| b == 0).count();
                let binary_threshold = (n as f64 * 0.1).max(1.0); 
                if null_bytes as f64 > binary_threshold {
                    debug!(
                        "Null byte heuristic detected binary file for {}",
                        file.metadata().map(|m| m.len()).unwrap_or(0)
                    );
                    return true;
                }
            }
        }
    }
    false
}

// Helper function to check binary content in a byte slice (for mmap)
fn is_binary_content(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }
    let sample_size = data.len().min(BINARY_CHECK_SIZE);
    let null_bytes = data[..sample_size].iter().filter(|&b| *b == 0).count();
    (null_bytes as f64 / sample_size as f64) > 0.3
}

// Function to get or compile a regex, using the cache
pub fn get_or_compile_regex(pattern: &str) -> RfgrepResult<Regex> {
    let mut cache = REGEX_CACHE.lock().unwrap();

    if let Some(regex) = cache.get(pattern) {
        debug!("Regex cache hit for pattern: {pattern}");
        Ok(regex.clone())
    } else {
        debug!("Regex cache miss for pattern: {pattern}. Compiling.");
        let regex = Regex::new(pattern).map_err(RfgrepError::Regex)?; 
        cache.put(pattern.to_string(), regex.clone());
        Ok(regex)
    }
}

pub fn search_file(path: &Path, pattern: &Regex) -> RfgrepResult<Vec<String>> {
    let start = Instant::now();
    let file_display = path.display();
    debug!("Starting search in file: {file_display}");

    let file = File::open(path)
        .map_err(RfgrepError::Io)
        .with_context(|| format!("Failed to open {file_display}"))?;
    let metadata = file
        .metadata()
        .map_err(RfgrepError::Io)
        .with_context(|| format!("Failed to get metadata for {file_display}"))?;
    let file_size = metadata.len();

    let matches_found = if file_size >= MMAP_THRESHOLD {
        debug!("Attempting memory mapping for file: {file_display} ({file_size} bytes)");
        match unsafe { Mmap::map(&file) } {
            Ok(mmap) => {
                debug!("Successfully memory mapped file: {file_display}");
                if is_binary_content(&mmap) {
                    info!("Skipping binary file (mmap): {file_display}");
                    return Ok(vec![]); 
                } 
                let content = unsafe { std::str::from_utf8_unchecked(&mmap) };
                find_matches_with_context(content.to_string(), pattern, path)?
            }
            Err(e) => {
                warn!("Failed to memory map file {file_display}: {e}. Falling back to streaming.");
                let reader = BufReader::new(file);
                find_matches_streaming(reader, pattern, path)?
            }
        }
    } else {
        debug!("Using streaming for file: {file_display} ({file_size} bytes)");
        let reader = BufReader::new(file);
        find_matches_streaming(reader, pattern, path)?
    };

    let elapsed = start.elapsed();
    debug!(
        "Finished search in file: {} ({} matches found in {:.2?})",
        file_display,
        matches_found.len(),
        elapsed
    );

    Ok(format_matches(path, matches_found, elapsed))
}

// Helper function to find matches and collect context for mmapped content
fn find_matches_with_context(
    content: String,
    pattern: &Regex,
    path: &Path,
) -> RfgrepResult<Vec<SearchMatch>> {
    let mut matches = Vec::new();
    let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    for (i, line) in lines.iter().enumerate() {
        if let Some(m) = pattern.find(line) {
            debug!("Match found in {}: line {}", path.display(), i + 1);
            // Get context before
            let start_idx = i.saturating_sub(CONTEXT_LINES);
            let context_before: Vec<(usize, String)> = (start_idx..i)
                .map(|idx| (idx + 1, lines[idx].clone())) 
                .collect();

            let end_idx = (i + CONTEXT_LINES + 1).min(lines.len());
            let context_after: Vec<(usize, String)> = ((i + 1)..end_idx)
                .map(|idx| (idx + 1, lines[idx].to_string()))
                .collect();

            matches.push(SearchMatch {
                line_number: i + 1,
                line: line.clone(),
                context_before,
                context_after,
                matched_text: m.as_str().to_string(),
                column_start: m.start(),
                column_end: m.end(),
            });
        }
    }
    Ok(matches)
}

fn find_matches_streaming<R: Read>(
    reader: BufReader<R>,
    pattern: &Regex,
    path: &Path,
) -> RfgrepResult<Vec<SearchMatch>> {
    let mut matches = Vec::new();
    let mut lines_buffer: VecDeque<(usize, String)> =
        VecDeque::with_capacity(2 * CONTEXT_LINES + 1);
    let mut reader_lines = reader.lines();
    let mut current_line_number = 0;

    while let Some(line_result) = reader_lines.next() {
        current_line_number += 1;
        let line = line_result.map_err(RfgrepError::Io).with_context(|| {
            format!(
                "Failed to read line {} from {}",
                current_line_number,
                path.display()
            )
        })?;

        lines_buffer.push_back((current_line_number, line.clone()));
        if lines_buffer.len() > 2 * CONTEXT_LINES + 1 {
            lines_buffer.pop_front();
        }

        if let Some(m) = pattern.find(&line) {
            debug!(
                "Match found in {}: line {} (streaming)",
                path.display(),
                current_line_number
            );
            let context_before: Vec<(usize, String)> =
                lines_buffer.iter().take(CONTEXT_LINES).cloned().collect();

            let mut context_after = Vec::new();
            let temp_reader_lines = reader_lines.by_ref().take(CONTEXT_LINES);
            for next_line_result in temp_reader_lines {
                let next_line_number = current_line_number + context_after.len() + 1;
                let next_line = next_line_result.map_err(RfgrepError::Io).with_context(|| {
                    format!(
                        "Failed to read line {} from {}",
                        next_line_number,
                        path.display()
                    )
                })?;
                context_after.push((next_line_number, next_line));
            }

            matches.push(SearchMatch {
                line_number: current_line_number,
                line: line.clone(),
                context_before,
                context_after,
                matched_text: m.as_str().to_string(),
                column_start: m.start(),
                column_end: m.end(),
            });
        }
    }

    Ok(matches)
}

fn format_matches(
    path: &Path,
    matches: Vec<SearchMatch>,
    elapsed: std::time::Duration,
) -> Vec<String> {
    let mut output = Vec::new();
    if !matches.is_empty() {
        output.push(format!("\n{} {}:", "File".green().bold(), path.display()));
        output.push(format!(
            "{} {} match(es) in {:.2}ms",
            "Found".green(),
            matches.len(),
            elapsed.as_millis()
        ));

        for (i, m) in matches.iter().enumerate() {
            if i > 0 {
                output.push("".to_string()); 
            }

            for (num, line) in &m.context_before {
                output.push(format!(
                    "  {} │ {}",
                    num.to_string().dimmed(),
                    line.dimmed()
                ));
            }

            let before = &m.line[..m.column_start];
            let matched = &m.matched_text;
            let after = &m.line[m.column_end..];
            output.push(format!(
                "→ {} │ {}{}{}",
                m.line_number.to_string().yellow().bold(),
                before,
                matched.yellow().bold(),
                after
            ));
            for (num, line) in &m.context_after {
                output.push(format!(
                    "  {} │ {}",
                    num.to_string().dimmed(),
                    line.dimmed()
                ));
            }
        }
    }
    output
}
