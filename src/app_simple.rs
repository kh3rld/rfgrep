//! Simplified application structure
use crate::cli::{
    Cli, Commands, PluginCommands, SearchAlgorithm as CliSearchAlgorithm, SearchMode,
};
use crate::error::{Result as RfgrepResult, RfgrepError};
use crate::output_formats::OutputFormatter;
use crate::plugin_cli::PluginCli;
use crate::plugin_system::{EnhancedPluginManager, PluginRegistry};
use crate::processor::search_file;
use crate::search_algorithms::SearchAlgorithm;
use crate::streaming_search::{StreamingConfig, StreamingSearchPipeline};
use crate::tui::{init_terminal, restore_terminal, TuiApp};
use crate::walker::walk_dir;
use colored::Colorize;
use std::path::Path;
use std::sync::Arc;

/// Simplified application that uses existing components
pub struct RfgrepApp {
    plugin_manager: Arc<EnhancedPluginManager>,
}

impl RfgrepApp {
    /// Create a new application instance
    pub fn new() -> RfgrepResult<Self> {
        let plugin_manager = Arc::new(EnhancedPluginManager::new());
        let registry = PluginRegistry::new(plugin_manager.clone());

        // Initialize plugins synchronously for now
        // TODO: Make this async when we have proper async runtime setup
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            crate::error::RfgrepError::Other(format!("Failed to create runtime: {}", e))
        })?;
        rt.block_on(async { registry.load_plugins().await })?;

        Ok(Self { plugin_manager })
    }

    /// Run the application with the given CLI arguments
    pub async fn run(&self, cli: Cli) -> RfgrepResult<()> {
        // Handle logging if specified
        if let Some(log_path) = &cli.log {
            // Create log file
            std::fs::write(log_path, "rfgrep log file created\n").map_err(RfgrepError::Io)?;
        }

        match &cli.command {
            Commands::Search {
                pattern,
                mode,
                algorithm,
                recursive,
                context_lines,
                case_sensitive,
                invert_match,
                max_matches,
                timeout_per_file,
                path: cmd_path,
                path_flag: cmd_path_flag,
                ..
            } => {
                self.handle_search(
                    pattern,
                    mode.clone(),
                    algorithm.clone(),
                    *recursive,
                    *context_lines,
                    *case_sensitive,
                    *invert_match,
                    *max_matches,
                    *timeout_per_file,
                    cmd_path
                        .as_ref()
                        .or(cmd_path_flag.as_ref())
                        .map(|p| p.as_path())
                        .unwrap_or(&cli.path),
                    cli.max_size,
                    cli.skip_binary,
                )
                .await
            }
            Commands::List {
                extensions,
                long,
                recursive,
                show_hidden,
                max_size,
                min_size,
                detailed,
                simple,
                stats,
                sort,
                reverse,
                limit,
                copy,
                output_format,
                path: cmd_path,
                path_flag: cmd_path_flag,
            } => {
                self.handle_list(
                    extensions.as_deref(),
                    *long,
                    *recursive,
                    *show_hidden,
                    *max_size,
                    *min_size,
                    *detailed,
                    *simple,
                    *stats,
                    sort.clone(),
                    *reverse,
                    *limit,
                    *copy,
                    output_format.clone(),
                    cmd_path.as_ref().map(|p| p.as_path()),
                    cmd_path_flag.as_ref().map(|p| p.as_path()),
                    &cli.path,
                ).await
            }
            Commands::Interactive { .. } => {
                println!("Interactive command not yet implemented in simplified version");
                Ok(())
            }
            Commands::Completions { shell } => self.handle_completions(*shell),
            Commands::Worker { path, pattern } => self.handle_worker(path, pattern).await,
            Commands::Plugins { command } => self.handle_plugin_command(command).await,
            Commands::Tui {
                pattern,
                algorithm,
                case_sensitive,
                mode,
                context_lines,
                path,
            } => {
                self.handle_tui_command(
                    pattern.as_deref(),
                    algorithm,
                    *case_sensitive,
                    mode,
                    *context_lines,
                    path,
                )
                .await
            }
        }
    }

    async fn handle_search(
        &self,
        pattern: &str,
        mode: crate::cli::SearchMode,
        algorithm: CliSearchAlgorithm,
        recursive: bool,
        context_lines: usize,
        case_sensitive: bool,
        invert_match: bool,
        max_matches: Option<usize>,
        timeout_per_file: Option<u64>,
        search_path: &Path,
        max_size: Option<usize>,
        skip_binary: bool,
    ) -> RfgrepResult<()> {
        // Note: Root check would need to be passed as parameter

        // Convert CLI algorithm to internal algorithm
        let search_algorithm = match algorithm {
            CliSearchAlgorithm::BoyerMoore => SearchAlgorithm::BoyerMoore,
            CliSearchAlgorithm::Regex => SearchAlgorithm::Regex,
            CliSearchAlgorithm::Simple => SearchAlgorithm::Simple,
        };

        // Build search pattern based on mode
        let search_pattern = match mode {
            crate::cli::SearchMode::Text => pattern.to_string(),
            crate::cli::SearchMode::Word => format!(r"\b{}\b", regex::escape(pattern)),
            crate::cli::SearchMode::Regex => pattern.to_string(),
        };

        // Discover files to search
        let entries: Vec<_> = walk_dir(search_path, recursive, true).collect();
        let files: Vec<_> = entries
            .into_iter()
            .filter(|entry| entry.path().is_file())
            .map(|entry| entry.path().to_path_buf())
            .collect();

        // Filter files
        let filtered_files: Vec<_> = files
            .into_iter()
            .filter(|path| {
                // Size filter
                if let Some(max_size) = max_size {
                    if let Ok(metadata) = path.metadata() {
                        let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
                        if size_mb > max_size as f64 {
                            return false;
                        }
                    }
                }

                // Binary filter
                if skip_binary && crate::processor::is_binary(path) {
                    return false;
                }

                true
            })
            .collect();

        println!("Searching {} files...", filtered_files.len());

        // Create streaming search configuration
        let config = StreamingConfig {
            algorithm: search_algorithm,
            context_lines,
            case_sensitive,
            invert_match,
            max_matches,
            timeout_per_file,
            chunk_size: 8192,
            buffer_size: 65536,
        };

        let pipeline = StreamingSearchPipeline::new(config);

        // Convert file paths to references for parallel search
        let file_refs: Vec<&Path> = filtered_files.iter().map(|p| p.as_path()).collect();

        // Use streaming search pipeline for better performance
        let all_matches = if file_refs.len() > 10 {
            // Use parallel search for many files
            pipeline
                .search_files_parallel(&file_refs, &search_pattern, 4)
                .await?
        } else {
            // Use sequential search for few files
            let mut all_matches = Vec::new();
            for file in &filtered_files {
                match pipeline.search_file(file, &search_pattern).await {
                    Ok(matches) => all_matches.extend(matches),
                    Err(e) => {
                        eprintln!("Error searching {}: {}", file.display(), e);
                    }
                }
            }
            all_matches
        };

        // Display results
        if all_matches.is_empty() {
            println!("{}", "No matches found".yellow());
        } else {
            println!(
                "\n{} {} {}",
                "Found".green(),
                all_matches.len(),
                "matches:".green()
            );

            let formatter = OutputFormatter::new(crate::output_formats::OutputFormat::Text);
            let output = formatter.format_results(&all_matches, pattern, search_path);
            println!("\n{output}");
        }

        Ok(())
    }

    fn handle_completions(&self, shell: clap_complete::Shell) -> RfgrepResult<()> {
        use clap::CommandFactory;
        let mut cmd = Cli::command();
        clap_complete::generate(shell, &mut cmd, "rfgrep", &mut std::io::stdout());
        Ok(())
    }

    async fn handle_worker(&self, path: &std::path::Path, pattern: &str) -> RfgrepResult<()> {
        // Worker mode: perform a search on a single file and print NDJSON lines to stdout
        if let Ok(s) = std::env::var("RFGREP_WORKER_SLEEP") {
            if let Ok(sec) = s.parse::<u64>() {
                std::thread::sleep(std::time::Duration::from_secs(sec));
            }
        }

        let regex = crate::processor::get_or_compile_regex(pattern)?;
        let matches = search_file(path, &regex)?;

        for m in matches {
            if let Ok(json) = serde_json::to_string(&m) {
                println!("{json}");
            }
        }

        Ok(())
    }

    async fn handle_plugin_command(&self, command: &PluginCommands) -> RfgrepResult<()> {
        let plugin_cli = PluginCli::new(self.plugin_manager.clone());

        match command {
            PluginCommands::List => plugin_cli.list_plugins().await,
            PluginCommands::Stats => plugin_cli.show_stats().await,
            PluginCommands::Info { name } => plugin_cli.show_plugin_info(name).await,
            PluginCommands::Enable { name } => plugin_cli.enable_plugin(name).await,
            PluginCommands::Disable { name } => plugin_cli.disable_plugin(name).await,
            PluginCommands::Priority { name, priority } => {
                plugin_cli.set_priority(name, *priority).await
            }
            PluginCommands::Config { name } => plugin_cli.show_config_options(name).await,
            PluginCommands::Test {
                name,
                file,
                pattern,
            } => plugin_cli.test_plugin(name, file, pattern).await,
        }
    }

    async fn handle_tui_command(
        &self,
        pattern: Option<&str>,
        algorithm: &CliSearchAlgorithm,
        case_sensitive: bool,
        mode: &SearchMode,
        context_lines: usize,
        _path: &str,
    ) -> RfgrepResult<()> {
        // Initialize TUI
        let mut terminal = init_terminal()?;
        let mut app = TuiApp::new()?;

        // Set initial pattern if provided
        if let Some(p) = pattern {
            app.set_pattern(p.to_string());
        }

        // Convert CLI types to TUI types
        let tui_algorithm = match algorithm {
            CliSearchAlgorithm::BoyerMoore => SearchAlgorithm::BoyerMoore,
            CliSearchAlgorithm::Regex => SearchAlgorithm::Regex,
            CliSearchAlgorithm::Simple => SearchAlgorithm::Simple,
        };

        let tui_mode = match mode {
            SearchMode::Text => crate::tui::SearchMode::Text,
            SearchMode::Word => crate::tui::SearchMode::Word,
            SearchMode::Regex => crate::tui::SearchMode::Regex,
        };

        // Set TUI state
        app.state.algorithm = tui_algorithm;
        app.state.case_sensitive = case_sensitive;
        app.state.context_lines = context_lines;
        app.state.search_mode = tui_mode;

        // If pattern is provided, perform initial search using plugin manager
        if let Some(p) = pattern {
            app.state.status_message = format!("Searching for: {}", p);
            use std::path::Path;
            let mut all_matches = Vec::new();
            let search_root = std::path::PathBuf::from(_path);
            let search_root = if search_root.as_os_str().is_empty() { std::path::PathBuf::from(".") } else { search_root };
            let entries: Vec<_> = walk_dir(&search_root, true, false).collect();
            for entry in entries {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(mut matches) = self.plugin_manager.search_file(path, p).await {
                        all_matches.append(&mut matches);
                    }
                }
            }
            app.set_matches(all_matches);
        }

        // Run TUI
        let result = app.run(&mut terminal).await;

        // Restore terminal
        restore_terminal(&mut terminal)?;

        result
    }

    async fn handle_list(
        &self,
        extensions: Option<&[String]>,
        long: bool,
        recursive: bool,
        show_hidden: bool,
        max_size: Option<usize>,
        min_size: Option<usize>,
        detailed: bool,
        simple: bool,
        stats: bool,
        sort: crate::cli::SortCriteria,
        reverse: bool,
        limit: Option<usize>,
        copy: bool,
        output_format: crate::cli::OutputFormat,
        cmd_path: Option<&Path>,
        cmd_path_flag: Option<&Path>,
        default_path: &Path,
    ) -> RfgrepResult<()> {
        let search_path = cmd_path_flag
            .or(cmd_path)
            .unwrap_or(default_path);

        // Discover files
        let entries: Vec<_> = walk_dir(search_path, recursive, show_hidden).collect();
        let mut files: Vec<_> = entries
            .into_iter()
            .filter(|entry| entry.path().is_file())
            .map(|entry| entry.path().to_path_buf())
            .collect();

        // Apply filters
        files.retain(|path| {
            // Extension filter
            if let Some(exts) = extensions {
                if let Some(ext) = path.extension() {
                    if let Some(ext_str) = ext.to_str() {
                        if !exts.iter().any(|e| e == ext_str) {
                            return false;
                        }
                    }
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
        });

        // Sort files
        match sort {
            crate::cli::SortCriteria::Name => files.sort_by(|a, b| a.file_name().cmp(&b.file_name())),
            crate::cli::SortCriteria::Size => {
                files.sort_by(|a, b| {
                    let size_a = a.metadata().map(|m| m.len()).unwrap_or(0);
                    let size_b = b.metadata().map(|m| m.len()).unwrap_or(0);
                    size_a.cmp(&size_b)
                });
            }
            crate::cli::SortCriteria::Date => {
                files.sort_by(|a, b| {
                    let time_a = a.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
                    let time_b = b.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
                    time_a.cmp(&time_b)
                });
            }
            crate::cli::SortCriteria::Type => {
                files.sort_by(|a, b| {
                    let ext_a = a.extension().and_then(|e| e.to_str()).unwrap_or("");
                    let ext_b = b.extension().and_then(|e| e.to_str()).unwrap_or("");
                    ext_a.cmp(ext_b)
                });
            }
            crate::cli::SortCriteria::Path => {
                files.sort_by(|a, b| a.cmp(b));
            }
        }

        if reverse {
            files.reverse();
        }

        // Apply limit
        if let Some(limit) = limit {
            files.truncate(limit);
        }

        // Output files
        if stats {
            println!("Summary: {} files found", files.len());
        } else if simple {
            for file in &files {
                println!("{}", file.display());
            }
        } else {
            for file in &files {
                if long {
                    if let Ok(metadata) = file.metadata() {
                        let size = metadata.len();
                        let modified = metadata.modified().unwrap_or(std::time::UNIX_EPOCH);
                        println!("{} {} {}", size, modified.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(), file.display());
                    } else {
                        println!("{}", file.display());
                    }
                } else {
                    println!("{}", file.display());
                }
            }
            // Always output summary for basic list command
            println!("Summary: {} files found", files.len());
            
            // If long format, also output extension summary
            if long {
                let mut extensions: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
                for file in &files {
                    if let Some(ext) = file.extension() {
                        if let Some(ext_str) = ext.to_str() {
                            *extensions.entry(ext_str.to_string()).or_insert(0) += 1;
                        }
                    }
                }
                if !extensions.is_empty() {
                    println!("Extensions:");
                    let mut ext_vec: Vec<_> = extensions.iter().collect();
                    ext_vec.sort_by(|a, b| a.0.cmp(b.0));
                    for (ext, count) in ext_vec {
                        println!("  .{}: {} files", ext, count);
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for RfgrepApp {
    fn default() -> Self {
        Self::new().expect("Failed to create RfgrepApp")
    }
}
