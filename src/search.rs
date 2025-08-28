use crate::PathBuf;
use crate::config::Config;
use crate::error::{Result as RfgrepResult, RfgrepError};
use crate::processor::search_file;
use crate::progress::ProgressReporter;
use log::debug;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use regex::Regex;
use std::{path::Path, sync::Arc};
use tokio::sync::mpsc;

#[allow(dead_code)]
pub struct SearchExecutor {
    config: Arc<Config>,
    progress: Arc<ProgressReporter>,
    pattern: Arc<Regex>,
}

impl SearchExecutor {
    #[allow(dead_code)]
    pub fn new(
        config: Config,
        progress: ProgressReporter,
        pattern_str: &str,
    ) -> RfgrepResult<Self> {
        let pattern = crate::processor::get_or_compile_regex(pattern_str)?;

        Ok(Self {
            config: Arc::new(config),
            progress: Arc::new(progress),
            pattern: Arc::new(pattern),
        })
    }

    #[allow(dead_code)]
    pub async fn execute(
        &self,
        root_path: &Path,
    ) -> RfgrepResult<Vec<crate::processor::SearchMatch>> {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let config = self.config.clone();
        let pattern = self.pattern.clone();
        let progress = self.progress.clone();

        let initial_files: Vec<PathBuf> = crate::walker::walk_dir(root_path, true, false)
            .filter(|entry| entry.path().is_file())
            .filter_map(|entry| {
                let path = entry.path();
                let file_name = path.display().to_string();
                if config.ignore.hidden_files && file_name.starts_with('.') {
                    debug!("Skipping hidden file: {file_name}");
                    return None;
                }
                if config.ignore.patterns.iter().any(|p| file_name.contains(p)) {
                    debug!("Skipping file matching ignore pattern: {file_name}");
                    return None;
                }

                // Replace let-chain with nested ifs for metadata check
                if let Some(max_size) = config.search.max_file_size
                    && let Ok(metadata) = path.metadata()
                {
                    let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
                    if size_mb > max_size as f64 {
                        debug!("Skipping large file ({}MB): {}", size_mb.round(), file_name);
                        return None;
                    }
                }

                if config.ignore.binary_files && crate::processor::is_binary(path) {
                    debug!("Skipping binary file: {file_name}");
                    return None;
                }
                if !config.search.default_extensions.is_empty() {
                    if let Some(file_ext) = path.extension().and_then(|e| e.to_str()) {
                        if !config
                            .search
                            .default_extensions
                            .iter()
                            .any(|e| e.eq_ignore_ascii_case(file_ext))
                        {
                            debug!("Skipping file with non-matching extension: {file_name}");
                            return None;
                        }
                    } else {
                        debug!(
                            "Skipping file with no extension (default_extensions set): {file_name}"
                        );
                        return None;
                    }
                }

                Some(path.to_path_buf())
            })
            .collect();

        let total_files = initial_files.len();
        self.progress.main_progress.set_length(total_files as u64);
        self.progress
            .main_progress
            .set_message(format!("Processing {total_files} files..."));
        let _tx_clone = tx.clone();
        let pattern = pattern.clone();
        let progress = progress.clone();
        let producer_handle = tokio::task::spawn_blocking(move || {
            initial_files.into_par_iter().for_each(|path| {
                if tx.is_closed() {
                    debug!(
                        "Receiver dropped, stopping file processing for {}",
                        path.display()
                    );
                    return;
                }

                match search_file(&path, &pattern) {
                    Ok(matches) => {
                        let _ = tx.send(matches);
                        progress.update(1, 0, 0);
                    }
                    Err(e) => eprintln!("Error processing file {}: {}", path.display(), e),
                };
            });
            Ok::<(), RfgrepError>(())
        });

        // Collect results
        let mut all_matches: Vec<crate::processor::SearchMatch> = Vec::new();
        while let Some(file_results) = rx.recv().await {
            all_matches.extend(file_results);
        }

        producer_handle
            .await
            .map_err(|e| RfgrepError::Other(format!("File processing task failed: {e}")))??;

        Ok(all_matches)
    }

    #[allow(dead_code)]
    pub fn get_progress_reporter(&self) -> Arc<ProgressReporter> {
        self.progress.clone()
    }

    #[allow(dead_code)]
    pub fn get_config(&self) -> Arc<Config> {
        self.config.clone()
    }

    #[allow(dead_code)]
    pub fn get_pattern(&self) -> Arc<Regex> {
        self.pattern.clone()
    }
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct SearchStats {
    pub files_searched: usize,
    pub bytes_searched: u64,
    pub matches_found: usize,
    pub time_taken: std::time::Duration,
}
