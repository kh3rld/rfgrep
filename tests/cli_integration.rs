use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;

fn target_debug() -> PathBuf {
    let mut p = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/rfgrep");
    p
}

#[test]
fn run_readme_examples() -> Result<(), Box<dyn std::error::Error>> {
    let test_dir = PathBuf::from("bench_data");
    if !test_dir.exists() {
        fs::create_dir_all(&test_dir)?;
        fs::write(
            test_dir.join("file1.txt"),
            "pattern\nHashMap in this file\nsome other line\nfile123\nfile12\nfile1234\n",
        )?;
        fs::write(test_dir.join("file2.dat"), "file1\npattern\nfile123\n")?;
        fs::create_dir_all(test_dir.join("subdir"))?;
        fs::write(test_dir.join("subdir/nested.txt"), "nested content\n")?;
        fs::write(test_dir.join("binary.bin"), "\x00\x01\x02")?;
        fs::write(test_dir.join(".hidden.txt"), "hidden\n")?;
    }

    let bin = target_debug();

    // 1: Basic Search
    Command::new(&bin)
        .arg("search")
        .arg("pattern")
        .arg("--recursive")
        .arg("--")
        .arg("bench_data")
        .assert()
        .success()
        .stdout(predicate::str::contains("Found"));

    // 2: Search with Options (regex, extensions, max-size, skip-binary, copy)
    Command::new(&bin)
        .arg("search")
        .arg("pattern")
        .arg("--mode")
        .arg("regex")
        .arg("--extensions")
        .arg("txt,dat")
        .arg("--max-size")
        .arg("5")
        .arg("--skip-binary")
        .arg("--copy")
        .arg("--")
        .arg("bench_data")
        .assert()
        .success();

    // 3: File Listing - Simple
    Command::new(&bin)
        .arg("list")
        .arg("--")
        .arg("bench_data")
        .assert()
        .success()
        .stdout(predicate::str::contains("Summary"));

    // 4: File Listing - Detailed and Recursive
    Command::new(&bin)
        .arg("list")
        .arg("--long")
        .arg("--recursive")
        .arg("--")
        .arg("bench_data")
        .assert()
        .success()
        .stdout(predicate::str::contains("Extensions"));

    // 5: File Listing - With Filters
    Command::new(&bin)
        .arg("list")
        .arg("--extensions")
        .arg("txt,dat")
        .arg("--max-size")
        .arg("10")
        .arg("--show-hidden")
        .arg("--")
        .arg("bench_data")
        .assert()
        .success();

    // 6: Example Search - 'HashMap' in .txt files
    Command::new(&bin)
        .arg("search")
        .arg("HashMap")
        .arg("--extensions")
        .arg("txt")
        .arg("--")
        .arg("bench_data")
        .assert()
        .success()
        .stdout(predicate::str::contains("HashMap"));

    // 7: Example List - .txt files under 1MB
    Command::new(&bin)
        .arg("list")
        .arg("--extensions")
        .arg("txt")
        .arg("--max-size")
        .arg("1")
        .arg("--")
        .arg("bench_data")
        .assert()
        .success();

    // 8: Example Search - Regex and Copy
    Command::new(&bin)
        .arg("search")
        .arg("file\\d+")
        .arg("--mode")
        .arg("regex")
        .arg("--copy")
        .arg("--")
        .arg("bench_data")
        .assert()
        .success();

    // 9: Global Option --log with search
    let log_search = "search_test.log";
    let _ = fs::remove_file(log_search);
    Command::new(&bin)
        .arg("search")
        .arg("some_content_for_log_test")
        .arg("--log")
        .arg(log_search)
        .arg("--")
        .arg("bench_data")
        .assert()
        .success();
    assert!(fs::metadata(log_search).is_ok());
    let _ = fs::remove_file(log_search);

    // 10: Global Option --log with list
    let log_list = "list_test.log";
    let _ = fs::remove_file(log_list);
    Command::new(&bin)
        .arg("list")
        .arg("--log")
        .arg(log_list)
        .arg("--")
        .arg("bench_data")
        .assert()
        .success();
    assert!(fs::metadata(log_list).is_ok());
    let _ = fs::remove_file(log_list);

    // 11: Global Option --path with search (using --path-flag)
    Command::new(&bin)
        .arg("search")
        .arg("another_pattern_for_path_test")
        .arg("--path-flag")
        .arg("bench_data")
        .assert()
        .success();

    // 12: Global Option --path with list
    Command::new(&bin)
        .arg("list")
        .arg("--path-flag")
        .arg("bench_data")
        .assert()
        .success();

    // 13: Search Command --mode text
    Command::new(&bin)
        .arg("search")
        .arg("specific text pattern")
        .arg("--mode")
        .arg("text")
        .arg("--")
        .arg("bench_data")
        .assert()
        .success();

    // 14: Search Command --mode word
    Command::new(&bin)
        .arg("search")
        .arg("pattern")
        .arg("--mode")
        .arg("word")
        .arg("--")
        .arg("bench_data")
        .assert()
        .success();

    // 15: Search Command --dry-run
    Command::new(&bin)
        .arg("search")
        .arg("pattern_for_dry_run")
        .arg("--dry-run")
        .arg("--")
        .arg("bench_data")
        .assert()
        .success();

    // 16: List Command --skip-binary
    Command::new(&bin)
        .arg("list")
        .arg("--skip-binary")
        .arg("--")
        .arg("bench_data")
        .assert()
        .success();

    Ok(())
}
