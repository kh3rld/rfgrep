//! Main application structure and command handling
use crate::cli::{Cli, Commands};
use crate::error::{Result as RfgrepResult, RfgrepError};
use crate::search::SearchEngine;
use crate::list::ListEngine;
use crate::interactive::InteractiveEngine;
use crate::output::OutputManager;
use crate::metrics::Metrics;
use std::sync::Arc;

/// Main application struct that coordinates all components
pub struct RfgrepApp {
    search_engine: SearchEngine,
    list_engine: ListEngine,
    interactive_engine: InteractiveEngine,
    output_manager: OutputManager,
    metrics: Arc<Metrics>,
}

impl RfgrepApp {
    /// Create a new application instance
    pub fn new() -> RfgrepResult<Self> {
        let metrics = Arc::new(Metrics::new());
        let search_engine = SearchEngine::new(metrics.clone())?;
        let list_engine = ListEngine::new(metrics.clone())?;
        let interactive_engine = InteractiveEngine::new(metrics.clone())?;
        let output_manager = OutputManager::new()?;

        Ok(Self {
            search_engine,
            list_engine,
            interactive_engine,
            output_manager,
            metrics,
        })
    }

    /// Run the application with the given CLI arguments
    pub async fn run(&self, cli: Cli) -> RfgrepResult<()> {
        match &cli.command {
            Commands::Search { .. } => {
                self.handle_search(cli).await
            }
            Commands::List { .. } => {
                self.handle_list(cli).await
            }
            Commands::Interactive { .. } => {
                self.handle_interactive(cli).await
            }
            Commands::Completions { shell } => {
                self.handle_completions(*shell)
            }
            Commands::Worker { path, pattern } => {
                self.handle_worker(path, pattern).await
            }
        }
    }

    async fn handle_search(&self, cli: Cli) -> RfgrepResult<()> {
        if let Commands::Search {
            pattern,
            mode,
            copy,
            output_format,
            ndjson,
            extensions,
            recursive,
            context_lines,
            case_sensitive,
            invert_match,
            max_matches,
            algorithm,
            path: cmd_path,
            path_flag: cmd_path_flag,
            timeout_per_file,
        } = cli.command
        {
            let search_root = cmd_path_flag
                .as_ref()
                .map(|p| p.as_path())
                .or_else(|| cmd_path.as_ref().map(|p| p.as_path()))
                .unwrap_or(&cli.path);

            let effective_recursive = if cmd_path.is_none() && cmd_path_flag.is_none() {
                true
            } else {
                *recursive
            };

            // Refuse to run as root unless explicitly allowed
            #[cfg(unix)]
            {
                if unsafe { libc::geteuid() } == 0 && !cli.allow_root {
                    eprintln!("Refusing to run as root. Use --allow-root to override.");
                    return Err(RfgrepError::Other("Refused to run as root".to_string()));
                }
            }

            let matches = self.search_engine.search(
                search_root,
                pattern,
                *mode,
                *algorithm,
                effective_recursive,
                extensions.as_deref(),
                *context_lines,
                *case_sensitive,
                *invert_match,
                *max_matches,
                *timeout_per_file,
                cli.max_size,
                cli.skip_binary,
                cli.dry_run,
            ).await?;

            if matches.is_empty() {
                println!("{}", "No matches found".yellow());
            } else {
                println!(
                    "\n{} {} {}",
                    "Found".green(),
                    matches.len(),
                    "matches:".green()
                );

                let output = self.output_manager.format_results(
                    &matches,
                    pattern,
                    search_root,
                    *output_format,
                    *ndjson,
                    cli.color,
                )?;

                println!("\n{output}");

                if *copy && !matches.is_empty() {
                    self.output_manager.copy_to_clipboard(&output)?;
                }
            }
        }
        Ok(())
    }

    async fn handle_list(&self, cli: Cli) -> RfgrepResult<()> {
        if let Commands::List {
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
        } = cli.command
        {
            let list_root = cmd_path_flag
                .as_ref()
                .map(|p| p.as_path())
                .or_else(|| cmd_path.as_ref().map(|p| p.as_path()))
                .unwrap_or(&cli.path);

            let files = self.list_engine.list_files(
                list_root,
                *recursive,
                *show_hidden,
                extensions.as_deref(),
                *max_size,
                *min_size,
                *sort,
                *reverse,
                *limit,
            ).await?;

            if *long {
                self.list_engine.print_long_format(&files);
            } else {
                self.list_engine.print_simple_list(&files);
            }

            if *stats {
                self.list_engine.print_statistics(&files);
            }

            if *copy {
                self.list_engine.copy_to_clipboard(&files)?;
            }
        }
        Ok(())
    }

    async fn handle_interactive(&self, cli: Cli) -> RfgrepResult<()> {
        if let Commands::Interactive {
            pattern,
            algorithm,
            extensions,
            recursive,
            path: cmd_path,
            path_flag: cmd_path_flag,
        } = cli.command
        {
            let interactive_root = cmd_path_flag
                .as_ref()
                .map(|p| p.as_path())
                .or_else(|| cmd_path.as_ref().map(|p| p.as_path()))
                .unwrap_or(&cli.path);

            self.interactive_engine.run(
                interactive_root,
                pattern,
                *algorithm,
                *recursive,
                extensions.as_deref(),
            ).await?;
        }
        Ok(())
    }

    fn handle_completions(&self, shell: clap_complete::Shell) -> RfgrepResult<()> {
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
        let matches = crate::processor::search_file(path, &regex)?;
        
        let formatter = self.output_manager.create_formatter(
            crate::output::OutputFormat::Json,
            true, // ndjson
        );
        
        let output = formatter.format_results(&matches, pattern, path);
        print!("{output}");
        
        Ok(())
    }
}

impl Default for RfgrepApp {
    fn default() -> Self {
        Self::new().expect("Failed to create RfgrepApp")
    }
}
