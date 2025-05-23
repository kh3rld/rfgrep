use anyhow::{Context, Result};
use colored::*;
use regex::Regex;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
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
    pub matched_text: String,
    pub column_start: usize,
    pub column_end: usize,
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

pub fn search_file(path: &Path, pattern: &Regex) -> Result<Vec<String>> {
    let start = Instant::now();
    let file = File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().filter_map(io::Result::ok).collect();
    let mut output = Vec::new();
    let mut current_context: Vec<SearchMatch> = Vec::new();

    // Find all matches with context
    for (i, line) in lines.iter().enumerate() {
        if let Some(m) = pattern.find(line) {
            // Get context before
            let start_idx = if i >= CONTEXT_LINES {
                i - CONTEXT_LINES
            } else {
                0
            };
            let context_before: Vec<(usize, String)> = (start_idx..i)
                .map(|idx| (idx + 1, lines[idx].clone()))
                .collect();

            // Get context after
            let end_idx = (i + CONTEXT_LINES + 1).min(lines.len());
            let context_after: Vec<(usize, String)> = ((i + 1)..end_idx)
                .map(|idx| (idx + 1, lines[idx].clone()))
                .collect();

            current_context.push(SearchMatch {
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

    // Format matches with context
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

            // Print context before
            for (num, line) in m.context_before {
                output.push(format!(
                    "  {} │ {}",
                    num.to_string().dimmed(),
                    line.dimmed()
                ));
            }

            // Print matching line with highlighted match
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

            // Print context after
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
