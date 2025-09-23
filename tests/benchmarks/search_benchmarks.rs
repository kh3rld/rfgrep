use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rfgrep::search_algorithms::{BoyerMooreSearch, RegexSearch, SimdSearch, SearchAlgorithmTrait};
use std::time::Duration;

fn create_test_content(size: usize) -> String {
    let mut content = String::with_capacity(size);
    for i in 0..size {
        if i % 100 == 0 {
            content.push_str("pattern ");
        } else {
            content.push_str("content ");
        }
    }
    content
}

fn bench_search_algorithms(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_algorithms");
    group.measurement_time(Duration::from_secs(10));
    
    let sizes = vec![1000, 10000, 100000, 1000000];
    let pattern = "pattern";
    
    for size in sizes {
        let content = create_test_content(size);
        
        group.bench_with_input(
            BenchmarkId::new("boyer_moore", size),
            &content,
            |b, content| {
                let searcher = BoyerMooreSearch::new(pattern);
                b.iter(|| {
                    black_box(searcher.search_with_context(content, pattern, 2))
                })
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("simd", size),
            &content,
            |b, content| {
                let searcher = SimdSearch::new(pattern);
                b.iter(|| {
                    black_box(searcher.search_with_context(content, pattern, 2))
                })
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("regex", size),
            &content,
            |b, content| {
                let searcher = RegexSearch::new(pattern).unwrap();
                b.iter(|| {
                    black_box(searcher.search_with_context(content, pattern, 2))
                })
            },
        );
    }
    
    group.finish();
}

fn bench_different_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("pattern_types");
    group.measurement_time(Duration::from_secs(5));
    
    let content = create_test_content(100000);
    let patterns = vec![
        ("short", "test"),
        ("medium", "pattern"),
        ("long", "this_is_a_very_long_pattern_to_test"),
        ("regex", r"\b\w+@\w+\.\w+\b"),
    ];
    
    for (name, pattern) in patterns {
        group.bench_with_input(
            BenchmarkId::new("boyer_moore", name),
            &pattern,
            |b, pattern| {
                let searcher = BoyerMooreSearch::new(pattern);
                b.iter(|| {
                    black_box(searcher.search_with_context(&content, pattern, 2))
                })
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("simd", name),
            &pattern,
            |b, pattern| {
                let searcher = SimdSearch::new(pattern);
                b.iter(|| {
                    black_box(searcher.search_with_context(&content, pattern, 2))
                })
            },
        );
    }
    
    group.finish();
}

fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    group.measurement_time(Duration::from_secs(5));
    
    let content = create_test_content(1000000);
    let pattern = "pattern";
    
    group.bench_function("boyer_moore_memory", |b| {
        let searcher = BoyerMooreSearch::new(pattern);
        b.iter(|| {
            let matches = searcher.search_with_context(&content, pattern, 2);
            black_box(matches.len())
        })
    });
    
    group.bench_function("simd_memory", |b| {
        let searcher = SimdSearch::new(pattern);
        b.iter(|| {
            let matches = searcher.search_with_context(&content, pattern, 2);
            black_box(matches.len())
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_search_algorithms,
    bench_different_patterns,
    bench_memory_usage
);
criterion_main!(benches);
