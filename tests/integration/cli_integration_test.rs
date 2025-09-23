use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn target_debug() -> PathBuf {
    let mut p = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/rfgrep");
    p
}

#[test]
fn test_basic_search_functionality() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "This is a test file with some content\n")?;

    Command::new(target_debug())
        .arg("search")
        .arg("test")
        .arg("--")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Found"));

    Ok(())
}

#[test]
fn test_search_with_options() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "This is a test file with some content\n")?;

    Command::new(target_debug())
        .arg("search")
        .arg("test")
        .arg("--recursive")
        .arg("--extensions")
        .arg("txt")
        .arg("--context-lines")
        .arg("2")
        .arg("--")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Found"));

    Ok(())
}

#[test]
fn test_list_command() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "content")?;

    Command::new(target_debug())
        .arg("list")
        .arg("--")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Summary"));

    Ok(())
}

#[test]
fn test_list_with_options() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "content")?;

    Command::new(target_debug())
        .arg("list")
        .arg("--long")
        .arg("--recursive")
        .arg("--extensions")
        .arg("txt")
        .arg("--")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Summary"));

    Ok(())
}

#[test]
fn test_simulate_command() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "This is a test file with some content\n")?;

    Command::new(target_debug())
        .arg("simulate")
        .arg("--")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Simulation"));

    Ok(())
}

#[test]
fn test_file_type_classification() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    
    // Create different file types
    fs::write(temp_dir.path().join("test.txt"), "text content")?;
    fs::write(temp_dir.path().join("test.rs"), "rust code")?;
    fs::write(temp_dir.path().join("test.json"), r#"{"key": "value"}"#)?;
    fs::write(temp_dir.path().join("test.exe"), b"\x00\x01\x02\x03")?;

    Command::new(target_debug())
        .arg("search")
        .arg("test")
        .arg("--file-types")
        .arg("default")
        .arg("--")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Found"));

    Ok(())
}

#[test]
fn test_safety_policy_options() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "This is a test file with some content\n")?;

    Command::new(target_debug())
        .arg("search")
        .arg("test")
        .arg("--safety-policy")
        .arg("conservative")
        .arg("--")
        .arg(temp_dir.path())
        .assert()
        .success();

    Ok(())
}

#[test]
fn test_threading_options() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "This is a test file with some content\n")?;

    Command::new(target_debug())
        .arg("search")
        .arg("test")
        .arg("--threads")
        .arg("4")
        .arg("--")
        .arg(temp_dir.path())
        .assert()
        .success();

    Ok(())
}

#[test]
fn test_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    // Test with nonexistent directory
    Command::new(target_debug())
        .arg("search")
        .arg("test")
        .arg("--")
        .arg("/nonexistent/directory")
        .assert()
        .failure();

    Ok(())
}

#[test]
fn test_help_commands() -> Result<(), Box<dyn std::error::Error>> {
    Command::new(target_debug())
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("rfgrep"));

    Command::new(target_debug())
        .arg("search")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("search"));

    Command::new(target_debug())
        .arg("list")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("list"));

    Ok(())
}

#[test]
fn test_version_command() -> Result<(), Box<dyn std::error::Error>> {
    Command::new(target_debug())
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("rfgrep"));

    Ok(())
}
