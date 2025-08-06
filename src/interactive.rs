use crate::config::PerformanceConfig;
use crate::memory::AdaptiveMemoryManager;
use crate::search_algorithms::{
    SearchAlgorithm, SearchAlgorithmFactory, SearchAlgorithmTrait, SearchMatch,
};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use log::info;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Interactive search session
#[allow(dead_code)]
pub struct InteractiveSearch {
    memory_manager: AdaptiveMemoryManager,
    search_algorithm: Box<dyn SearchAlgorithmTrait>,
    results: Arc<Mutex<Vec<SearchMatch>>>,
    filtered_results: Arc<Mutex<Vec<SearchMatch>>>,
    current_query: String,
    file_paths: Vec<PathBuf>,
    context_lines: usize,
    is_running: Arc<Mutex<bool>>,
}

impl InteractiveSearch {
    #[allow(dead_code)]
    pub fn new(
        pattern: &str,
        algorithm: SearchAlgorithm,
        file_paths: Vec<PathBuf>,
        config: PerformanceConfig,
    ) -> Self {
        let memory_manager = AdaptiveMemoryManager::new(config);
        let search_algorithm = SearchAlgorithmFactory::create(algorithm, pattern);

        Self {
            memory_manager,
            search_algorithm,
            results: Arc::new(Mutex::new(Vec::new())),
            filtered_results: Arc::new(Mutex::new(Vec::new())),
            current_query: pattern.to_string(),
            file_paths,
            context_lines: 2,
            is_running: Arc::new(Mutex::new(true)),
        }
    }

    /// Start the interactive search session
    #[allow(dead_code)]
    pub fn run(&mut self) -> io::Result<()> {
        info!("Starting interactive search session");

        // Initial search
        self.perform_search()?;

        // Interactive loop
        while *self.is_running.lock().unwrap() {
            self.display_results()?;
            self.handle_user_input()?;
        }

        Ok(())
    }

    /// Perform search across all files
    #[allow(dead_code)]
    fn perform_search(&mut self) -> io::Result<()> {
        let pb = ProgressBar::new(self.file_paths.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} files",
                )
                .unwrap()
                .progress_chars("=>-"),
        );

        let mut all_results = Vec::new();

        for (i, path) in self.file_paths.iter().enumerate() {
            pb.set_position(i as u64);

            if let Ok(content) = std::fs::read_to_string(path) {
                let matches = self
                    .search_algorithm
                    .search_with_context(&content, self.context_lines);

                // Add file path to each match
                for mut m in matches {
                    m.line = format!("{}:{}", path.display(), m.line);
                    all_results.push(m);
                }
            }
        }

        pb.finish_with_message("Search completed");

        *self.results.lock().unwrap() = all_results;
        *self.filtered_results.lock().unwrap() = self.results.lock().unwrap().clone();

        Ok(())
    }

    /// Display current results
    #[allow(dead_code)]
    fn display_results(&self) -> io::Result<()> {
        let results = self.filtered_results.lock().unwrap();
        let total_results = results.len();

        println!("\n{}", "=".repeat(80).dimmed());
        println!(
            "{} {} results for '{}'",
            "Found".green(),
            total_results,
            self.current_query.yellow()
        );
        println!("{}", "=".repeat(80).dimmed());

        if total_results == 0 {
            println!("{}", "No matches found".yellow());
            return Ok(());
        }

        // Display first 20 results
        let display_count = total_results.min(20);
        for (i, m) in results.iter().take(display_count).enumerate() {
            self.display_match(i + 1, m)?;
        }

        if total_results > display_count {
            println!(
                "{}",
                format!("... and {} more results", total_results - display_count).dimmed()
            );
        }

        self.display_help()?;

        Ok(())
    }

    /// Display a single search match
    #[allow(dead_code)]
    fn display_match(&self, index: usize, m: &SearchMatch) -> io::Result<()> {
        println!("\n{}", format!("[{index}]").cyan());

        // Display context before
        for (num, line) in &m.context_before {
            println!("  {} │ {}", format!("{num}").dimmed(), line.dimmed());
        }

        // Display matching line with highlight
        let line_len = m.line.len();
        let column_start = m.column_start.min(line_len);
        let column_end = m.column_end.min(line_len);

        let before = if column_start < line_len {
            &m.line[..column_start]
        } else {
            ""
        };
        let matched = &m.matched_text;
        let after = if column_end < line_len {
            &m.line[column_end..]
        } else {
            ""
        };

        println!(
            "→ {} │ {}{}{}",
            m.line_number.to_string().yellow().bold(),
            before,
            matched.yellow().bold(),
            after
        );

        // Display context after
        for (num, line) in &m.context_after {
            println!("  {} │ {}", format!("{num}").dimmed(), line.dimmed());
        }

        Ok(())
    }

    /// Display interactive help
    #[allow(dead_code)]
    fn display_help(&self) -> io::Result<()> {
        println!("\n{}", "Interactive Commands:".green().bold());
        println!("  {} - New search", "n".yellow());
        println!("  {} - Filter results", "f".yellow());
        println!("  {} - Clear filters", "c".yellow());
        println!("  {} - Save results", "s".yellow());
        println!("  {} - Quit", "q".yellow());
        print!("{} ", "Command:".cyan());
        io::stdout().flush()?;

        Ok(())
    }

    /// Handle user input
    #[allow(dead_code)]
    fn handle_user_input(&mut self) -> io::Result<()> {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let command = input.trim().to_lowercase();

        match command.as_str() {
            "n" | "new" => self.new_search()?,
            "f" | "filter" => self.filter_results()?,
            "c" | "clear" => self.clear_filters()?,
            "s" | "save" => self.save_results()?,
            "q" | "quit" => self.quit()?,
            _ => println!("{}", "Unknown command. Type 'q' to quit.".red()),
        }

        Ok(())
    }

    /// Start a new search
    #[allow(dead_code)]
    fn new_search(&mut self) -> io::Result<()> {
        print!("{} ", "Enter new search pattern:".cyan());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let new_pattern = input.trim();
        if !new_pattern.is_empty() {
            self.current_query = new_pattern.to_string();
            self.search_algorithm =
                SearchAlgorithmFactory::create(SearchAlgorithm::BoyerMoore, &self.current_query);
            self.perform_search()?;
        }

        Ok(())
    }

    /// Filter current results
    #[allow(dead_code)]
    fn filter_results(&mut self) -> io::Result<()> {
        print!("{} ", "Enter filter (file path or content):".cyan());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let filter = input.trim().to_lowercase();
        if !filter.is_empty() {
            let results = self.results.lock().unwrap();
            let filtered: Vec<SearchMatch> = results
                .iter()
                .filter(|m| {
                    m.line.to_lowercase().contains(&filter)
                        || m.matched_text.to_lowercase().contains(&filter)
                })
                .cloned()
                .collect();

            *self.filtered_results.lock().unwrap() = filtered;
        }

        Ok(())
    }

    /// Clear all filters
    #[allow(dead_code)]
    fn clear_filters(&mut self) -> io::Result<()> {
        let results = self.results.lock().unwrap().clone();
        *self.filtered_results.lock().unwrap() = results;
        println!("{}", "Filters cleared".green());

        Ok(())
    }

    /// Save results to file
    #[allow(dead_code)]
    fn save_results(&self) -> io::Result<()> {
        let results = self.filtered_results.lock().unwrap();
        let filename = format!(
            "rfgrep_results_{}.txt",
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        );

        let mut file = std::fs::File::create(&filename)?;
        writeln!(file, "rfgrep Search Results")?;
        writeln!(file, "Query: {}", self.current_query)?;
        writeln!(file, "Results: {}", results.len())?;
        writeln!(
            file,
            "Time: {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        )?;
        writeln!(file, "{}", "=".repeat(80))?;

        for (i, m) in results.iter().enumerate() {
            writeln!(file, "\n[{}] {}", i + 1, m.line)?;
            writeln!(file, "Match: {}", m.matched_text)?;
        }

        println!("{}", format!("Results saved to {filename}").green());

        Ok(())
    }

    /// Quit interactive mode
    #[allow(dead_code)]
    fn quit(&mut self) -> io::Result<()> {
        println!("{}", "Exiting interactive mode".yellow());
        *self.is_running.lock().unwrap() = false;

        Ok(())
    }

    /// Get search statistics
    #[allow(dead_code)]
    pub fn get_stats(&self) -> SearchStats {
        let results = self.results.lock().unwrap();
        let filtered = self.filtered_results.lock().unwrap();

        SearchStats {
            total_results: results.len(),
            filtered_results: filtered.len(),
            query: self.current_query.clone(),
            files_searched: self.file_paths.len(),
            memory_usage: self.memory_manager.get_current_memory_usage(),
        }
    }
}

/// Search statistics for interactive mode
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SearchStats {
    pub total_results: usize,
    pub filtered_results: usize,
    pub query: String,
    pub files_searched: usize,
    pub memory_usage: u64,
}

/// Interactive search builder
#[allow(dead_code)]
pub struct InteractiveSearchBuilder {
    pattern: String,
    algorithm: SearchAlgorithm,
    file_paths: Vec<PathBuf>,
    config: PerformanceConfig,
}

impl InteractiveSearchBuilder {
    #[allow(dead_code)]
    pub fn new(pattern: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
            algorithm: SearchAlgorithm::BoyerMoore,
            file_paths: Vec::new(),
            config: PerformanceConfig::default(),
        }
    }

    #[allow(dead_code)]
    pub fn algorithm(mut self, algorithm: SearchAlgorithm) -> Self {
        self.algorithm = algorithm;
        self
    }

    #[allow(dead_code)]
    pub fn files(mut self, files: Vec<PathBuf>) -> Self {
        self.file_paths = files;
        self
    }

    #[allow(dead_code)]
    pub fn config(mut self, config: PerformanceConfig) -> Self {
        self.config = config;
        self
    }

    #[allow(dead_code)]
    pub fn build(self) -> InteractiveSearch {
        InteractiveSearch::new(&self.pattern, self.algorithm, self.file_paths, self.config)
    }
}
