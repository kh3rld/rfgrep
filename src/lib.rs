pub mod cli;
pub mod clipboard;
pub mod list;
pub mod processor;
pub mod walker;
mod search;
pub mod error;
mod progress;
mod config;
use crate::config::Config;
pub use anyhow::{Context, Result};
pub use clap::Parser;
pub use cli::{Cli, Commands, SearchMode};
pub use list::{FileInfo, print_long_format, print_simple_list, should_list_file};
pub use processor::{SearchMatch, is_binary, search_file};
use std::path::Path;
pub use std::path::PathBuf;
pub use walker::walk_dir;

pub struct AppConfig {
    pub chunk_size: Option<u32>,
    pub rfgrep_exe: PathBuf,
    pub results_dir: PathBuf,
}
impl AppConfig {
    pub fn from_cli(cli: &Cli) -> Self {
        let rfgrep_exe = cli.path.join("rfgrep");
        let results_dir = cli.path.join("results");
        std::fs::create_dir_all(&results_dir).expect("Failed to create results directory");

        AppConfig {
            chunk_size: Some(100), // Default chunk size
            rfgrep_exe,
            results_dir,
        }
    }
}

pub fn load_config() -> AppConfig {
    let mut cfg = Config::default();
    // Load config from file if available
    if let Ok(config) = Config::load() {
        cfg = config;
    }
    // Convert to AppConfig
    AppConfig {
        chunk_size: Some(cfg.search.chunk_size as u32),
        rfgrep_exe: std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("rfgrep")),
        results_dir: std::path::PathBuf::from("results"),
    }
}

pub fn run_external_command(
    command: &str,
    args: &[&str],
    env: Option<&str>,
) -> std::io::Result<()> {
    let mut cmd = std::process::Command::new(command);
    cmd.args(args);
    if let Some(env_var) = env {
        cmd.env("RFGREP_TEST_ENV", env_var);
    }
    cmd.status()?;
    Ok(())
}
pub fn run_benchmarks(config: &AppConfig, test_dir: &Path) -> Result<()> {
    println!("Warming up rfgrep...");
    run_external_command(
        config.rfgrep_exe.to_str().unwrap(),
        &["search", "xyz123", test_dir.to_str().unwrap()],
        None,
    )?;

    println!("Running search performance benchmarks...");
    run_external_command(
        "hyperfine",
        &[
            "--warmup",
            "3",
            "--export-json",
            config.results_dir.join("search.json").to_str().unwrap(),
            "--export-markdown",
            config.results_dir.join("search.md").to_str().unwrap(),
            config.rfgrep_exe.to_str().unwrap(),
            "search",
            "pattern1",
            test_dir.to_str().unwrap(),
        ],
        None,
    )?;

    Ok(())
}
pub fn run_benchmarks_cli(cli: &Cli) -> Result<()> {
    let config = AppConfig::from_cli(cli);
    let test_dir = cli.path.join("test_data");

    if !test_dir.exists() {
        std::fs::create_dir_all(&test_dir).context("Failed to create test directory")?;
    }

    run_benchmarks(&config, &test_dir)
}
