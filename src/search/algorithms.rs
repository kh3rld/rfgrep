//! Advanced search algorithms including SIMD, Aho-Corasick, and fuzzy search
use crate::processor::SearchMatch;
use std::path::Path;

/// Trait for search algorithms
pub trait SearchAlgorithmTrait {
    fn search(&self, text: &str, pattern: &str) -> Vec<usize>;
    fn search_with_context(
        &self,
        text: &str,
        pattern: &str,
        context_lines: usize,
    ) -> Vec<SearchMatch>;
    fn name(&self) -> &'static str;
}

/// SIMD-optimized search using memchr
pub struct SimdSearch {
    pattern: Vec<u8>,
    pattern_str: String,
}

impl SimdSearch {
    pub fn new(pattern: &str) -> Self {
        Self {
            pattern: pattern.as_bytes().to_vec(),
            pattern_str: pattern.to_string(),
        }
    }
}

impl SearchAlgorithmTrait for SimdSearch {
    fn search(&self, text: &str, _pattern: &str) -> Vec<usize> {
        if self.pattern.is_empty() {
            return vec![];
        }

        let text_bytes = text.as_bytes();
        let mut matches = Vec::new();
        let mut pos = 0;

        while let Some(found_pos) = memchr::memmem::find(&text_bytes[pos..], &self.pattern) {
            let absolute_pos = pos + found_pos;
            matches.push(absolute_pos);
            pos = absolute_pos + 1;

            if pos >= text_bytes.len() {
                break;
            }
        }

        matches
    }

    fn search_with_context(
        &self,
        text: &str,
        pattern: &str,
        context_lines: usize,
    ) -> Vec<SearchMatch> {
        let matches = self.search(text, pattern);
        let lines: Vec<&str> = text.lines().collect();
        let mut results = Vec::new();

        for &match_pos in &matches {
            let pre_lines = text[..match_pos].lines().count();
            let line_number = pre_lines.max(1);
            let line_index = line_number - 1;

            if line_index < lines.len() {
                let line = lines[line_index];
                let context_before = self.get_context_before(&lines, line_index, context_lines);
                let context_after = self.get_context_after(&lines, line_index, context_lines);

                let line_start = text[..match_pos].rfind('\n').unwrap_or(0);
                let column_start = match_pos - line_start;
                let column_end = column_start + self.pattern.len();
                let matched_text = if column_start < line.len() && column_end <= line.len() {
                    line[column_start..column_end].to_string()
                } else {
                    self.pattern_str.clone()
                };

                results.push(SearchMatch {
                    path: Path::new("").to_path_buf(),
                    line_number,
                    line: line.to_string(),
                    context_before,
                    context_after,
                    matched_text,
                    column_start,
                    column_end,
                });
            }
        }

        results
    }

    fn name(&self) -> &'static str {
        "SIMD"
    }
}

impl SimdSearch {
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

/// Aho-Corasick algorithm for multiple pattern matching
pub struct AhoCorasickSearch {
    patterns: Vec<String>,
    automaton: aho_corasick::AhoCorasick,
}

impl AhoCorasickSearch {
    pub fn new(patterns: Vec<String>) -> crate::error::Result<Self> {
        let automaton = aho_corasick::AhoCorasick::new(&patterns)
            .map_err(|e| crate::error::RfgrepError::Other(format!("Aho-Corasick error: {e}")))?;

        Ok(Self {
            patterns,
            automaton,
        })
    }
}

impl SearchAlgorithmTrait for AhoCorasickSearch {
    fn search(&self, text: &str, _pattern: &str) -> Vec<usize> {
        self.automaton.find_iter(text).map(|m| m.start()).collect()
    }

    fn search_with_context(
        &self,
        text: &str,
        pattern: &str,
        context_lines: usize,
    ) -> Vec<SearchMatch> {
        let matches = self.search(text, pattern);
        let lines: Vec<&str> = text.lines().collect();
        let mut results = Vec::new();

        for &match_pos in &matches {
            let pre_lines = text[..match_pos].lines().count();
            let line_number = pre_lines.max(1);
            let line_index = line_number - 1;

            if line_index < lines.len() {
                let line = lines[line_index];
                let context_before = self.get_context_before(&lines, line_index, context_lines);
                let context_after = self.get_context_after(&lines, line_index, context_lines);

                let matched_text = self
                    .automaton
                    .find_iter(&text[match_pos..])
                    .next()
                    .map(|m| m.pattern().as_usize())
                    .and_then(|i| self.patterns.get(i))
                    .map_or(pattern, |v| v)
                    .to_string();

                results.push(SearchMatch {
                    path: Path::new("").to_path_buf(),
                    line_number,
                    line: line.to_string(),
                    context_before,
                    context_after,
                    matched_text: matched_text.clone(),
                    column_start: match_pos - text[..match_pos].rfind('\n').unwrap_or(0),
                    column_end: match_pos - text[..match_pos].rfind('\n').unwrap_or(0)
                        + matched_text.len(),
                });
            }
        }

        results
    }

    fn name(&self) -> &'static str {
        "Aho-Corasick"
    }
}

impl AhoCorasickSearch {
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

/// Fuzzy search using edit distance
pub struct FuzzySearch {
    pattern: String,
    max_distance: usize,
}

impl FuzzySearch {
    pub fn new(pattern: &str, max_distance: usize) -> Self {
        Self {
            pattern: pattern.to_string(),
            max_distance,
        }
    }

    fn levenshtein_distance(&self, s1: &str, s2: &str) -> usize {
        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();
        let s1_len = s1_chars.len();
        let s2_len = s2_chars.len();

        let mut matrix = vec![vec![0; s2_len + 1]; s1_len + 1];

        for i in 0..=s1_len {
            matrix[i][0] = i;
        }
        for j in 0..=s2_len {
            matrix[0][j] = j;
        }

        for i in 1..=s1_len {
            for j in 1..=s2_len {
                let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                    0
                } else {
                    1
                };
                matrix[i][j] = (matrix[i - 1][j] + 1)
                    .min(matrix[i][j - 1] + 1)
                    .min(matrix[i - 1][j - 1] + cost);
            }
        }

        matrix[s1_len][s2_len]
    }
}

impl SearchAlgorithmTrait for FuzzySearch {
    fn search(&self, text: &str, _pattern: &str) -> Vec<usize> {
        let mut matches = Vec::new();
        let words: Vec<&str> = text.split_whitespace().collect();

        for (i, word) in words.iter().enumerate() {
            if self.levenshtein_distance(word, &self.pattern) <= self.max_distance {
                let mut pos = 0;
                for (j, w) in words.iter().enumerate() {
                    if j == i {
                        break;
                    }
                    pos += w.len() + 1;
                }
                matches.push(pos);
            }
        }

        matches
    }

    fn search_with_context(
        &self,
        text: &str,
        pattern: &str,
        context_lines: usize,
    ) -> Vec<SearchMatch> {
        let matches = self.search(text, pattern);
        let lines: Vec<&str> = text.lines().collect();
        let mut results = Vec::new();

        for &match_pos in &matches {
            let pre_lines = text[..match_pos].lines().count();
            let line_number = pre_lines.max(1);
            let line_index = line_number - 1;

            if line_index < lines.len() {
                let line = lines[line_index];
                let context_before = self.get_context_before(&lines, line_index, context_lines);
                let context_after = self.get_context_after(&lines, line_index, context_lines);

                let line_start = text[..match_pos].rfind('\n').unwrap_or(0);
                let column_start = match_pos - line_start;
                let column_end = column_start + self.pattern.len();
                let matched_text = if column_start < line.len() && column_end <= line.len() {
                    line[column_start..column_end].to_string()
                } else {
                    self.pattern.clone()
                };

                results.push(SearchMatch {
                    path: Path::new("").to_path_buf(),
                    line_number,
                    line: line.to_string(),
                    context_before,
                    context_after,
                    matched_text,
                    column_start,
                    column_end,
                });
            }
        }

        results
    }

    fn name(&self) -> &'static str {
        "Fuzzy"
    }
}

impl FuzzySearch {
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

/// Rabin-Karp algorithm for rolling hash search
pub struct RabinKarpSearch {
    pattern: String,
    pattern_hash: u64,
    base: u64,
    mod_value: u64,
}

impl RabinKarpSearch {
    pub fn new(pattern: &str) -> Self {
        let base = 256u64;
        let mod_value = 1000000007u64;
        let pattern_hash = Self::calculate_hash(pattern, base, mod_value);

        Self {
            pattern: pattern.to_string(),
            pattern_hash,
            base,
            mod_value,
        }
    }

    fn calculate_hash(text: &str, base: u64, mod_value: u64) -> u64 {
        let mut hash = 0u64;
        for ch in text.chars() {
            hash = (hash * base + ch as u64) % mod_value;
        }
        hash
    }

    fn rolling_hash(&self, text: &str, start: usize, length: usize) -> u64 {
        let mut hash = 0u64;
        let mut power = 1u64;

        for i in 0..length {
            if start + i < text.len() {
                hash =
                    (hash + (text.chars().nth(start + i).unwrap() as u64) * power) % self.mod_value;
                if i < length - 1 {
                    power = (power * self.base) % self.mod_value;
                }
            }
        }

        hash
    }
}

impl SearchAlgorithmTrait for RabinKarpSearch {
    fn search(&self, text: &str, _pattern: &str) -> Vec<usize> {
        let mut matches = Vec::new();
        let pattern_len = self.pattern.len();
        let text_len = text.len();

        if pattern_len == 0 || pattern_len > text_len {
            return matches;
        }

        let mut text_hash = self.rolling_hash(text, 0, pattern_len);
        let mut power = 1u64;

        for _ in 0..pattern_len - 1 {
            power = (power * self.base) % self.mod_value;
        }

        for i in 0..=text_len - pattern_len {
            if text_hash == self.pattern_hash {
                if &text[i..i + pattern_len] == self.pattern {
                    matches.push(i);
                }
            }

            if i < text_len - pattern_len {
                text_hash = (text_hash + self.mod_value
                    - (text.chars().nth(i).unwrap() as u64 * power) % self.mod_value)
                    % self.mod_value;
                text_hash = (text_hash * self.base
                    + text.chars().nth(i + pattern_len).unwrap() as u64)
                    % self.mod_value;
            }
        }

        matches
    }

    fn search_with_context(
        &self,
        text: &str,
        pattern: &str,
        context_lines: usize,
    ) -> Vec<SearchMatch> {
        let matches = self.search(text, pattern);
        let lines: Vec<&str> = text.lines().collect();
        let mut results = Vec::new();

        for &match_pos in &matches {
            let pre_lines = text[..match_pos].lines().count();
            let line_number = pre_lines.max(1);
            let line_index = line_number - 1;

            if line_index < lines.len() {
                let line = lines[line_index];
                let context_before = self.get_context_before(&lines, line_index, context_lines);
                let context_after = self.get_context_after(&lines, line_index, context_lines);

                let line_start = text[..match_pos].rfind('\n').unwrap_or(0);
                let column_start = match_pos - line_start;
                let column_end = column_start + self.pattern.len();
                let matched_text = if column_start < line.len() && column_end <= line.len() {
                    line[column_start..column_end].to_string()
                } else {
                    self.pattern.clone()
                };

                results.push(SearchMatch {
                    path: Path::new("").to_path_buf(),
                    line_number,
                    line: line.to_string(),
                    context_before,
                    context_after,
                    matched_text,
                    column_start,
                    column_end,
                });
            }
        }

        results
    }

    fn name(&self) -> &'static str {
        "Rabin-Karp"
    }
}

impl RabinKarpSearch {
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
