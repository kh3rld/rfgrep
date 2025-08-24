use std::collections::HashMap;

/// Boyer-Moore string search algorithm for efficient text matching
pub struct BoyerMoore {
    pattern: Vec<u8>,
    bad_char_table: HashMap<u8, usize>,
    good_suffix_table: Vec<usize>,
}

impl BoyerMoore {
    #[allow(dead_code)]
    pub fn new(pattern: &str) -> Self {
        let pattern_bytes = pattern.as_bytes().to_vec();
        let bad_char_table = Self::build_bad_char_table(&pattern_bytes);
        let good_suffix_table = Self::build_good_suffix_table(&pattern_bytes);

        Self {
            pattern: pattern_bytes,
            bad_char_table,
            good_suffix_table,
        }
    }

    /// Build the bad character table for Boyer-Moore algorithm
    fn build_bad_char_table(pattern: &[u8]) -> HashMap<u8, usize> {
        let mut table = HashMap::new();
        let pattern_len = pattern.len();

        for (i, &byte) in pattern.iter().enumerate() {
            table.insert(byte, pattern_len - 1 - i);
        }

        table
    }

    /// Build the good suffix table for Boyer-Moore algorithm
    fn build_good_suffix_table(pattern: &[u8]) -> Vec<usize> {
        let pattern_len = pattern.len();
        let mut table = vec![1; pattern_len];

        if pattern_len > 1 {
            table[pattern_len - 2] = pattern_len;
        }

        table
    }

    /// Search for the pattern in the given text
    pub fn search(&self, text: &str) -> Vec<usize> {
        let text_bytes = text.as_bytes();
        let pattern_len = self.pattern.len();
        let text_len = text_bytes.len();
        let mut matches = Vec::new();

        if pattern_len == 0 || text_len < pattern_len {
            return matches;
        }

        let mut i = pattern_len - 1;
        while i < text_len {
            let mut j = pattern_len - 1;
            let mut k = i;

            while j > 0 && text_bytes[k] == self.pattern[j] {
                k -= 1;
                j -= 1;
            }

            if j == 0 && text_bytes[k] == self.pattern[0] {
                matches.push(k);
            }

            let bad_char_shift = self
                .bad_char_table
                .get(&text_bytes[i])
                .unwrap_or(&pattern_len);
            let good_suffix_shift = if j < pattern_len - 1 {
                self.good_suffix_table[j + 1]
            } else {
                1
            };

            let shift = bad_char_shift.max(&good_suffix_shift);
            i += shift;
        }

        matches
    }

    /// Search for all occurrences with context
    pub fn search_with_context(&self, text: &str, context_lines: usize) -> Vec<SearchMatch> {
        let matches = self.search(text);
        let lines: Vec<&str> = text.lines().collect();
        let mut results = Vec::new();

        for &match_pos in &matches {
            let line_number = text[..match_pos].lines().count() + 1;
            let line_index = line_number - 1;

            if line_index < lines.len() {
                let line = lines[line_index];
                let context_before = self.get_context_before(&lines, line_index, context_lines);
                let context_after = self.get_context_after(&lines, line_index, context_lines);

                results.push(SearchMatch {
                    line_number,
                    line: line.to_string(),
                    context_before,
                    context_after,
                    matched_text: self.pattern.iter().map(|&b| b as char).collect(),
                    column_start: match_pos - text[..match_pos].rfind('\n').unwrap_or(0),
                    column_end: match_pos - text[..match_pos].rfind('\n').unwrap_or(0)
                        + self.pattern.len(),
                });
            }
        }

        results
    }
}

/// Search match result with context
#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub line_number: usize,
    pub line: String,
    pub context_before: Vec<(usize, String)>,
    pub context_after: Vec<(usize, String)>,
    pub matched_text: String,
    pub column_start: usize,
    pub column_end: usize,
}

/// Search algorithm types
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum SearchAlgorithm {
    BoyerMoore,
    Regex,
    Simple,
}

/// Search algorithm factory
#[allow(dead_code)]
pub struct SearchAlgorithmFactory;

impl SearchAlgorithmFactory {
    #[allow(dead_code)]
    pub fn create(algorithm: SearchAlgorithm, pattern: &str) -> Box<dyn SearchAlgorithmTrait> {
        match algorithm {
            SearchAlgorithm::BoyerMoore => Box::new(BoyerMoore::new(pattern)),
            SearchAlgorithm::Regex => Box::new(RegexSearch::new(pattern)),
            SearchAlgorithm::Simple => Box::new(SimpleSearch::new(pattern)),
        }
    }
}

/// Trait for search algorithms
pub trait SearchAlgorithmTrait {
    fn search(&self, text: &str) -> Vec<usize>;
    #[allow(dead_code)]
    fn search_with_context(&self, text: &str, context_lines: usize) -> Vec<SearchMatch>;

    fn get_context_before(
        &self,
        lines: &[&str],
        current_line: usize,
        context_lines: usize,
    ) -> Vec<(usize, String)> {
        let start = current_line.saturating_sub(context_lines);
        (start..current_line)
            .map(|i| (i + 1, lines[i].to_string()))
            .collect()
    }

    fn get_context_after(
        &self,
        lines: &[&str],
        current_line: usize,
        context_lines: usize,
    ) -> Vec<(usize, String)> {
        let end = (current_line + context_lines + 1).min(lines.len());
        ((current_line + 1)..end)
            .map(|i| (i + 1, lines[i].to_string()))
            .collect()
    }
}

impl SearchAlgorithmTrait for BoyerMoore {
    fn search(&self, text: &str) -> Vec<usize> {
        self.search(text)
    }

    fn search_with_context(&self, text: &str, context_lines: usize) -> Vec<SearchMatch> {
        self.search_with_context(text, context_lines)
    }
}

/// Simple text search implementation
pub struct SimpleSearch {
    pattern: String,
}

impl SimpleSearch {
    #[allow(dead_code)]
    pub fn new(pattern: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
        }
    }
}

impl SearchAlgorithmTrait for SimpleSearch {
    fn search(&self, text: &str) -> Vec<usize> {
        let mut matches = Vec::new();
        let mut start = 0;

        while let Some(pos) = text[start..].find(&self.pattern) {
            matches.push(start + pos);
            start += pos + 1;
        }

        matches
    }

    fn search_with_context(&self, text: &str, context_lines: usize) -> Vec<SearchMatch> {
        let matches = self.search(text);
        let lines: Vec<&str> = text.lines().collect();
        let mut results = Vec::new();

        for &match_pos in &matches {
            let line_number = text[..match_pos].lines().count() + 1;
            let line_index = line_number - 1;

            if line_index < lines.len() {
                let line = lines[line_index];
                let context_before = self.get_context_before(&lines, line_index, context_lines);
                let context_after = self.get_context_after(&lines, line_index, context_lines);

                results.push(SearchMatch {
                    line_number,
                    line: line.to_string(),
                    context_before,
                    context_after,
                    matched_text: self.pattern.clone(),
                    column_start: match_pos - text[..match_pos].rfind('\n').unwrap_or(0),
                    column_end: match_pos - text[..match_pos].rfind('\n').unwrap_or(0)
                        + self.pattern.len(),
                });
            }
        }

        results
    }
}

/// Regex search implementation
pub struct RegexSearch {
    pattern: regex::Regex,
}

impl RegexSearch {
    #[allow(dead_code)]
    pub fn new(pattern: &str) -> Self {
        Self {
            pattern: regex::Regex::new(pattern).expect("Invalid regex pattern"),
        }
    }
}

impl SearchAlgorithmTrait for RegexSearch {
    fn search(&self, text: &str) -> Vec<usize> {
        self.pattern.find_iter(text).map(|m| m.start()).collect()
    }

    fn search_with_context(&self, text: &str, context_lines: usize) -> Vec<SearchMatch> {
        let matches = self.search(text);
        let lines: Vec<&str> = text.lines().collect();
        let mut results = Vec::new();

        for &match_pos in &matches {
            let line_number = text[..match_pos].lines().count() + 1;
            let line_index = line_number - 1;

            if line_index < lines.len() {
                let line = lines[line_index];
                let context_before = self.get_context_before(&lines, line_index, context_lines);
                let context_after = self.get_context_after(&lines, line_index, context_lines);

                if let Some(m) = self.pattern.find(line) {
                    results.push(SearchMatch {
                        line_number,
                        line: line.to_string(),
                        context_before,
                        context_after,
                        matched_text: m.as_str().to_string(),
                        column_start: m.start(),
                        column_end: m.end(),
                    });
                }
            }
        }

        results
    }
}
