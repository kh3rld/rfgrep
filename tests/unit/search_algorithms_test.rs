use regex::Regex;
use rfgrep::search_algorithms::{BoyerMooreSearch, RegexSearch, SearchAlgorithmTrait, SimdSearch};

#[cfg(test)]
mod boyer_moore_tests {
    use super::*;

    #[test]
    fn test_simple_pattern() {
        let searcher = BoyerMooreSearch::new("test");
        let content = "This is a test file";
        let matches = searcher.search_with_context(content, "test", 2);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].line_number, 1);
        assert!(matches[0].line.contains("test"));
    }

    #[test]
    fn test_multiple_matches() {
        let searcher = BoyerMooreSearch::new("test");
        let content = "test line 1\ntest line 2\nnot a test\nanother test line";
        let matches = searcher.search_with_context(content, "test", 2);

        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn test_no_matches() {
        let searcher = BoyerMooreSearch::new("nonexistent");
        let content = "This file has no matches";
        let matches = searcher.search_with_context(content, "nonexistent", 2);

        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_case_sensitive() {
        let searcher = BoyerMooreSearch::new("Test");
        let content = "test\nTEST\nTest\nTeSt";
        let matches = searcher.search_with_context(content, "Test", 2);

        assert_eq!(matches.len(), 2); // "Test" and "TeSt"
    }

    #[test]
    fn test_empty_pattern() {
        let searcher = BoyerMooreSearch::new("");
        let content = "Some content";
        let matches = searcher.search_with_context(content, "", 2);

        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_empty_content() {
        let searcher = BoyerMooreSearch::new("test");
        let content = "";
        let matches = searcher.search_with_context(content, "test", 2);

        assert_eq!(matches.len(), 0);
    }
}

#[cfg(test)]
mod regex_tests {
    use super::*;

    #[test]
    fn test_simple_regex() {
        let searcher = RegexSearch::new("test").unwrap();
        let content = "This is a test file";
        let matches = searcher.search_with_context(content, "test", 2);

        assert_eq!(matches.len(), 1);
        assert!(matches[0].line.contains("test"));
    }

    #[test]
    fn test_complex_regex() {
        let searcher = RegexSearch::new(r"\b\w+@\w+\.\w+\b").unwrap();
        let content = "Email: test@example.com and another@domain.org";
        let matches = searcher.search_with_context(content, r"\b\w+@\w+\.\w+\b", 2);

        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_invalid_regex() {
        // deepsource-ignore RS-E1002: Intentionally invalid regex to verify error handling
        let result = RegexSearch::new("[invalid regex");
        assert!(result.is_err());
    }

    #[test]
    fn test_regex_with_special_chars() {
        let searcher = RegexSearch::new(r"\$\d+\.\d{2}").unwrap();
        let content = "Price: $100.50 and $25.99";
        let matches = searcher.search_with_context(content, r"\$\d+\.\d{2}", 2);

        assert_eq!(matches.len(), 2);
    }
}

#[cfg(test)]
mod simd_tests {
    use super::*;

    #[test]
    fn test_simd_search() {
        let searcher = SimdSearch::new("test");
        let content = "This is a test file";
        let matches = searcher.search_with_context(content, "test", 2);

        assert_eq!(matches.len(), 1);
        assert!(matches[0].line.contains("test"));
    }

    #[test]
    fn test_simd_performance() {
        let searcher = SimdSearch::new("pattern");
        let content = "pattern ".repeat(1000);

        let start = std::time::Instant::now();
        let matches = searcher.search_with_context(&content, "pattern", 2);
        let duration = start.elapsed();

        assert_eq!(matches.len(), 1000);
        assert!(
            duration.as_millis() < 10,
            "SIMD search too slow: {:?}",
            duration
        );
    }
}

#[cfg(test)]
mod performance_comparison {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_algorithm_performance() {
        let content = "test pattern ".repeat(10000);
        let pattern = "pattern";

        // Boyer-Moore
        let boyer_moore = BoyerMooreSearch::new(pattern);
        let start = Instant::now();
        let _bm_matches = boyer_moore.search_with_context(&content, pattern, 2);
        let bm_duration = start.elapsed();

        // SIMD
        let simd = SimdSearch::new(pattern);
        let start = Instant::now();
        let _simd_matches = simd.search_with_context(&content, pattern, 2);
        let simd_duration = start.elapsed();

        // Both should be fast, but SIMD should be faster for large content
        assert!(
            bm_duration.as_millis() < 100,
            "Boyer-Moore too slow: {:?}",
            bm_duration
        );
        assert!(
            simd_duration.as_millis() < 100,
            "SIMD too slow: {:?}",
            simd_duration
        );
    }
}
