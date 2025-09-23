//! Zero-copy string processing utilities for rfgrep
use crate::error::{Result as RfgrepResult, RfgrepError};
use memmap2::Mmap;
use regex::Regex;
use std::path::PathBuf;

/// Zero-copy search match that uses string slices instead of owned strings
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ZeroCopyMatch<'a> {
    pub path: PathBuf,
    pub line_number: usize,
    pub column: usize,
    pub matched_text: &'a str,
    pub context_before: Vec<&'a str>,
    pub context_after: Vec<&'a str>,
}

/// Zero-copy search result that holds references to the original content
pub struct ZeroCopySearchResult<'a> {
    pub content: &'a str,
    pub matches: Vec<ZeroCopyMatch<'a>>,
}

impl<'a> ZeroCopySearchResult<'a> {
    /// Create a new zero-copy search result
    pub fn new(content: &'a str) -> Self {
        Self {
            content,
            matches: Vec::new(),
        }
    }

    /// Add a match to the result
    pub fn add_match(
        &mut self,
        path: PathBuf,
        line_number: usize,
        column: usize,
        matched_text: &'a str,
        context_before: Vec<&'a str>,
        context_after: Vec<&'a str>,
    ) {
        self.matches.push(ZeroCopyMatch {
            path,
            line_number,
            column,
            matched_text,
            context_before,
            context_after,
        });
    }

    /// Get the number of matches
    pub fn len(&self) -> usize {
        self.matches.len()
    }

    /// Check if there are any matches
    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }
}

/// Zero-copy string processor for efficient text processing
pub struct ZeroCopyProcessor<'a> {
    content: &'a str,
    lines: Vec<&'a str>,
}

impl<'a> ZeroCopyProcessor<'a> {
    /// Create a new zero-copy processor
    pub fn new(content: &'a str) -> Self {
        let lines = content.lines().collect();
        Self { content, lines }
    }

    /// Search for pattern using zero-copy string processing
    pub fn search_with_context(
        &self,
        pattern: &Regex,
        path: PathBuf,
        context_lines: usize,
    ) -> RfgrepResult<ZeroCopySearchResult<'a>> {
        let mut result = ZeroCopySearchResult::new(self.content);

        for (line_idx, line) in self.lines.iter().enumerate() {
            for mat in pattern.find_iter(line) {
                let line_number = line_idx + 1;
                let column = mat.start() + 1;
                let matched_text = &line[mat.start()..mat.end()];

                // Get context lines
                let context_before = self.get_context_before(line_idx, context_lines);
                let context_after = self.get_context_after(line_idx, context_lines);

                result.add_match(
                    path.clone(),
                    line_number,
                    column,
                    matched_text,
                    context_before,
                    context_after,
                );
            }
        }

        Ok(result)
    }

    /// Get context lines before the current line
    fn get_context_before(&self, line_idx: usize, context_lines: usize) -> Vec<&'a str> {
        let start = line_idx.saturating_sub(context_lines);

        self.lines[start..line_idx].to_vec()
    }

    /// Get context lines after the current line
    fn get_context_after(&self, line_idx: usize, context_lines: usize) -> Vec<&'a str> {
        let end = if line_idx + context_lines + 1 < self.lines.len() {
            line_idx + context_lines + 1
        } else {
            self.lines.len()
        };

        self.lines[line_idx + 1..end].to_vec()
    }

    /// Find all line boundaries in the content
    pub fn find_line_boundaries(&self) -> Vec<usize> {
        let mut boundaries = Vec::new();
        let mut pos = 0;

        for line in self.lines.iter() {
            boundaries.push(pos);
            pos += line.len() + 1;
        }

        boundaries
    }

    /// Get a specific line by index
    pub fn get_line(&self, line_idx: usize) -> Option<&'a str> {
        self.lines.get(line_idx).copied()
    }

    /// Get the total number of lines
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Check if content is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get content as bytes
    pub fn as_bytes(&self) -> &[u8] {
        self.content.as_bytes()
    }

    /// Get content length
    pub fn len(&self) -> usize {
        self.content.len()
    }
}

/// Memory-mapped zero-copy processor for large files
pub struct MmapZeroCopyProcessor<'a> {
    mmap: &'a Mmap,
    content: &'a str,
    processor: ZeroCopyProcessor<'a>,
}

impl<'a> MmapZeroCopyProcessor<'a> {
    /// Create a new memory-mapped zero-copy processor
    pub fn new(mmap: &'a Mmap) -> RfgrepResult<Self> {
        let content = std::str::from_utf8(mmap).map_err(|e| {
            RfgrepError::Other(format!("Invalid UTF-8 in memory-mapped file: {}", e))
        })?;

        let processor = ZeroCopyProcessor::new(content);

        Ok(Self {
            mmap,
            content,
            processor,
        })
    }

    /// Search using zero-copy processing
    pub fn search_with_context(
        &self,
        pattern: &Regex,
        path: PathBuf,
        context_lines: usize,
    ) -> RfgrepResult<ZeroCopySearchResult<'a>> {
        self.processor
            .search_with_context(pattern, path, context_lines)
    }

    /// Get the underlying processor
    pub fn processor(&self) -> &ZeroCopyProcessor<'a> {
        &self.processor
    }
}

/// Zero-copy string utilities
pub struct ZeroCopyUtils;

impl ZeroCopyUtils {
    /// Split string into lines without allocation
    pub fn lines(content: &str) -> impl Iterator<Item = &str> {
        content.lines()
    }

    /// Find all occurrences of a pattern without allocation
    pub fn find_all<'a>(content: &'a str, pattern: &Regex) -> Vec<&'a str> {
        pattern
            .find_iter(content)
            .map(|mat| &content[mat.start()..mat.end()])
            .collect()
    }

    /// Extract context around a match without allocation
    pub fn extract_context(
        content: &str,
        start: usize,
        end: usize,
        context_chars: usize,
    ) -> (&str, &str, &str) {
        let context_start = start.saturating_sub(context_chars);
        let context_end = (end + context_chars).min(content.len());

        let before = &content[context_start..start];
        let matched = &content[start..end];
        let after = &content[end..context_end];

        (before, matched, after)
    }

    /// Count occurrences of a pattern without allocation
    pub fn count_matches(content: &str, pattern: &Regex) -> usize {
        pattern.find_iter(content).count()
    }

    /// Check if content contains a pattern without allocation
    pub fn contains_pattern(content: &str, pattern: &Regex) -> bool {
        pattern.is_match(content)
    }
}

/// Convert zero-copy matches to owned matches (when needed)
pub fn to_owned_matches<'a>(
    zero_copy_matches: &[ZeroCopyMatch<'a>],
) -> Vec<crate::processor::SearchMatch> {
    zero_copy_matches
        .iter()
        .map(|m| crate::processor::SearchMatch {
            path: m.path.clone(),
            line_number: m.line_number,
            line: m.matched_text.to_string(), // This should be the full line content
            column_start: m.column,
            column_end: m.column + m.matched_text.len(),
            matched_text: m.matched_text.to_string(),
            context_before: m
                .context_before
                .iter()
                .enumerate()
                .map(|(i, s)| (i + 1, s.to_string()))
                .collect(),
            context_after: m
                .context_after
                .iter()
                .enumerate()
                .map(|(i, s)| (i + 1, s.to_string()))
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    #[test]
    fn test_zero_copy_processor() {
        let content = "line 1\nline 2 with pattern\nline 3";
        let processor = ZeroCopyProcessor::new(content);

        assert_eq!(processor.line_count(), 3);
        assert_eq!(processor.len(), content.len());
    }

    #[test]
    fn test_zero_copy_search() {
        let content = "line 1\nline 2 with pattern\nline 3";
        let processor = ZeroCopyProcessor::new(content);
        let pattern = Regex::new("pattern").unwrap();
        let path = PathBuf::from("test.txt");

        let result = processor.search_with_context(&pattern, path, 1).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result.matches[0].matched_text, "pattern");
        assert_eq!(result.matches[0].line_number, 2);
    }

    #[test]
    fn test_zero_copy_utils() {
        let content = "test pattern test";
        let pattern = Regex::new("pattern").unwrap();

        let matches = ZeroCopyUtils::find_all(content, &pattern);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], "pattern");

        assert_eq!(ZeroCopyUtils::count_matches(content, &pattern), 1);
        assert!(ZeroCopyUtils::contains_pattern(content, &pattern));
    }

    #[test]
    fn test_context_extraction() {
        let content = "before pattern after";
        let (before, matched, after) = ZeroCopyUtils::extract_context(content, 7, 14, 3);

        assert_eq!(before, "re "); // "before" -> "re " (last 3 chars before position 7)
        assert_eq!(matched, "pattern");
        assert_eq!(after, " af"); // "after" -> " af" (first 3 chars after position 14)
    }
}
