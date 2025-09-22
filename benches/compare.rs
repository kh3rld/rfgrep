use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rfgrep::search_algorithms::SearchAlgorithm;
use rfgrep::streaming_search::{StreamingConfig, StreamingSearchPipeline};

use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn criterion_benchmark(c: &mut Criterion) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path().to_path_buf();

    // Generate comprehensive test data
    generate_test_data(&test_dir);

    // Benchmark different algorithms
    benchmark_algorithms(c, &test_dir);

    // Benchmark different file sizes
    benchmark_file_sizes(c, &test_dir);

    // Benchmark different patterns
    benchmark_patterns(c, &test_dir);

    // Benchmark memory usage
    benchmark_memory_usage(c, &test_dir);
}

fn generate_test_data(test_dir: &Path) {
    // Create small files
    for i in 1..=100 {
        let content = format!(
            "This is test file {} with pattern1 and some content. Line {} has more text.",
            i, i
        );
        fs::write(test_dir.join(format!("small_{}.txt", i)), content).unwrap();
    }

    // Create medium files
    for i in 1..=20 {
        let content = "pattern1 ".repeat(100_000 / 9); // ~100KB
        fs::write(test_dir.join(format!("medium_{}.txt", i)), content).unwrap();
    }

    // Create large files
    for i in 1..=5 {
        let content = "pattern1 ".repeat(1_000_000 / 9); // ~1MB
        fs::write(test_dir.join(format!("large_{}.txt", i)), content).unwrap();
    }

    // Create binary files
    for i in 1..=10 {
        let content = vec![0u8; 50_000];
        fs::write(test_dir.join(format!("binary_{}.bin", i)), content).unwrap();
    }

    // Create source code files
    for i in 1..=50 {
        let content = format!(
            "fn test_function_{}() {{\n    let pattern1 = \"test\";\n    println!(\"{{}}\", pattern1);\n}}",
            i
        );
        fs::write(test_dir.join(format!("code_{}.rs", i)), content).unwrap();
    }
}

fn benchmark_algorithms(c: &mut Criterion, test_dir: &Path) {
    let algorithms = vec![
        ("boyer_moore", SearchAlgorithm::BoyerMoore),
        ("regex", SearchAlgorithm::Regex),
        ("simple", SearchAlgorithm::Simple),
    ];

    for (name, algorithm) in algorithms {
        let config = StreamingConfig {
            algorithm,
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

        c.bench_with_input(
            BenchmarkId::new("algorithm", name),
            &(pipeline, pattern),
            |b, (pipeline, pattern)| {
                b.iter(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    let _ = rt.block_on(async {
                        let files: Vec<_> = fs::read_dir(test_dir)
                            .unwrap()
                            .filter_map(|entry| entry.ok())
                            .map(|entry| entry.path())
                            .filter(|path| path.extension().is_some_and(|ext| ext == "txt"))
                            .collect();
                        let file_refs: Vec<&Path> = files.iter().map(|p| p.as_path()).collect();
                        pipeline.search_files_parallel(&file_refs, pattern, 4).await
                    });
                });
            },
        );
    }
}

fn benchmark_file_sizes(c: &mut Criterion, test_dir: &Path) {
    let sizes = vec![("small", 1_000), ("medium", 100_000), ("large", 1_000_000)];

    for (size_name, size_bytes) in sizes {
        let temp_file = test_dir.join(format!("size_test_{}.txt", size_name));
        let content = "pattern1 ".repeat(size_bytes / 9); // Approximate size
        fs::write(&temp_file, content).unwrap();

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

        c.bench_with_input(
            BenchmarkId::new("file_size", size_name),
            &(pipeline, pattern, temp_file),
            |b, (pipeline, pattern, file)| {
                b.iter(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    let _ = rt.block_on(async { pipeline.search_file(file, pattern).await });
                });
            },
        );
    }
}

fn benchmark_patterns(c: &mut Criterion, test_dir: &Path) {
    let patterns = vec![
        ("simple", "pattern1"),
        ("regex", r"\b\w+pattern\w+\b"),
        ("complex_regex", r"pattern\d+.*content"),
    ];

    for (pattern_name, pattern) in patterns {
        let algorithm = if pattern_name.starts_with("regex") {
            SearchAlgorithm::Regex
        } else {
            SearchAlgorithm::BoyerMoore
        };

        let config = StreamingConfig {
            algorithm,
            context_lines: 0,
            case_sensitive: true,
            invert_match: false,
            max_matches: None,
            timeout_per_file: None,
            chunk_size: 8192,
            buffer_size: 65536,
        };

        let pipeline = StreamingSearchPipeline::new(config);

        c.bench_with_input(
            BenchmarkId::new("pattern", pattern_name),
            &(pipeline, pattern),
            |b, (pipeline, pattern)| {
                b.iter(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    let _ = rt.block_on(async {
                        let files: Vec<_> = fs::read_dir(test_dir)
                            .unwrap()
                            .filter_map(|entry| entry.ok())
                            .map(|entry| entry.path())
                            .filter(|path| path.extension().is_some_and(|ext| ext == "txt"))
                            .take(10) // Limit for pattern benchmarks
                            .collect();
                        let file_refs: Vec<&Path> = files.iter().map(|p| p.as_path()).collect();
                        pipeline.search_files_parallel(&file_refs, pattern, 2).await
                    });
                });
            },
        );
    }
}

fn benchmark_memory_usage(c: &mut Criterion, test_dir: &Path) {
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

    c.bench_function("memory_usage", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();

            let _ = rt.block_on(async {
                let files: Vec<_> = fs::read_dir(test_dir)
                    .unwrap()
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.path())
                    .filter(|path| path.extension().is_some_and(|ext| ext == "txt"))
                    .collect();
                let file_refs: Vec<&Path> = files.iter().map(|p| p.as_path()).collect();
                pipeline.search_files_parallel(&file_refs, pattern, 4).await
            });
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
