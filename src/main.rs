mod cli;
mod clipboard;
mod list;
mod processor;
mod walker;

use anyhow::Context;
use anyhow::Result;
use byte_unit::Byte;
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
use walker::walk_dir;

fn main() -> Result<()> {
    let cli = Cli::parse();
    setup_logging(&cli)?;

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
            extensions: _,
            recursive,
        } => {
            let regex = build_regex(pattern, mode)?;
            let matches = Mutex::new(Vec::new());

            let files: Vec<_> = walk_dir(&cli.path, *recursive, false)
                .filter(|entry| entry.file_type().is_file())
                .collect();

            pb.set_message(format!("Processing {} files...", files.len()));

            files.par_chunks(100).for_each(|chunk| {
                for entry in chunk {
                    let path = entry.path();
                    if let Ok(file_matches) = process_file(path, &cli, &regex, &pb) {
                        if !file_matches.is_empty() {
                            let mut matches = matches.lock().unwrap();
                            matches.extend(file_matches);
                        }
                    }
                }
            });

            let mut matches = matches.into_inner().unwrap();
            matches.sort();

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
                    println!("{}", "-".repeat(80).dimmed());
                    println!("{}", m);
                }
            }

            if *copy && !matches.is_empty() {
                let mut clipboard =
                    arboard::Clipboard::new().context("Failed to access clipboard")?;
                clipboard
                    .set_text(matches.join("\n"))
                    .context("Failed to copy results to clipboard")?;
                println!("\n{}", "Results copied to clipboard!".green());
            }
        }

        Commands::List {
            extensions,
            long,
            recursive,
            show_hidden,
        } => {
            let files = Mutex::new(Vec::new());
            let total_size = AtomicU64::new(0);
            let extension_counts = Mutex::new(std::collections::HashMap::new());

            let entries: Vec<_> = walk_dir(&cli.path, *recursive, *show_hidden).collect();

            entries.par_iter().for_each(|entry| {
                let path = entry.path();
                if path.is_dir() {
                    return;
                }

                if !should_list_file(path, &cli, extensions) {
                    return;
                }

                if let Ok(metadata) = fs::metadata(path) {
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
                println!("  {}: {}", format!(".{}", ext).cyan(), count);
            }
        }
    }

    pb.finish_with_message("Done");
    Ok(())
}

fn setup_logging(cli: &Cli) -> Result<()> {
    let mut builder = Builder::from_env(Env::default().default_filter_or("info"));

    if let Some(log_path) = &cli.log {
        if let Some(parent_dir) = log_path.parent() {
            if !parent_dir.exists() {
                fs::create_dir_all(parent_dir)?;
            }
        }
        let log_file = fs::File::create(log_path)?;
        builder.target(Target::Pipe(Box::new(log_file)));
    } else {
        builder.target(Target::Stderr);
    }

    builder.try_init()?;
    Ok(())
}

fn build_regex(pattern: &str, mode: &SearchMode) -> Result<Regex> {
    let pattern = match mode {
        SearchMode::Text => regex::escape(pattern),
        SearchMode::Word => format!(r"\b{}\b", regex::escape(pattern)),
        SearchMode::Regex => pattern.to_string(),
    };
    Regex::new(&pattern).map_err(Into::into)
}

fn process_file(path: &Path, cli: &Cli, regex: &Regex, pb: &ProgressBar) -> Result<Vec<String>> {
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
    pb.set_message(format!("Processing {}", file_name));

    if cli.dry_run {
        info!("Dry run: {}", file_name);
        return Ok(vec![]);
    }

    if let Some(max) = cli.max_size {
        if let Ok(metadata) = path.metadata() {
            let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
            if size_mb > max as f64 {
                warn!("Skipping large file ({}MB): {}", size_mb.round(), file_name);
                return Ok(vec![]);
            }
        }
    }

    if cli.skip_binary && is_binary(path) {
        warn!("Skipping binary file: {}", file_name);
        return Ok(vec![]);
    }

    search_file(path, regex).with_context(|| format!("Failed to search in file: {}", file_name))
}
