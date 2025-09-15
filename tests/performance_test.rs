use rfgrep::search_algorithms::*;

#[test]
fn test_simd_search_performance() {
    let text = "This is a test text with multiple occurrences of the word test. Test test test!";
    let pattern = "test";

    let simd_search = SimdSearch::new(pattern);
    let boyer_moore = BoyerMoore::new(pattern);
    let simple_search = SimpleSearch::new(pattern);

    let simd_matches = simd_search.search(text, pattern);
    let bm_matches = boyer_moore.search(text, pattern);
    let simple_matches = simple_search.search(text, pattern);

    // All algorithms should find the same matches
    assert_eq!(simd_matches, bm_matches);
    assert_eq!(simd_matches, simple_matches);

    // Should find 4 occurrences of "test"
    assert_eq!(simd_matches.len(), 4);

    // Verify positions (case-sensitive search)
    let expected_positions = vec![10, 58, 69, 74];
    assert_eq!(simd_matches, expected_positions);
}

#[test]
fn test_search_with_context() {
    let text = "Line 1\nLine 2\nLine 3 with pattern\nLine 4\nLine 5";
    let pattern = "pattern";

    let simd_search = SimdSearch::new(pattern);
    let matches = simd_search.search_with_context(text, pattern, 1);

    assert_eq!(matches.len(), 1);
    let match_result = &matches[0];

    assert_eq!(match_result.line_number, 3);
    assert_eq!(match_result.line, "Line 3 with pattern");
    assert_eq!(match_result.matched_text, "pattern");

    // Check context
    assert_eq!(match_result.context_before.len(), 1);
    assert_eq!(match_result.context_before[0].1, "Line 2");

    assert_eq!(match_result.context_after.len(), 1);
    assert_eq!(match_result.context_after[0].1, "Line 4");
}

#[test]
fn test_algorithm_factory() {
    let pattern = "test";

    let simd = SearchAlgorithmFactory::create(SearchAlgorithm::Simd, pattern);
    let boyer_moore = SearchAlgorithmFactory::create(SearchAlgorithm::BoyerMoore, pattern);
    let regex = SearchAlgorithmFactory::create(SearchAlgorithm::Regex, pattern);
    let simple = SearchAlgorithmFactory::create(SearchAlgorithm::Simple, pattern);

    let text = "test text test";

    let simd_matches = simd.search(text, pattern);
    let bm_matches = boyer_moore.search(text, pattern);
    let regex_matches = regex.search(text, pattern);
    let simple_matches = simple.search(text, pattern);

    // All should find the same matches
    assert_eq!(simd_matches, bm_matches);
    assert_eq!(simd_matches, regex_matches);
    assert_eq!(simd_matches, simple_matches);

    // Should find 2 occurrences
    assert_eq!(simd_matches.len(), 2);
}

#[test]
fn test_regex_search() {
    let pattern = r"\btest\b"; // Word boundary
    let regex_search = RegexSearch::new(pattern);

    let text = "test text testing tested";

    let matches = regex_search.search(text, pattern);

    // Should only match "test" as a word, not "testing" or "tested"
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0], 0); // "test" at position 0
}

#[test]
fn test_case_insensitive_search() {
    let text = "Test TEST test TeSt";
    let pattern = "test";

    let simd_search = SimdSearch::new(pattern);
    let matches = simd_search.search(text, pattern);

    // Should find only exact matches (case-sensitive)
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0], 10); // "test" at position 10
}
