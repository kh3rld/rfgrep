mod cli;
mod config;
mod error;
mod interactive;
mod list;
mod memory;
mod output_formats;
mod processor;
mod search_algorithms;
mod walker;

use crate::error::{Result as RfgrepResult, RfgrepError};
use byte_unit::Byte;
use clap::CommandFactory;
use clap::Parser;
use cli::*;
use colored::*;
use env_logger::{Builder, Env, Target};
use indicatif::{ProgressBar, ProgressStyle};
use list::*;
use log::{info, warn};
use processor::*;
use rayon::prelude::*;
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use walker::walk_dir;

fn main() -> RfgrepResult<()> {
    let cli = Cli::parse();
    setup_logging(&cli)?;

    let start_time = Instant::now();
    info!("Application started with command: {:?}", cli.command);

    let pb = ProgressBar::new_spinner().with_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} [{elapsed_precise}] {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );

    match &cli.command {
        Commands::Search {
            pattern,
            mode,
            copy,
            output_format,
            extensions: _,
            recursive,
            context_lines: _,
            case_sensitive: _,
            invert_match: _,
            max_matches,
            algorithm: _,
        } => {
            let regex = if matches!(mode, SearchMode::Regex) {
                processor::get_or_compile_regex(pattern)?
            } else {
                build_regex(pattern, mode)?
            };
            let matches = Mutex::new(Vec::new());
            let processing_errors = Mutex::new(Vec::<RfgrepError>::new()); // Mutex to collect errors

            // Collect all files first
            let files: Vec<_> = walk_dir(&cli.path, *recursive, false)
                .filter(|entry| entry.path().is_file())
                .collect();

            pb.set_message(format!("Processing {} files...", files.len()));

            // Calculate adaptive chunk size
            let num_cores = num_cpus::get().max(1);
            let chunk_size = (files.len() / num_cores).max(1);

            // Process files in parallel using rayon with adaptive chunking
            files.par_chunks(chunk_size).for_each(|chunk| {
                for entry in chunk {
                    let path = entry.path();
                    match process_file(path, &cli, &regex, &pb) {
                        Ok(file_matches) => {
                            if !file_matches.is_empty() {
                                let mut matches = matches.lock().unwrap();
                                matches.extend(file_matches);
                            }
                        }
                        Err(e) => {
                            // Collect the error
                            let mut errors = processing_errors.lock().unwrap();
                            errors.push(e);
                        }
                    }
                }
            });

            let mut matches = matches.into_inner().unwrap();
            matches.sort();

            // Apply max_matches limit if specified
            if let Some(max_matches) = max_matches
                && matches.len() > *max_matches
            {
                matches.truncate(*max_matches);
            }

            if matches.is_empty() {
                println!("{}", "No matches found".yellow());
            } else {
                println!(
                    "\n{} {} {}",
                    "Found".green(),
                    matches.len(),
                    "matches:".green()
                );
                for m in &matches {
                    println!("{m}");
                }

                // TODO: Integrate output formats when SearchMatch conversion is ready
                if matches!(output_format, cli::OutputFormat::Json) {
                    println!("\n{}", "JSON output format not yet integrated".yellow());
                }
            }

            let collected_errors = processing_errors.into_inner().unwrap();
            if !collected_errors.is_empty() {
                eprintln!("\n{}", "Errors encountered during processing:".red().bold());
                for err in collected_errors {
                    eprintln!("{}", err.to_string().red());
                }
            }

            if *copy && !matches.is_empty() {
                let mut clipboard = arboard::Clipboard::new().map_err(RfgrepError::Clipboard)?;
                clipboard
                    .set_text(matches.join("\n"))
                    .map_err(RfgrepError::Clipboard)?;
                println!("\n{}", "Results copied to clipboard!".green());
            }
        }

        Commands::List {
            extensions,
            long,
            recursive,
            show_hidden,
            max_size: _,
            min_size: _,
            detailed: _,
            simple: _,
            stats: _,
            sort: _,
            reverse: _,
            limit: _,
            copy: _,
            output_format: _,
        } => {
            let files = Mutex::new(Vec::new());
            let total_size = AtomicU64::new(0);
            let extension_counts = Mutex::new(std::collections::HashMap::new());
            let processing_errors = Mutex::new(Vec::<RfgrepError>::new()); // Mutex to collect errors

            let entries: Vec<_> = walk_dir(&cli.path, *recursive, *show_hidden).collect();

            entries.par_iter().for_each(|entry| {
                let path = entry.path();
                if path.is_dir() {
                    return;
                }

                if !should_list_file(path, &cli, extensions) {
                    return;
                }

                match fs::metadata(path).map_err(RfgrepError::Io) {
                    Ok(metadata) => {
                        let ext = path
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("none")
                            .to_string();

                        let file_info = FileInfo {
                            path: path.to_path_buf(),
                            size: metadata.len(),
                            extension: ext.clone(),
                            is_binary: processor::is_binary(path),
                        };

                        {
                            let mut counts_locked = extension_counts.lock().unwrap();
                            *counts_locked.entry(ext).or_insert(0) += 1;
                        }
                        total_size.fetch_add(metadata.len(), Ordering::Relaxed);
                        {
                            let mut files_locked = files.lock().unwrap();
                            files_locked.push(file_info);
                        }
                    }
                    Err(e) => {
                        let mut errors = processing_errors.lock().unwrap();
                        errors.push(e);
                    }
                }
            });

            let mut files = files.into_inner().unwrap();
            files.par_sort_by_key(|f| f.size);

            if *long {
                print_long_format(&files);
            } else {
                print_simple_list(&files);
            }

            let extension_counts_map = extension_counts.into_inner().unwrap();
            let mut ext_counts: Vec<_> = extension_counts_map.into_iter().collect();
            ext_counts.par_sort_by(|a, b| b.1.cmp(&a.1));

            println!("\n{}", "Summary:".green().bold());
            println!("{}: {}", "Total files".cyan(), files.len());
            let adjusted = Byte::from_u64(total_size.load(Ordering::Relaxed))
                .get_appropriate_unit(byte_unit::UnitType::Binary);
            println!(
                "{}: {:.2} {}",
                "Total size".cyan(),
                adjusted.get_value(),
                adjusted.get_unit()
            );
            println!("\n{}", "Extensions:".green().bold());
            for (ext, count) in ext_counts {
                println!("  {}: {}", format!(".{ext}").cyan(), count);
            }

            let collected_errors = processing_errors.into_inner().unwrap();
            if !collected_errors.is_empty() {
                eprintln!("\n{}", "Errors encountered during processing:".red().bold());
                for err in collected_errors {
                    eprintln!("{}", err.to_string().red());
                }
            }
        }
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(*shell, &mut cmd, "rfgrep", &mut std::io::stdout());
        }
        Commands::Interactive {
            pattern,
            algorithm,
            extensions,
            recursive,
        } => {
            use crate::config::PerformanceConfig;
            use crate::interactive::InteractiveSearchBuilder;
            use crate::search_algorithms::SearchAlgorithm;

            let search_algorithm = match algorithm {
                cli::InteractiveAlgorithm::BoyerMoore => SearchAlgorithm::BoyerMoore,
                cli::InteractiveAlgorithm::Regex => SearchAlgorithm::Regex,
                cli::InteractiveAlgorithm::Simple => SearchAlgorithm::Simple,
            };

            let files: Vec<_> = walk_dir(&cli.path, *recursive, false)
                .filter(|entry| entry.path().is_file())
                .map(|entry| entry.path().to_path_buf())
                .collect();

            let filtered_files: Vec<_> = if let Some(exts) = extensions {
                files
                    .into_iter()
                    .filter(|path| {
                        if let Some(ext) = path.extension()
                            && let Some(ext_str) = ext.to_str()
                        {
                            return exts.contains(&ext_str.to_string());
                        }
                        false
                    })
                    .collect()
            } else {
                files
            };

            println!("{}", "Starting interactive search mode...".green().bold());
            println!("Pattern: {}", pattern.yellow());
            println!("Algorithm: {algorithm:?}");
            println!("Files to search: {}", filtered_files.len());
            println!("{}", "Press 'q' to quit, 'h' for help".dimmed());

            let config = PerformanceConfig::default();
            let mut interactive_search = InteractiveSearchBuilder::new(pattern)
                .algorithm(search_algorithm)
                .files(filtered_files)
                .config(config)
                .build();

            if let Err(e) = interactive_search.run() {
                eprintln!("{}", format!("Interactive mode error: {e}").red());
                return Err(RfgrepError::Io(e));
            }
        }
    }

    pb.finish_with_message("Done");
    info!(
        "Application finished. Total elapsed time: {:.2?}",
        start_time.elapsed()
    );
    Ok(())
}

fn setup_logging(cli: &Cli) -> RfgrepResult<()> {
    let mut builder = Builder::from_env(Env::default().default_filter_or("info"));

    builder.format(|buf, record| {
        use std::io::Write;
        writeln!(
            buf,
            "{} [{}] [{}] {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            record.level(),
            record.module_path().unwrap_or("unknown"),
            record.args()
        )
    });

    if let Some(log_path) = &cli.log {
        if let Some(parent_dir) = log_path.parent()
            && !parent_dir.exists()
        {
            fs::create_dir_all(parent_dir).map_err(RfgrepError::Io)?;
        }
        let log_file = fs::File::create(log_path).map_err(RfgrepError::Io)?;
        builder.target(Target::Pipe(Box::new(log_file)));
    } else {
        builder.target(Target::Stderr);
    }

    builder
        .try_init()
        .map_err(|e| RfgrepError::Other(e.to_string()))?;
    Ok(())
}

fn build_regex(pattern: &str, mode: &SearchMode) -> RfgrepResult<Regex> {
    let pattern = match mode {
        SearchMode::Text => regex::escape(pattern),
        SearchMode::Word => format!(r"\b{}\b", regex::escape(pattern)),
        SearchMode::Regex => pattern.to_string(),
    };
    Regex::new(&pattern).map_err(RfgrepError::Regex)
}

fn process_file(
    path: &Path,
    cli: &Cli,
    regex: &Regex,
    pb: &ProgressBar,
) -> RfgrepResult<Vec<String>> {
    // todo: Changed return type to RfgrepResult
    if let Commands::Search {
        extensions: Some(exts),
        ..
    } = &cli.command
    {
        if let Some(file_ext) = path.extension().and_then(|e| e.to_str()) {
            if !exts.iter().any(|e| e.eq_ignore_ascii_case(file_ext)) {
                return Ok(vec![]);
            }
        } else {
            return Ok(vec![]);
        }
    }

    let file_name = path.display().to_string();
    pb.set_message(format!("Processing {file_name}"));

    if cli.dry_run {
        info!("Dry run: {file_name}");
        return Ok(vec![]);
    }

    if let Some(max) = cli.max_size
        && let Ok(metadata) = path.metadata()
    {
        let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
        if size_mb > max as f64 {
            warn!("Skipping large file ({}MB): {}", size_mb.round(), file_name);
            return Ok(vec![]);
        }
    }

    if cli.skip_binary && is_binary(path) {
        warn!("Skipping binary file: {file_name}");
        return Ok(vec![]);
    }

    search_file(path, regex).map_err(|e| RfgrepError::FileProcessing {
        path: path.to_path_buf(),
        source: Box::new(e),
    })
}
