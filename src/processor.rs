// processor.rs
//! File-level search helpers and match extraction used by the rfgrep core.
use crate::error::{Result as RfgrepResult, RfgrepError};
// infer is used via `infer::get_from_path` when needed; no top-level import required here
use dashmap::DashMap;
use lazy_static::lazy_static;
use log::{debug, info, warn};
use memmap2::Mmap;
use regex::Regex;
use std::collections::VecDeque;
use std::fs::File;
use std::fs::Metadata;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::time::Instant;

const CONTEXT_LINES: usize = 2;
const BINARY_CHECK_SIZE: usize = 8000;
const MMAP_THRESHOLD: u64 = 16 * 1024 * 1024;
const MAX_SCAN_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100 MB

/// Get adaptive mmap threshold based on available system memory
fn get_adaptive_mmap_threshold() -> u64 {
    #[cfg(unix)]
    {
        use std::fs;
        if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
            if let Some(available_line) = meminfo
                .lines()
                .find(|line| line.starts_with("MemAvailable:"))
            {
                if let Some(kb_str) = available_line.split_whitespace().nth(1) {
                    if let Ok(kb) = kb_str.parse::<u64>() {
                        // Use 1/8 of available memory, but cap at 1GB
                        let threshold = (kb * 1024 / 8).min(1024 * 1024 * 1024);
                        return threshold.max(MMAP_THRESHOLD);
                    }
                }
            }
        }
    }

    // Fallback to default threshold
    MMAP_THRESHOLD
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SearchMatch {
    pub path: std::path::PathBuf,
    pub line_number: usize,
    pub line: String,
    pub context_before: Vec<(usize, String)>,
    pub context_after: Vec<(usize, String)>,
    pub matched_text: String,
    pub column_start: usize,
    pub column_end: usize,
}

lazy_static! {
    static ref REGEX_CACHE: DashMap<String, Regex> = DashMap::new();
}

pub fn is_binary(file: &Path) -> bool {
    if let Ok(Some(k)) = infer::get_from_path(file) {
        if !k.mime_type().starts_with("text/") {
            debug!(
                "Infer detected binary for {}: {}",
                file.display(),
                k.mime_type()
            );
            return true;
        }
    }

    if let Ok(mut f) = File::open(file) {
        let mut buffer = vec![0u8; BINARY_CHECK_SIZE];
        match f.read(&mut buffer) {
            Ok(n) if n > 0 => {
                let null_bytes = buffer[..n].iter().filter(|&&b| b == 0).count();
                let binary_threshold = (n as f64 * 0.1).max(1.0);
                if (null_bytes as f64) > binary_threshold {
                    debug!(
                        "Null byte heuristic detected binary file: {}",
                        f.metadata().map(|m| m.len()).unwrap_or(0)
                    );
                    return true;
                }
            }
            Ok(_) => { /* zero bytes read, treat as non-binary */ }
            Err(e) => {
                debug!("Failed to read sample from {}: {}", file.display(), e);
            }
        }
    }
    false
}

/// Decide whether a file should be skipped entirely before attempting to read/scan it.
/// Uses extension blacklist, mime detection, executable heuristics and size thresholds.
pub fn should_skip(path: &Path, metadata: &Metadata) -> bool {
    // Skip directories (caller should avoid passing directories, but be defensive)
    if metadata.is_dir() {
        return true;
    }

    // Skip kernel pseudo-filesystems like /proc and device nodes under /dev by default
    if let Ok(s) = path.canonicalize() {
        if let Some(root_str) = s.to_str() {
            if root_str.starts_with("/proc") || root_str.starts_with("/dev") {
                debug!("Skipping kernel fs path: {}", path.display());
                return true;
            }
        }
    }

    // Skip special file types (sockets, pipes, block/char devices)
    let ftype = metadata.file_type();
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        if ftype.is_socket()
            || ftype.is_fifo()
            || ftype.is_block_device()
            || ftype.is_char_device()
            || ftype.is_symlink()
        {
            debug!("Skipping special unix file type: {}", path.display());
            return true;
        }
    }
    #[cfg(windows)]
    {
        if ftype.is_symlink() {
            debug!("Skipping symlink on windows: {}", path.display());
            return true;
        }
    }

    // 1) Extension blacklist (common media, archives, binaries)
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        let ext = ext.to_ascii_lowercase();
        const SKIP_EXTENSIONS: &[&str] = &[
            "jpg", "jpeg", "png", "gif", "bmp", "webp", "ico", "mp4", "mkv", "mov", "avi", "flv",
            "wmv", "webm", "mp3", "flac", "wav", "aac", "ogg", "pdf", "doc", "docx", "xls", "xlsx",
            "ppt", "pptx", "exe", "dll", "so", "dylib", "appimage", "bin", "class", "jar", "iso",
            "img", "cab", "zip", "tar", "gz", "tgz", "7z", "rar",
        ];
        if SKIP_EXTENSIONS.contains(&ext.as_str()) {
            debug!("Skipping by extension: {}", path.display());
            return true;
        }
    }

    // 2) Executable heuristic: on Unix, if executable bit set and no text extension, skip
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perm = metadata.permissions();
        if (perm.mode() & 0o111) != 0 {
            debug!("Skipping executable file: {}", path.display());
            return true;
        }
    }
    #[cfg(windows)]
    {
        if let Some(ext) = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
        {
            if ext == "exe" || ext == "dll" || ext == "msi" {
                debug!("Skipping Windows executable: {}", path.display());
                return true;
            }
        }
    }

    // 3) Size threshold: avoid scanning huge files (likely media/archives)
    let file_size = metadata.len();
    if file_size > MAX_SCAN_FILE_SIZE {
        debug!(
            "Skipping large file (> {} bytes): {}",
            MAX_SCAN_FILE_SIZE,
            path.display()
        );
        return true;
    }

    // 4) Mime/type detection via infer: skip non-text types
    if let Ok(Some(kind)) = infer::get_from_path(path) {
        let mime = kind.mime_type();
        if !mime.starts_with("text/") {
            debug!(
                "Infer detected non-text mime (skip): {} -> {}",
                path.display(),
                mime
            );
            return true;
        }
    }

    false
}

fn is_binary_content(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }
    let sample_size = data.len().min(BINARY_CHECK_SIZE);
    let null_bytes = data[..sample_size].iter().filter(|&b| *b == 0).count();
    (null_bytes as f64 / sample_size as f64) > 0.3
}

pub fn get_or_compile_regex(pattern: &str) -> RfgrepResult<Regex> {
    if let Some(regex) = REGEX_CACHE.get(pattern) {
        debug!("Regex cache hit for pattern: {pattern}");
        Ok(regex.clone())
    } else {
        debug!("Regex cache miss for pattern: {pattern}. Compiling.");
        let regex = Regex::new(pattern).map_err(RfgrepError::Regex)?;
        REGEX_CACHE.insert(pattern.to_string(), regex.clone());
        Ok(regex)
    }
}

pub fn search_file(path: &Path, pattern: &Regex) -> RfgrepResult<Vec<SearchMatch>> {
    let _start = Instant::now();
    let file_display = path.display();
    debug!("Starting search in file: {file_display}");
    let file = File::open(path).map_err(RfgrepError::Io)?;
    let metadata = file.metadata().map_err(RfgrepError::Io)?;
    let file_size = metadata.len();

    // Quick heuristic to skip problematic files before trying to mmap/read them.
    if should_skip(path, &metadata) {
        info!("Skipping file by pre-scan heuristic: {file_display}");
        return Ok(vec![]);
    }

    let matches_found = if file_size >= get_adaptive_mmap_threshold() {
        debug!("Attempting memory mapping for file: {file_display} ({file_size} bytes)");
        match unsafe { Mmap::map(&file) } {
            Ok(mmap) => {
                debug!("Successfully memory mapped file: {file_display}");
                if is_binary_content(&mmap) {
                    info!("Skipping binary file (mmap): {file_display}");
                    return Ok(vec![]);
                }
                // SAFETY: Validate UTF-8 before conversion
                match std::str::from_utf8(&mmap) {
                    Ok(content) => find_matches_with_context(content.to_string(), pattern, path)?,
                    Err(e) => {
                        warn!(
                            "Invalid UTF-8 in file {file_display}, falling back to streaming: {e}"
                        );
                        let reader = BufReader::new(file);
                        find_matches_streaming(reader, pattern, path)?
                    }
                }
            }
            Err(_) => {
                warn!("Failed to memory map, falling back to streaming: {file_display}");
                let reader = BufReader::new(file);
                find_matches_streaming(reader, pattern, path)?
            }
        }
    } else {
        let reader = BufReader::new(file);
        find_matches_streaming(reader, pattern, path)?
    };

    debug!(
        "Finished search in file: {} ({} matches found)",
        file_display,
        matches_found.len()
    );
    Ok(matches_found)
}

fn find_matches_with_context(
    content: String,
    pattern: &Regex,
    path: &Path,
) -> RfgrepResult<Vec<SearchMatch>> {
    let mut matches = Vec::new();
    let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    for (i, line) in lines.iter().enumerate() {
        if let Some(m) = pattern.find(line) {
            let start_idx = i.saturating_sub(CONTEXT_LINES);
            let context_before: Vec<(usize, String)> = (start_idx..i)
                .map(|idx| (idx + 1, lines[idx].clone()))
                .collect();
            let end_idx = (i + CONTEXT_LINES + 1).min(lines.len());
            let context_after: Vec<(usize, String)> = ((i + 1)..end_idx)
                .map(|idx| (idx + 1, lines[idx].to_string()))
                .collect();
            matches.push(SearchMatch {
                path: path.to_path_buf(),
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
    let mut buffer: VecDeque<(usize, String)> = VecDeque::with_capacity(2 * CONTEXT_LINES + 1);
    let mut lines_iter = reader.lines();
    let mut line_no = 0usize;
    while let Some(line_res) = lines_iter.next() {
        line_no += 1;
        let line = line_res.map_err(RfgrepError::Io)?;
        buffer.push_back((line_no, line.clone()));
        if buffer.len() > 2 * CONTEXT_LINES + 1 {
            buffer.pop_front();
        }
        if let Some(m) = pattern.find(&line) {
            let context_before: Vec<(usize, String)> =
                buffer.iter().take(CONTEXT_LINES).cloned().collect();
            let mut context_after = Vec::new();
            for next in lines_iter.by_ref().take(CONTEXT_LINES) {
                let nline = next.map_err(RfgrepError::Io)?;
                line_no += 1;
                context_after.push((line_no, nline));
            }
            matches.push(SearchMatch {
                path: path.to_path_buf(),
                line_number: line_no,
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
