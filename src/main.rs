mod cli;
mod clipboard;
mod list;
mod processor;
mod walker;

use anyhow::Result;
use clap::Parser;
use cli::*;
use env_logger::{Builder, Env, Target};
use indicatif::{ProgressBar, ProgressStyle};
use list::*;
use log::{info, warn};
use processor::*;
use regex::Regex;
use std::fs;
use std::path::Path;
use walker::walk_dir;

fn main() -> Result<()> {
    let cli = Cli::parse();
    setup_logging(&cli)?;

    let pb = ProgressBar::new_spinner()
        .with_style(ProgressStyle::default_bar().template("{spinner} [{elapsed}] {msg}")?);

    match &cli.command {
        Commands::Search {
            pattern,
            mode,
            copy,
            extensions: _,
        } => {
            let regex = build_regex(pattern, mode)?;
            let mut matches = Vec::new();

            for entry in walk_dir(&cli.path, false, false) {
                if entry.file_type().is_dir() {
                    continue;
                }

                let path = entry.path();
                process_file(path, &cli, &regex, &mut matches, &pb)?;
            }

            if *copy && !matches.is_empty() {
                clipboard::copy_to_clipboard(&matches.join("\n"))?;
            }
        }

        Commands::List {
            extensions,
            long,
            recursive,
            show_hidden,
        } => {
            let mut files = Vec::new();
            let mut total_size = 0;
            let mut extension_counts = std::collections::HashMap::new();

            for entry in walk_dir(&cli.path, *recursive, *show_hidden) {
                let path = entry.path();
                if path.is_dir() {
                    continue;
                }

                if !should_list_file(path, &cli, extensions) {
                    continue;
                }

                let metadata = fs::metadata(path)?;
                let _file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default();

                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("none")
                    .to_string();

                *extension_counts.entry(ext.clone()).or_insert(0) += 1;
                total_size += metadata.len();

                files.push(FileInfo {
                    path: path.to_path_buf(),
                    size: metadata.len(),
                    extension: ext,
                    is_binary: processor::is_binary(path),
                });
            }

            if *long {
                print_long_format(&files);
            } else {
                print_simple_list(&files);
            }

            // Print summary
            println!("\nSummary:");
            println!("Total files: {}", files.len());
            println!("Total size: {} MB", total_size / 1024 / 1024);
            println!("Extensions:");
            for (ext, count) in extension_counts {
                println!("  .{}: {}", ext, count);
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

fn process_file(
    path: &Path,
    cli: &Cli,
    regex: &Regex,
    matches: &mut Vec<String>,
    pb: &ProgressBar,
) -> Result<()> {
    if let Commands::Search {
        extensions: Some(exts),
        ..
    } = &cli.command
    {
        if let Some(file_ext) = path.extension().and_then(|e| e.to_str()) {
            if !exts.iter().any(|e| e.eq_ignore_ascii_case(file_ext)) {
                return Ok(());
            }
        } else {
            return Ok(());
        }
    }
    pb.set_message(format!("Processing {}", path.display()));

    if cli.dry_run {
        info!("Dry run: {}", path.display());
        return Ok(());
    }

    if let Some(max) = cli.max_size {
        if path.metadata()?.len() > max * 1024 * 1024 {
            warn!("Skipping large file: {}", path.display());
            return Ok(());
        }
    }

    if cli.skip_binary && is_binary(path) {
        warn!("Skipping binary file: {}", path.display());
        return Ok(());
    }

    let file_matches = search_file(path, regex)?;
    matches.extend(file_matches);
    Ok(())
}
