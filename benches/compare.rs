use criterion::{Criterion, criterion_group, criterion_main};
use rfgrep::AppConfig;
use rfgrep::Cli;
use rfgrep::Parser;

use std::fs;
use tempfile::TempDir;

fn run_external_command(command: &str, args: &[&str], env: Option<&str>) -> std::io::Result<()> {
    let mut cmd = std::process::Command::new(command);
    cmd.args(args);
    if let Some(env_var) = env {
        cmd.env("RFGREP_TEST_ENV", env_var);
    }
    cmd.status()?;
    Ok(())
}

fn criterion_benchmark(c: &mut Criterion) {
    let cli = Cli::parse_from(["rfgrep", "search", "pattern1"]);
    let config = AppConfig::from_cli(&cli);

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path().to_path_buf();

    fs::write(test_dir.join("test.txt"), "This is a test file.")
        .expect("Failed to write test file");

    c.bench_function("rfgrep_search", |b| {
        b.iter(|| {
            let _ = run_external_command(
                config.rfgrep_exe.to_str().unwrap(),
                &["search", "pattern1", test_dir.to_str().unwrap()],
                None,
            );
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
