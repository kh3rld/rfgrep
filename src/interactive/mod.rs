//! Interactive search engine with TUI interface
use crate::cli::InteractiveAlgorithm;
use crate::error::{Result as RfgrepResult, RfgrepError};
use crate::metrics::Metrics;
use crate::search::algorithms::*;
use crate::search::SearchEngine;
use colored::Colorize;
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;

/// Interactive search engine
pub struct InteractiveEngine {
    metrics: Arc<Metrics>,
    search_engine: SearchEngine,
    history: VecDeque<String>,
    max_history: usize,
}

/// Interactive search state
#[derive(Debug, Clone)]
pub struct SearchState {
    pub pattern: String,
    pub files: Vec<std::path::PathBuf>,
    pub matches: Vec<crate::processor::SearchMatch>,
    pub current_file_index: usize,
    pub current_match_index: usize,
    pub filter: Option<String>,
}

impl InteractiveEngine {
    /// Create a new interactive engine
    pub fn new(metrics: Arc<Metrics>) -> RfgrepResult<Self> {
        let search_engine = SearchEngine::new(metrics.clone())?;
        Ok(Self {
            metrics,
            search_engine,
            history: VecDeque::new(),
            max_history: 100,
        })
    }

    /// Run interactive search
    pub async fn run(
        &mut self,
        root_path: &Path,
        initial_pattern: &str,
        algorithm: InteractiveAlgorithm,
        recursive: bool,
        extensions: Option<&[String]>,
    ) -> RfgrepResult<()> {
        println!("{}", "Starting interactive search mode...".green().bold());
        println!("Pattern: {}", initial_pattern.yellow());
        println!("Algorithm: {algorithm:?}");
        println!("{}", "Press 'q' to quit, 'h' for help".dimmed());

        // Discover files
        let files = self
            .discover_files(root_path, recursive, extensions)
            .await?;
        println!("Files to search: {}", files.len());

        // Create search state
        let mut state = SearchState {
            pattern: initial_pattern.to_string(),
            files,
            matches: Vec::new(),
            current_file_index: 0,
            current_match_index: 0,
            filter: None,
        };

        // Perform initial search
        self.perform_search(&mut state, &algorithm).await?;

        // Start interactive loop
        self.interactive_loop(&mut state, &algorithm).await?;

        Ok(())
    }

    /// Discover files to search
    async fn discover_files(
        &self,
        root_path: &Path,
        recursive: bool,
        extensions: Option<&[String]>,
    ) -> RfgrepResult<Vec<std::path::PathBuf>> {
        use crate::walker::walk_dir;

        let files: Vec<_> = walk_dir(root_path, recursive, true)
            .filter(|entry| entry.path().is_file())
            .map(|entry| entry.path().to_path_buf())
            .collect();

        // Apply extension filter
        let filtered_files: Vec<_> = if let Some(extensions) = extensions {
            files
                .into_iter()
                .filter(|path| {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        extensions.iter().any(|e| e.eq_ignore_ascii_case(ext))
                    } else {
                        false
                    }
                })
                .collect()
        } else {
            files
        };

        Ok(filtered_files)
    }

    /// Perform search with current state
    async fn perform_search(
        &self,
        state: &mut SearchState,
        algorithm: &InteractiveAlgorithm,
    ) -> RfgrepResult<()> {
        if state.pattern.is_empty() {
            state.matches.clear();
            return Ok(());
        }

        // Create search algorithm
        let search_algorithm: Box<dyn SearchAlgorithmTrait> = match algorithm {
            InteractiveAlgorithm::BoyerMoore => Box::new(SimdSearch::new(&state.pattern)),
            InteractiveAlgorithm::Regex => Box::new(RegexSearch::new(&state.pattern)?),
            InteractiveAlgorithm::Simple => Box::new(SimdSearch::new(&state.pattern)),
        };

        // Search all files
        let mut all_matches = Vec::new();
        for file in &state.files {
            if let Ok(content) = std::fs::read_to_string(file) {
                let file_matches =
                    search_algorithm.search_with_context(&content, &state.pattern, 2);
                for mut m in file_matches {
                    m.path = file.clone();
                    all_matches.push(m);
                }
            }
        }

        // Apply filter if set
        if let Some(filter) = &state.filter {
            state.matches = all_matches
                .into_iter()
                .filter(|m| m.line.to_lowercase().contains(&filter.to_lowercase()))
                .collect();
        } else {
            state.matches = all_matches;
        }

        // Sort matches
        state.matches.sort();

        // Update metrics
        self.metrics
            .matches_found
            .inc_by(state.matches.len() as u64);

        Ok(())
    }

    /// Main interactive loop
    async fn interactive_loop(
        &mut self,
        state: &mut SearchState,
        algorithm: &InteractiveAlgorithm,
    ) -> RfgrepResult<()> {
        use std::io::{self, Write};

        loop {
            // Display current state
            self.display_state(state);

            // Get user input
            print!("\n> ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.is_empty() {
                continue;
            }

            // Add to history
            self.add_to_history(input.to_string());

            // Process command
            match self.process_command(input, state, algorithm).await? {
                CommandResult::Continue => continue,
                CommandResult::Quit => break,
                CommandResult::Search => {
                    self.perform_search(state, algorithm).await?;
                }
            }
        }

        Ok(())
    }

    /// Display current search state
    fn display_state(&self, state: &SearchState) {
        println!("\n{}", "=".repeat(60).cyan());
        println!("Pattern: {}", state.pattern.yellow());
        println!("Files: {}", state.files.len());
        println!("Matches: {}", state.matches.len());

        if let Some(filter) = &state.filter {
            println!("Filter: {}", filter.blue());
        }

        if !state.matches.is_empty() {
            println!("\n{}", "Recent matches:".green().bold());
            let start = state.current_match_index.saturating_sub(5);
            let end = (state.current_match_index + 5).min(state.matches.len());

            for (i, m) in state.matches[start..end].iter().enumerate() {
                let actual_index = start + i;
                let marker = if actual_index == state.current_match_index {
                    "→"
                } else {
                    " "
                };
                println!(
                    "{} {}:{}:{}: {}",
                    marker.yellow(),
                    m.path.display(),
                    m.line_number,
                    m.column_start + 1,
                    m.line
                );
            }
        }
    }

    /// Process user command
    async fn process_command(
        &self,
        input: &str,
        state: &mut SearchState,
        _algorithm: &InteractiveAlgorithm,
    ) -> RfgrepResult<CommandResult> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(CommandResult::Continue);
        }

        match parts[0] {
            "q" | "quit" | "exit" => Ok(CommandResult::Quit),
            "h" | "help" => {
                self.show_help();
                Ok(CommandResult::Continue)
            }
            "s" | "search" => {
                if parts.len() > 1 {
                    state.pattern = parts[1..].join(" ");
                    Ok(CommandResult::Search)
                } else {
                    println!("Usage: search <pattern>");
                    Ok(CommandResult::Continue)
                }
            }
            "f" | "filter" => {
                if parts.len() > 1 {
                    state.filter = Some(parts[1..].join(" "));
                    Ok(CommandResult::Search)
                } else {
                    state.filter = None;
                    Ok(CommandResult::Search)
                }
            }
            "c" | "clear" => {
                state.filter = None;
                state.pattern.clear();
                Ok(CommandResult::Search)
            }
            "n" | "next" => {
                if state.current_match_index < state.matches.len().saturating_sub(1) {
                    state.current_match_index += 1;
                }
                Ok(CommandResult::Continue)
            }
            "p" | "prev" | "previous" => {
                if state.current_match_index > 0 {
                    state.current_match_index -= 1;
                }
                Ok(CommandResult::Continue)
            }
            "g" | "goto" => {
                if parts.len() > 1 {
                    if let Ok(index) = parts[1].parse::<usize>() {
                        if index < state.matches.len() {
                            state.current_match_index = index;
                        }
                    }
                }
                Ok(CommandResult::Continue)
            }
            "o" | "open" => {
                if !state.matches.is_empty() {
                    let current_match = &state.matches[state.current_match_index];
                    self.open_file(current_match)?;
                }
                Ok(CommandResult::Continue)
            }
            "save" => {
                if parts.len() > 1 {
                    self.save_results(&state.matches, parts[1])?;
                } else {
                    self.save_results(&state.matches, "results.txt")?;
                }
                Ok(CommandResult::Continue)
            }
            "stats" => {
                self.show_statistics(state);
                Ok(CommandResult::Continue)
            }
            "history" => {
                self.show_history();
                Ok(CommandResult::Continue)
            }
            _ => {
                // Treat as search pattern
                state.pattern = input.to_string();
                Ok(CommandResult::Search)
            }
        }
    }

    /// Show help information
    fn show_help(&self) {
        println!("\n{}", "Available commands:".green().bold());
        println!("  {} - Search for pattern", "search <pattern>".yellow());
        println!("  {} - Filter current results", "filter <text>".yellow());
        println!("  {} - Clear filters", "clear".yellow());
        println!("  {} - Next match", "next".yellow());
        println!("  {} - Previous match", "prev".yellow());
        println!("  {} - Go to match number", "goto <number>".yellow());
        println!("  {} - Open current match in editor", "open".yellow());
        println!("  {} - Save results to file", "save [filename]".yellow());
        println!("  {} - Show statistics", "stats".yellow());
        println!("  {} - Show command history", "history".yellow());
        println!("  {} - Show this help", "help".yellow());
        println!("  {} - Quit", "quit".yellow());
    }

    /// Open file in editor
    fn open_file(&self, match_: &crate::processor::SearchMatch) -> RfgrepResult<()> {
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
        let status = std::process::Command::new(&editor)
            .arg(&match_.path)
            .status()?;

        if !status.success() {
            eprintln!("Failed to open file in editor: {}", editor);
        }

        Ok(())
    }

    /// Save results to file
    fn save_results(
        &self,
        matches: &[crate::processor::SearchMatch],
        filename: &str,
    ) -> RfgrepResult<()> {
        let mut content = String::new();
        for m in matches {
            content.push_str(&format!(
                "{}:{}:{}: {}\n",
                m.path.display(),
                m.line_number,
                m.column_start + 1,
                m.line
            ));
        }

        std::fs::write(filename, content)?;
        println!("Results saved to {}", filename.green());

        Ok(())
    }

    /// Show search statistics
    fn show_statistics(&self, state: &SearchState) {
        println!("\n{}", "Search Statistics:".green().bold());
        println!("Pattern: {}", state.pattern.yellow());
        println!("Files searched: {}", state.files.len());
        println!("Total matches: {}", state.matches.len());

        if !state.matches.is_empty() {
            let files_with_matches: std::collections::HashSet<_> =
                state.matches.iter().map(|m| &m.path).collect();
            println!("Files with matches: {}", files_with_matches.len());

            // Extension statistics
            let mut ext_counts = std::collections::HashMap::new();
            for m in &state.matches {
                if let Some(ext) = m.path.extension().and_then(|e| e.to_str()) {
                    *ext_counts.entry(ext).or_insert(0) += 1;
                }
            }

            let mut ext_vec: Vec<_> = ext_counts.into_iter().collect();
            ext_vec.sort_by(|a, b| b.1.cmp(&a.1));

            if !ext_vec.is_empty() {
                println!("\nMatches by file type:");
                for (ext, count) in ext_vec.iter().take(5) {
                    println!("  .{}: {}", ext, count);
                }
            }
        }
    }

    /// Add command to history
    fn add_to_history(&mut self, command: String) {
        if self.history.len() >= self.max_history {
            self.history.pop_front();
        }
        self.history.push_back(command);
    }

    /// Show command history
    fn show_history(&self) {
        println!("\n{}", "Command History:".green().bold());
        for (i, cmd) in self.history.iter().enumerate() {
            println!("  {}: {}", i + 1, cmd);
        }
    }
}

/// Command processing result
#[derive(Debug)]
enum CommandResult {
    Continue,
    Quit,
    Search,
}

/// Regex search implementation for interactive mode
pub struct RegexSearch {
    pattern: String,
    regex: regex::Regex,
}

impl RegexSearch {
    pub fn new(pattern: &str) -> RfgrepResult<Self> {
        let regex = regex::Regex::new(pattern)
            .map_err(|e| RfgrepError::Other(format!("Invalid regex: {e}")))?;
        Ok(Self {
            pattern: pattern.to_string(),
            regex,
        })
    }
}

impl SearchAlgorithmTrait for RegexSearch {
    fn search(&self, text: &str, _pattern: &str) -> Vec<usize> {
        self.regex.find_iter(text).map(|m| m.start()).collect()
    }

    fn search_with_context(
        &self,
        text: &str,
        _pattern: &str,
        context_lines: usize,
    ) -> Vec<crate::processor::SearchMatch> {
        let matches = self.search(text, _pattern);
        let lines: Vec<&str> = text.lines().collect();
        let mut results = Vec::new();

        for &match_pos in &matches {
            let pre_lines = text[..match_pos].lines().count();
            let line_number = pre_lines.max(1);
            let line_index = line_number - 1;

            if line_index < lines.len() {
                let line = lines[line_index];
                let context_before = self.get_context_before(&lines, line_index, context_lines);
                let context_after = self.get_context_after(&lines, line_index, context_lines);

                let matched_text = self
                    .regex
                    .find(&text[match_pos..])
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();

                results.push(crate::processor::SearchMatch {
                    path: std::path::PathBuf::new(),
                    line_number,
                    line: line.to_string(),
                    context_before,
                    context_after,
                    matched_text: matched_text.clone(),
                    column_start: match_pos - text[..match_pos].rfind('\n').unwrap_or(0),
                    column_end: match_pos - text[..match_pos].rfind('\n').unwrap_or(0)
                        + matched_text.len(),
                });
            }
        }

        results
    }

    fn name(&self) -> &'static str {
        "Regex"
    }
}

impl RegexSearch {
    fn get_context_before(
        &self,
        lines: &[&str],
        current_line: usize,
        context_lines: usize,
    ) -> Vec<(usize, String)> {
        let start = current_line.saturating_sub(context_lines);
        (start..current_line)
            .map(|i| (i + 1, lines[i].to_string()))
            .collect()
    }

    fn get_context_after(
        &self,
        lines: &[&str],
        current_line: usize,
        context_lines: usize,
    ) -> Vec<(usize, String)> {
        let end = (current_line + context_lines + 1).min(lines.len());
        ((current_line + 1)..end)
            .map(|i| (i + 1, lines[i].to_string()))
            .collect()
    }
}
