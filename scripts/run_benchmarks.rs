use std::{
    env,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context, Result}; 
use chrono::Local; 
use rand::Rng; 

// --- Configuration Constants ---
const TEST_DIR_NAME: &str = "bench_data";
const RESULTS_BASE_DIR_NAME: &str = "results";
const RFGREP_DEV_PATH: &str = "./target/release/rfgrep";

struct Config {
    test_dir: PathBuf,
    results_dir: PathBuf,
    rfgrep_exe: PathBuf,
    project_root: PathBuf,
}

impl Config {
    fn new() -> Result<Self> {
        let mut current_dir = env::current_dir().context("Failed to get current directory")?;
        
        if current_dir.ends_with("benches") {
            current_dir.pop();
        }
        let project_root = current_dir;

        let date_str = Local::now().format("%Y-%m-%d").to_string();
        let results_dir = project_root.join(RESULTS_BASE_DIR_NAME).join(date_str);

        fs::create_dir_all(&results_dir)
            .with_context(|| format!("Failed to create results directory: {:?}", results_dir))?;

        let rfgrep_dev_full_path = project_root.join(RFGREP_DEV_PATH);
        let rfgrep_exe = if rfgrep_dev_full_path.exists() {
            rfgrep_dev_full_path
        } else {
            which::which("rfgrep").with_context(|| {
                format!(
                    "rfgrep executable not found at '{}' or in PATH. Please build rfgrep (e.g., 'cargo build --release') or ensure it's in your PATH.",
                    rfgrep_dev_full_path.display()
                )
            })?
        };

        Ok(Config {
            test_dir: project_root.join(TEST_DIR_NAME),
            results_dir,
            rfgrep_exe,
            project_root, 
        })
    }
}

fn generate_data(config: &Config) -> Result<()> {
    println!("Generating test data in '{}'...", config.test_dir.display());
    fs::create_dir_all(&config.test_dir)
        .with_context(|| format!("Failed to create test data directory: '{}'", config.test_dir.display()))?;

    let mut rng = rand::thread_rng();

    println!("Generating 10,000 small files (1-10KB)...");
    for i in 1..=10_000 {
        let size = rng.gen_range(1_000..=10_000);
        let file_path = config.test_dir.join(format!("file{}.txt", i));
        let mut file = File::create(&file_path)
            .with_context(|| format!("Failed to create small file: '{}'", file_path.display()))?;
        let mut buffer = vec![0u8; size];
        rng.fill(&mut buffer[..]); 
        file.write_all(&buffer)
            .with_context(|| format!("Failed to write to small file: '{}'", file_path.display()))?;
    }

    let medium_dir = config.test_dir.join("medium");
    fs::create_dir_all(&medium_dir)
        .with_context(|| format!("Failed to create medium files directory: '{}'", medium_dir.display()))?;
    println!("Generating 100 medium files (100KB-1MB)...");
    for i in 1..=100 {
        let size = rng.gen_range(100_000..=1_000_000);
        let file_path = medium_dir.join(format!("file{}.dat", i));
        let mut file = File::create(&file_path)
            .with_context(|| format!("Failed to create medium file: '{}'", file_path.display()))?;
        let mut buffer = vec![0u8; size];
        rng.fill(&mut buffer[..]);
        file.write_all(&buffer)
            .with_context(|| format!("Failed to write to medium file: '{}'", file_path.display()))?;
    }

    let large_dir = config.test_dir.join("large");
    fs::create_dir_all(&large_dir)
        .with_context(|| format!("Failed to create large files directory: '{}'", large_dir.display()))?;
    println!("Generating 5 large files (10-50MB)...");
    for i in 1..=5 {
        let size = rng.gen_range(10_000_000..=50_000_000);
        let file_path = large_dir.join(format!("file{}.bin", i));
        let mut file = File::create(&file_path)
            .with_context(|| format!("Failed to create large file: '{}'", file_path.display()))?;
        let mut buffer = vec![0u8; size];
        rng.fill(&mut buffer[..]);
        file.write_all(&buffer)
            .with_context(|| format!("Failed to write to large file: '{}'", file_path.display()))?;
    }
    println!("Test data generation complete.");
    Ok(())
}

/// Helper to run an external command and check its status.
fn run_external_command(program: &str, args: &[&str], current_dir: Option<&Path>) -> Result<()> {
    let mut cmd_builder = Command::new(program);
    cmd_builder.args(args);
    if let Some(dir) = current_dir {
        cmd_builder.current_dir(dir);
    }

    println!("Executing: {} {}", program, args.join(" "));

    let status = cmd_builder.status().with_context(|| {
        format!(
            "Failed to execute command: {} {}",
            program,
            args.join(" ")
        )
    })?;

    if !status.success() {
        anyhow::bail!(
            "Command failed with status {}: {} {}",
            status,
            program,
            args.join(" ")
        );
    }
    Ok(())
}

fn run_benchmarks(config: &Config) -> Result<()> {
    println!("Warming up rfgrep...");
    run_external_command(
        config.rfgrep_exe.to_str().unwrap(),
        &["search", "xyz123", config.test_dir.to_str().unwrap()],
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
            config
                .rfgrep_exe
                .to_str()
                .unwrap(),
            "search",
            "pattern1",
            config.test_dir.to_str().unwrap(),
            "grep",
            "-r",
            "pattern1",
            config.test_dir.to_str().unwrap(),
            "rg",
            "pattern1",
            config.test_dir.to_str().unwrap(),
            "fd",
            "-X",
            "grep",
            "pattern1",
            config.test_dir.to_str().unwrap(),
        ],
        None,
    )?;

    println!("Running extension filtering benchmarks...");
    run_external_command(
        "hyperfine",
        &[
            "--export-json",
            config.results_dir.join("extensions.json").to_str().unwrap(),
            config
                .rfgrep_exe
                .to_str()
                .unwrap(),
            "search",
            "pattern",
            "--extensions",
            "txt",
            config.test_dir.to_str().unwrap(),
            "rg",
            "pattern",
            "-g",
            "*.txt",
            config.test_dir.to_str().unwrap(),
        ],
        None,
    )?;

    println!("Running binary detection benchmarks...");
    run_external_command(
        "hyperfine",
        &[
            "--export-json",
            config.results_dir.join("binary.json").to_str().unwrap(),
            config
                .rfgrep_exe
                .to_str()
                .unwrap(),
            "search",
            "pattern",
            "--binary",
            config.test_dir.to_str().unwrap(),
            "rg",
            "pattern",
            "-g",
            "*.bin",
            config.test_dir.to_str().unwrap(),
            "fd",
            "-X",
            "rg",
            "pattern",
            "-g",
            "*.bin",
            config.test_dir.to_str().unwrap(),
        ],
        None,
    )?;
    Ok(())
}