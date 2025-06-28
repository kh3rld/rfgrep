use anyhow::{Context, Result};
use colored::*;
use lazy_static::lazy_static;
use memmap2::Mmap;
use regex::Regex;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Instant;

const CONTEXT_LINES: usize = 2;
const BINARY_CHECK_SIZE: usize = 8000;

#[derive(Debug)]
pub struct SearchMatch {
    pub line_number: usize,
    pub line: String,
    pub context_before: Vec<(usize, String)>,
    pub context_after: Vec<(usize, String)>,
    pub column_start: usize,
    pub column_end: usize,
}

// Regex cache for common patterns
lazy_static! {
    static ref REGEX_CACHE: std::sync::Mutex<std::collections::HashMap<String, Regex>> =
        std::sync::Mutex::new(std::collections::HashMap::new());
}

/// Get a cached regex or compile and cache it
pub fn get_or_compile_regex(pattern: &str) -> Result<Regex> {
    let mut cache = REGEX_CACHE.lock().unwrap();
    if let Some(re) = cache.get(pattern) {
        return Ok(re.clone());
    }
    let re = Regex::new(pattern)?;
    cache.insert(pattern.to_string(), re.clone());
    Ok(re)
}

pub fn is_binary(file: &Path) -> bool {
    if let Ok(mut file) = File::open(file) {
        let mut buffer = vec![0u8; BINARY_CHECK_SIZE];
        if let Ok(n) = file.read(&mut buffer) {
            if n > 0 {
                let null_bytes = buffer[..n].iter().filter(|&&b| b == 0).count();
                return (null_bytes as f64 / n as f64) > 0.3;
            }
        }
    }
    false
}

/// Search file using memory mapping for I/O efficiency
pub fn search_file(path: &Path, pattern: &Regex) -> Result<Vec<String>> {
    let start = Instant::now();
    let file = File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
    let mmap =
        unsafe { Mmap::map(&file).with_context(|| format!("Failed to mmap {}", path.display()))? };
    let content = std::str::from_utf8(&mmap).unwrap_or("");
    let mut output = Vec::new();
    let mut current_context: Vec<SearchMatch> = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if let Some(m) = pattern.find(line) {
            let start_idx = i.saturating_sub(CONTEXT_LINES);
            let context_before: Vec<(usize, String)> = (start_idx..i)
                .map(|idx| (idx + 1, lines[idx].to_string()))
                .collect();
            let end_idx = (i + CONTEXT_LINES + 1).min(lines.len());
            let context_after: Vec<(usize, String)> = ((i + 1)..end_idx)
                .map(|idx| (idx + 1, lines[idx].to_string()))
                .collect();
            current_context.push(SearchMatch {
                line_number: i + 1,
                line: line.to_string(),
                context_before,
                context_after,
                column_start: m.start(),
                column_end: m.end(),
            });
        }
    }
    if !current_context.is_empty() {
        let elapsed = start.elapsed();
        output.push(format!("\n{} {}:", "File".green().bold(), path.display()));
        output.push(format!(
            "{} {} match(es) in {:.2}ms",
            "Found".green(),
            current_context.len(),
            elapsed.as_millis()
        ));
        for m in current_context {
            output.push("-".repeat(80).dimmed().to_string());
            for (num, line) in m.context_before {
                output.push(format!(
                    "  {} │ {}",
                    num.to_string().dimmed(),
                    line.dimmed()
                ));
            }
            let before = &m.line[..m.column_start];
            let matched = &m.line[m.column_start..m.column_end];
            let after = &m.line[m.column_end..];
            output.push(format!(
                "→ {} │ {}{}{}",
                m.line_number.to_string().yellow().bold(),
                before,
                matched.yellow().bold(),
                after
            ));
            for (num, line) in m.context_after {
                output.push(format!(
                    "  {} │ {}",
                    num.to_string().dimmed(),
                    line.dimmed()
                ));
            }
        }
    }
    Ok(output)
}
