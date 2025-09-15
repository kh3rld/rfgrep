//! File listing engine with advanced filtering and statistics
use crate::error::{Result as RfgrepResult, RfgrepError};
use crate::metrics::Metrics;
use crate::walker::walk_dir;
use colored::*;
use std::path::Path;
use std::sync::Arc;

/// File information for listing
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: std::path::PathBuf,
    pub size: u64,
    pub extension: String,
    pub is_binary: bool,
    pub modified: Option<std::time::SystemTime>,
}

/// List engine for file operations
pub struct ListEngine {
    metrics: Arc<Metrics>,
}

impl ListEngine {
    /// Create a new list engine
    pub fn new(metrics: Arc<Metrics>) -> RfgrepResult<Self> {
        Ok(Self { metrics })
    }

    /// List files with various filters and options
    pub async fn list_files(
        &self,
        root_path: &Path,
        recursive: bool,
        show_hidden: bool,
        extensions: Option<&[String]>,
        max_size: Option<usize>,
        min_size: Option<usize>,
        sort: crate::cli::SortCriteria,
        reverse: bool,
        limit: Option<usize>,
    ) -> RfgrepResult<Vec<FileInfo>> {
        let entries: Vec<_> = walk_dir(root_path, recursive, show_hidden).collect();

        // Process files in parallel
        use rayon::prelude::*;
        use std::sync::Mutex;

        let file_results = Mutex::new(Vec::new());
        let errors = Mutex::new(Vec::new());

        entries.par_iter().for_each(|entry| {
            let path = entry.path();
            if path.is_dir() {
                return;
            }

            if !self.should_include_file(path, extensions, max_size, min_size) {
                return;
            }

            match self.get_file_info(path) {
                Ok(file_info) => {
                    file_results.lock().unwrap().push(file_info);
                }
                Err(e) => {
                    errors.lock().unwrap().push(e);
                }
            }
        });

        let collected_errors = errors.into_inner().unwrap();
        if !collected_errors.is_empty() {
            eprintln!("Errors encountered during file listing:");
            for err in collected_errors {
                eprintln!("  {err}");
            }
        }

        let mut files = file_results.into_inner().unwrap();

        // Sort files
        self.sort_files(&mut files, sort, reverse);

        // Apply limit
        if let Some(limit) = limit {
            files.truncate(limit);
        }

        // Update metrics
        self.metrics.files_scanned.inc_by(files.len() as u64);

        Ok(files)
    }

    /// Check if a file should be included based on filters
    fn should_include_file(
        &self,
        path: &Path,
        extensions: Option<&[String]>,
        max_size: Option<usize>,
        min_size: Option<usize>,
    ) -> bool {
        // Extension filter
        if let Some(extensions) = extensions {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if !extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Size filters
        if let Ok(metadata) = path.metadata() {
            let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);

            if let Some(max) = max_size {
                if size_mb > max as f64 {
                    return false;
                }
            }

            if let Some(min) = min_size {
                if size_mb < min as f64 {
                    return false;
                }
            }
        }

        true
    }

    /// Get file information
    fn get_file_info(&self, path: &Path) -> RfgrepResult<FileInfo> {
        let metadata = std::fs::metadata(path)?;
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("none")
            .to_string();

        Ok(FileInfo {
            path: path.to_path_buf(),
            size: metadata.len(),
            extension,
            is_binary: crate::processor::is_binary(path),
            modified: metadata.modified().ok(),
        })
    }

    /// Sort files based on criteria
    fn sort_files(&self, files: &mut [FileInfo], sort: crate::cli::SortCriteria, reverse: bool) {
        match sort {
            crate::cli::SortCriteria::Name => {
                files.sort_by(|a, b| a.path.cmp(&b.path));
            }
            crate::cli::SortCriteria::Size => {
                files.sort_by(|a, b| a.size.cmp(&b.size));
            }
            crate::cli::SortCriteria::Date => {
                files.sort_by(|a, b| match (&a.modified, &b.modified) {
                    (Some(a_time), Some(b_time)) => a_time.cmp(b_time),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                });
            }
            crate::cli::SortCriteria::Type => {
                files.sort_by(|a, b| a.extension.cmp(&b.extension));
            }
            crate::cli::SortCriteria::Path => {
                files.sort_by(|a, b| a.path.cmp(&b.path));
            }
        }

        if reverse {
            files.reverse();
        }
    }

    /// Print files in long format
    pub fn print_long_format(&self, files: &[FileInfo]) {
        if files.is_empty() {
            println!("No files found");
            return;
        }

        println!("{:<12} {:<8} {:<6} {}", "SIZE", "BINARY", "TYPE", "PATH");
        println!("{}", "-".repeat(50));

        for file in files {
            let size_str = self.format_size(file.size);
            let binary_str = if file.is_binary { "yes" } else { "no" };
            let type_str = if file.extension == "none" {
                "file"
            } else {
                &file.extension
            };

            println!(
                "{:<12} {:<8} {:<6} {}",
                size_str,
                binary_str,
                type_str,
                file.path.display()
            );
        }
    }

    /// Print files in simple format
    pub fn print_simple_list(&self, files: &[FileInfo]) {
        for file in files {
            println!("{}", file.path.display());
        }
    }

    /// Print file statistics
    pub fn print_statistics(&self, files: &[FileInfo]) {
        if files.is_empty() {
            println!("No files found");
            return;
        }

        let total_size: u64 = files.iter().map(|f| f.size).sum();
        let binary_count = files.iter().filter(|f| f.is_binary).count();
        let text_count = files.len() - binary_count;

        // Extension statistics
        let mut ext_counts = std::collections::HashMap::new();
        for file in files {
            *ext_counts.entry(&file.extension).or_insert(0) += 1;
        }

        let mut ext_vec: Vec<_> = ext_counts.into_iter().collect();
        ext_vec.sort_by(|a, b| b.1.cmp(&a.1));

        println!("\n{}", "Statistics:".green().bold());
        println!("{}: {}", "Total files".cyan(), files.len());
        println!("{}: {}", "Text files".cyan(), text_count);
        println!("{}: {}", "Binary files".cyan(), binary_count);
        println!("{}: {}", "Total size".cyan(), self.format_size(total_size));

        if !ext_vec.is_empty() {
            println!("\n{}", "File types:".green().bold());
            for (ext, count) in ext_vec.iter().take(10) {
                let ext_display = if *ext == "none" { "no extension" } else { ext };
                println!("  {}: {}", format!(".{ext_display}").cyan(), count);
            }
        }
    }

    /// Copy file list to clipboard
    pub fn copy_to_clipboard(&self, files: &[FileInfo]) -> RfgrepResult<()> {
        let content = files
            .iter()
            .map(|f| f.path.display().to_string())
            .collect::<Vec<_>>()
            .join("\n");

        let can_use_clipboard =
            std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok();

        if can_use_clipboard {
            match arboard::Clipboard::new() {
                Ok(mut clipboard) => {
                    clipboard
                        .set_text(content)
                        .map_err(|e| RfgrepError::Other(format!("Clipboard error: {e}")))?;
                    println!("\n{}", "File list copied to clipboard!".green());
                }
                Err(e) => {
                    self.fallback_to_file(&content)?;
                    eprintln!("Clipboard init failed: {e}");
                }
            }
        } else {
            self.fallback_to_file(&content)?;
        }

        Ok(())
    }

    /// Fallback to writing to a temporary file
    fn fallback_to_file(&self, content: &str) -> RfgrepResult<()> {
        let tmp = std::env::temp_dir().join("rfgrep_files.txt");
        std::fs::write(&tmp, content).map_err(|e| RfgrepError::Io(e))?;
        println!("\n{} {}", "File list written to".green(), tmp.display());
        Ok(())
    }

    /// Format file size in human-readable format
    fn format_size(&self, bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        const THRESHOLD: f64 = 1024.0;

        if bytes == 0 {
            return "0 B".to_string();
        }

        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= THRESHOLD && unit_index < UNITS.len() - 1 {
            size /= THRESHOLD;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }
}
