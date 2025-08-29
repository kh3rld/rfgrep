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
use log::{debug, info, warn};
use processor::*;
use rayon::prelude::*;
use regex::Regex;
use rfgrep::metrics::Metrics;
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

    // Start Prometheus metrics endpoint on background thread
    let metrics = std::sync::Arc::new(Metrics::new());
    {
        let metrics_clone = metrics.clone();
        std::thread::spawn(move || {
            // Minimal hyper server to serve /metrics
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(async move {
                use hyper::service::{make_service_fn, service_fn};
                use hyper::{Body, Request, Response, Server};

                let make_svc = make_service_fn(move |_| {
                    let metrics = metrics_clone.clone();
                    async move {
                        Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                            let metrics = metrics.clone();
                            async move {
                                if req.uri().path() == "/metrics" {
                                    let body = metrics.gather();
                                    Ok::<_, hyper::Error>(Response::new(Body::from(body)))
                                } else {
                                    Ok::<_, hyper::Error>(Response::new(Body::from("Not Found")))
                                }
                            }
                        }))
                    }
                });

                let addr = ([127, 0, 0, 1], 9100).into();
                let server = Server::bind(&addr).serve(make_svc);
                if let Err(e) = server.await {
                    log::error!("Metrics server error: {e}");
                }
            });
        });
    }

    let pb = ProgressBar::new_spinner().with_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} [{elapsed_precise}] {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );

    match &cli.command {
        Commands::Worker { path, pattern } => {
            // Worker mode: perform a search on a single file and print NDJSON lines to stdout
            if let Ok(s) = std::env::var("RFGREP_WORKER_SLEEP") {
                if let Ok(sec) = s.parse::<u64>() {
                    std::thread::sleep(std::time::Duration::from_secs(sec));
                }
            }
            let regex = processor::get_or_compile_regex(pattern)?;
            let matches = processor::search_file(path, &regex)?;
            let formatter = crate::output_formats::OutputFormatter::new(
                crate::output_formats::OutputFormat::Json,
            )
            .with_ndjson(true);
            let out = formatter.format_results(&matches, pattern, path);
            print!("{out}");
            return Ok(());
        }

        Commands::Search {
            pattern,
            mode,
            copy,
            output_format,
            ndjson,
            timeout_per_file,
            extensions: _,
            recursive,
            context_lines: _,
            case_sensitive: _,
            invert_match: _,
            max_matches,
            algorithm: _,
            path: cmd_path,
            path_flag: cmd_path_flag,
            ..
        } => {
            let search_root = cmd_path_flag
                .as_ref()
                .map(|p| p.as_path())
                .or_else(|| cmd_path.as_ref().map(|p| p.as_path()))
                .unwrap_or(&cli.path);

            // If no explicit path was provided on the command line, default to recursive search
            // (search entire directory tree). If the user provided a path, respect --recursive flag.
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

            let regex = if matches!(mode, SearchMode::Regex) {
                processor::get_or_compile_regex(pattern)?
            } else {
                build_regex(pattern, mode)?
            };
            let matches = Mutex::new(Vec::<processor::SearchMatch>::new());
            let processing_errors = Mutex::new(Vec::<RfgrepError>::new()); // Mutex to collect errors

            // If NDJSON streaming requested, create a bounded channel for backpressure
            let ndjson_mode = *ndjson;
            let (nd_tx, nd_rx) = if ndjson_mode {
                let (s, r) = crossbeam_channel::bounded::<processor::SearchMatch>(1024);
                (Some(s), Some(r))
            } else {
                (None, None)
            };

            // Collect all files first
            // For search, include hidden/ignored files to ensure bundled examples (bench_data) are found
            let files: Vec<_> = walk_dir(search_root, effective_recursive, true)
                .filter(|entry| entry.path().is_file())
                .collect();

            info!("Found {} files to process", files.len());
            debug!(
                "Files: {:?}",
                files
                    .iter()
                    .map(|e| e.path().display().to_string())
                    .collect::<Vec<_>>()
            );
            pb.set_message(format!("Processing {} files...", files.len()));

            // Calculate adaptive chunk size
            let num_cores = num_cpus::get().max(1);
            let chunk_size = (files.len() / num_cores).max(1);

            // Process files in parallel using rayon with adaptive chunking
            let shared_metrics = metrics.clone();
            files.par_chunks(chunk_size).for_each(|chunk| {
                for entry in chunk {
                    let path = entry.path();
                    match process_file(
                        path,
                        &cli,
                        &regex,
                        &pb,
                        *timeout_per_file,
                        shared_metrics.clone(),
                    ) {
                        Ok(file_matches) => {
                            if !file_matches.is_empty() {
                                if let Some(tx) = &nd_tx {
                                    // Send matches to NDJSON channel (blocking when full) for backpressure
                                    for m in file_matches {
                                        // ignore send errors when receiver dropped
                                        let _ = tx.send(m);
                                    }
                                } else {
                                    let mut matches = matches.lock().unwrap();
                                    matches.extend(file_matches);
                                }
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

            // If NDJSON, drain the channel and print as we go
            if let Some(rx) = nd_rx {
                use crate::output_formats::OutputFormatter;
                let mut formatter = OutputFormatter::new(crate::output_formats::OutputFormat::Json)
                    .with_ndjson(true);
                formatter = match cli.color {
                    cli::ColorChoice::Always => formatter.with_color(true),
                    cli::ColorChoice::Never => formatter.with_color(false),
                    cli::ColorChoice::Auto => formatter,
                };

                // Receive until channel is empty and all producers are done
                while let Ok(m) = rx.recv() {
                    // Each received SearchMatch is a complete match; print NDJSON line
                    // Reuse formatter.format_json on a single-match slice
                    let line = formatter.format_results(&[m], pattern, &cli.path);
                    print!("{line}");
                }
            }

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

                // Structured output via OutputFormatter
                use crate::output_formats::OutputFormatter;
                match output_format {
                    cli::OutputFormat::Json => {
                        let mut formatter =
                            OutputFormatter::new(crate::output_formats::OutputFormat::Json)
                                .with_ndjson(*ndjson);
                        // set color based on CLI
                        formatter = match cli.color {
                            cli::ColorChoice::Always => formatter.with_color(true),
                            cli::ColorChoice::Never => formatter.with_color(false),
                            cli::ColorChoice::Auto => formatter, // default formatter already TTY-aware
                        };
                        // Use the search root as path metadata
                        let path = std::path::Path::new(&cli.path);
                        let json_out = formatter.format_results(&matches, pattern, path);
                        println!("\n{json_out}");
                    }
                    _ => {
                        // Fallback to plain text printer, group matches by file for readability
                        let mut formatter =
                            OutputFormatter::new(crate::output_formats::OutputFormat::Text);
                        formatter = match cli.color {
                            cli::ColorChoice::Always => formatter.with_color(true),
                            cli::ColorChoice::Never => formatter.with_color(false),
                            cli::ColorChoice::Auto => formatter,
                        };

                        // Group matches by path and print file header once
                        let mut matches_by_file: std::collections::BTreeMap<_, Vec<_>> =
                            std::collections::BTreeMap::new();
                        for m in &matches {
                            matches_by_file
                                .entry(m.path.clone())
                                .or_insert_with(Vec::new)
                                .push(m.clone());
                        }

                        for (file_path, file_matches) in matches_by_file {
                            println!("\n{}", file_path.display());
                            let out = formatter.format_results(&file_matches, pattern, &file_path);
                            println!("{out}");
                        }
                    }
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
                // Attempt to use system clipboard; in CI/headless environments fall back to a temp file
                // For clipboard/text we'll produce a text representation
                use crate::output_formats::{OutputFormat, OutputFormatter};
                let formatter = OutputFormatter::new(OutputFormat::Text);
                let path = std::path::Path::new(&cli.path);
                let joined = formatter.format_results(&matches, pattern, path);
                let can_use_clipboard =
                    std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok();

                if can_use_clipboard {
                    match arboard::Clipboard::new() {
                        Ok(mut clipboard) => match clipboard.set_text(joined.clone()) {
                            Ok(_) => println!("\n{}", "Results copied to clipboard!".green()),
                            Err(e) => {
                                warn!("Clipboard set failed: {e}. Falling back to temp file.");
                                let tmp = std::env::temp_dir().join("rfgrep_copy.txt");
                                if let Err(e) = std::fs::write(&tmp, joined) {
                                    warn!("Failed to write fallback clipboard file: {e}");
                                } else {
                                    println!(
                                        "\n{} {}",
                                        "Results written to".green(),
                                        tmp.display()
                                    );
                                }
                            }
                        },
                        Err(e) => {
                            warn!("Clipboard init failed: {e}. Falling back to temp file.");
                            let tmp = std::env::temp_dir().join("rfgrep_copy.txt");
                            if let Err(e) = std::fs::write(&tmp, joined) {
                                warn!("Failed to write fallback clipboard file: {e}");
                            } else {
                                println!("\n{} {}", "Results written to".green(), tmp.display());
                            }
                        }
                    }
                } else {
                    // Headless environment: write to temp file and inform the user
                    let tmp = std::env::temp_dir().join("rfgrep_copy.txt");
                    if let Err(e) = std::fs::write(&tmp, joined) {
                        warn!("Failed to write fallback clipboard file: {e}");
                    } else {
                        println!("\n{} {}", "Results written to".green(), tmp.display());
                    }
                }
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
            path: cmd_path,
            path_flag: cmd_path_flag,
        } => {
            let list_root = cmd_path_flag
                .as_ref()
                .map(|p| p.as_path())
                .or_else(|| cmd_path.as_ref().map(|p| p.as_path()))
                .unwrap_or(&cli.path);
            let files = Mutex::new(Vec::new());
            let total_size = AtomicU64::new(0);
            let extension_counts = Mutex::new(std::collections::HashMap::new());
            let processing_errors = Mutex::new(Vec::<RfgrepError>::new()); // Mutex to collect errors

            let entries: Vec<_> = walk_dir(list_root, *recursive, *show_hidden).collect();

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
            path: cmd_path,
            path_flag: cmd_path_flag,
        } => {
            use crate::config::PerformanceConfig;
            use crate::interactive::InteractiveSearchBuilder;
            use crate::search_algorithms::SearchAlgorithm;

            let interactive_root = cmd_path_flag
                .as_ref()
                .map(|p| p.as_path())
                .or_else(|| cmd_path.as_ref().map(|p| p.as_path()))
                .unwrap_or(&cli.path);

            let search_algorithm = match algorithm {
                cli::InteractiveAlgorithm::BoyerMoore => SearchAlgorithm::BoyerMoore,
                cli::InteractiveAlgorithm::Regex => SearchAlgorithm::Regex,
                cli::InteractiveAlgorithm::Simple => SearchAlgorithm::Simple,
            };

            let files: Vec<_> = walk_dir(interactive_root, *recursive, false)
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
    timeout_per_file: Option<u64>,
    metrics: std::sync::Arc<Metrics>,
) -> RfgrepResult<Vec<crate::processor::SearchMatch>> {
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

    // Update metrics
    metrics.files_scanned.inc();

    // If no timeout requested, do in-process search for simplicity
    if timeout_per_file.is_none() {
        let res = crate::processor::search_file(path, regex);
        if let Ok(ref v) = res {
            if v.is_empty() {
                metrics.files_skipped.inc();
            } else {
                metrics.matches_found.inc_by(v.len() as u64);
            }
        }
        return res;
    }

    // Use a process-based worker for per-file scanning to enable hard timeout (kill on timeout)
    // Build command: <current-exe> worker <path> <regex-as-str>
    let exe = std::env::current_exe().map_err(RfgrepError::Io)?;
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("worker");
    cmd.arg(path.as_os_str());
    // Pass the effective regex pattern string so worker uses the same compiled pattern
    cmd.arg(regex.as_str());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    // On unix, create a new process group so we can kill the group reliably
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                libc::setpgid(0, 0);
                Ok(())
            });
        }
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| RfgrepError::Other(format!("Spawn worker failed: {e}")))?;

    let dur = timeout_per_file
        .map(std::time::Duration::from_secs)
        .unwrap();
    let start = std::time::Instant::now();

    // Poll wait with small sleep and enforce kill if exceeded
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => break,
            Ok(None) => {
                if start.elapsed() > dur {
                    // timeout: kill child and its process group if possible
                    #[cfg(unix)]
                    {
                        unsafe {
                            let pid = child.id() as libc::pid_t;
                            let pgid = libc::getpgid(pid);
                            if pgid > 0 {
                                let _ = libc::kill(-pgid, libc::SIGKILL);
                            } else {
                                let _ = child.kill();
                            }
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        let _ = child.kill();
                    }
                    let _ = child.wait();
                    metrics.worker_timeouts.inc();
                    warn!("Timeout scanning file ({}s): {}", dur.as_secs(), file_name);
                    return Ok(vec![]);
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => {
                return Err(RfgrepError::Other(format!(
                    "Error waiting for child: {e}"
                )));
            }
        }
    }

    let output = child
        .wait_with_output()
        .map_err(|e| RfgrepError::Other(format!("Failed to collect child output: {e}")))?;

    // Parse NDJSON output (each line is a match JSON) or a full JSON array
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !stderr.is_empty() {
        debug!("Worker stderr for {file_name}: {stderr}");
    }

    let mut results = Vec::new();
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<processor::SearchMatch>(line) {
            Ok(m) => results.push(m),
            Err(_) => {
                // try to parse as object with matches array
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                    if let Some(arr) = v.get("matches").and_then(|a| a.as_array()) {
                        for it in arr {
                            if let Ok(m) =
                                serde_json::from_value::<processor::SearchMatch>(it.clone())
                            {
                                results.push(m);
                            }
                        }
                    }
                }
            }
        }
    }

    if results.is_empty() {
        metrics.files_skipped.inc();
    } else {
        metrics.matches_found.inc_by(results.len() as u64);
    }

    Ok(results)
}
