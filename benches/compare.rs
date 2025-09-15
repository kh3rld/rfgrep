use criterion::{criterion_group, criterion_main, Criterion};
use rfgrep::search_algorithms::SearchAlgorithm;
use rfgrep::streaming_search::{StreamingConfig, StreamingSearchPipeline};

use std::fs;
use tempfile::TempDir;

fn criterion_benchmark(c: &mut Criterion) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path().to_path_buf();

    // Create test files
    fs::write(
        test_dir.join("test1.txt"),
        "This is a test file with pattern1 in it.",
    )
    .expect("Failed to write test file");
    fs::write(
        test_dir.join("test2.txt"),
        "Another file with pattern1 and more content.",
    )
    .expect("Failed to write test file");
    fs::write(
        test_dir.join("test3.txt"),
        "Third file without the pattern.",
    )
    .expect("Failed to write test file");

    // Create streaming search configuration
    let config = StreamingConfig {
        algorithm: SearchAlgorithm::BoyerMoore,
        context_lines: 0,
        case_sensitive: true,
        invert_match: false,
        max_matches: None,
        timeout_per_file: None,
        chunk_size: 8192,
        buffer_size: 65536,
    };

    let pipeline = StreamingSearchPipeline::new(config);
    let pattern = "pattern1";

    c.bench_function("rfgrep_search_streaming", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let _ = rt.block_on(async {
                let files = [test_dir.join("test1.txt"), test_dir.join("test2.txt")];
                let file_refs: Vec<&std::path::Path> = files.iter().map(|p| p.as_path()).collect();
                pipeline.search_files_parallel(&file_refs, pattern, 2).await
            });
        });
    });

    c.bench_function("rfgrep_search_single_file", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let _ = rt.block_on(async {
                pipeline
                    .search_file(&test_dir.join("test1.txt"), pattern)
                    .await
            });
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
