//! Advanced search engine with streaming and plugin support
pub mod algorithms;
pub mod cache;
pub mod engine;
pub mod plugins;
pub mod streaming;

use crate::cli::{SearchAlgorithm, SearchMode};
use crate::processor::SearchMatch;

pub use engine::SearchEngine;

/// Search configuration
#[derive(Debug, Clone)]
pub struct SearchConfig {
    pub mode: SearchMode,
    pub algorithm: SearchAlgorithm,
    pub recursive: bool,
    pub extensions: Option<Vec<String>>,
    pub context_lines: usize,
    pub case_sensitive: bool,
    pub invert_match: bool,
    pub max_matches: Option<usize>,
    pub timeout_per_file: Option<u64>,
    pub max_file_size: Option<usize>,
    pub skip_binary: bool,
    pub dry_run: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            mode: SearchMode::Text,
            algorithm: SearchAlgorithm::BoyerMoore,
            recursive: true,
            extensions: None,
            context_lines: 0,
            case_sensitive: false,
            invert_match: false,
            max_matches: None,
            timeout_per_file: None,
            max_file_size: None,
            skip_binary: false,
            dry_run: false,
        }
    }
}

/// Search result with metadata
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub matches: Vec<SearchMatch>,
    pub files_searched: usize,
    pub files_skipped: usize,
    pub total_bytes: u64,
    pub duration: std::time::Duration,
}
