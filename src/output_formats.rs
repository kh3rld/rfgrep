use crate::search_algorithms::SearchMatch;
use serde_json::{Value, json};
use std::path::Path;

/// Output format types
#[derive(Debug, Clone)]
pub enum OutputFormat {
    Text,
    Json,
    Xml,
    Html,
    Markdown,
}

/// Output formatter for different formats
pub struct OutputFormatter {
    format: OutputFormat,
    include_metadata: bool,
    include_context: bool,
}

impl OutputFormatter {
    pub fn new(format: OutputFormat) -> Self {
        Self {
            format,
            include_metadata: true,
            include_context: true,
        }
    }

    pub fn with_metadata(mut self, include: bool) -> Self {
        self.include_metadata = include;
        self
    }

    pub fn with_context(mut self, include: bool) -> Self {
        self.include_context = include;
        self
    }

    /// Format search results
    pub fn format_results(&self, matches: &[SearchMatch], query: &str, path: &Path) -> String {
        match self.format {
            OutputFormat::Text => self.format_text(matches, query, path),
            OutputFormat::Json => self.format_json(matches, query, path),
            OutputFormat::Xml => self.format_xml(matches, query, path),
            OutputFormat::Html => self.format_html(matches, query, path),
            OutputFormat::Markdown => self.format_markdown(matches, query, path),
        }
    }

    /// Format as JSON
    fn format_json(&self, matches: &[SearchMatch], query: &str, path: &Path) -> String {
        let mut result = json!({
            "query": query,
            "path": path.to_string_lossy(),
            "total_matches": matches.len(),
            "matches": []
        });

        let matches_array = result["matches"].as_array_mut().unwrap();

        for m in matches {
            let mut match_obj = json!({
                "line_number": m.line_number,
                "line": m.line,
                "matched_text": m.matched_text,
                "column_start": m.column_start,
                "column_end": m.column_end,
            });

            if self.include_context {
                let context_before: Vec<Value> = m
                    .context_before
                    .iter()
                    .map(|(num, line)| {
                        json!({
                            "line_number": num,
                            "content": line
                        })
                    })
                    .collect();

                let context_after: Vec<Value> = m
                    .context_after
                    .iter()
                    .map(|(num, line)| {
                        json!({
                            "line_number": num,
                            "content": line
                        })
                    })
                    .collect();

                match_obj["context_before"] = Value::Array(context_before);
                match_obj["context_after"] = Value::Array(context_after);
            }

            matches_array.push(match_obj);
        }

        serde_json::to_string_pretty(&result).unwrap()
    }

    /// Format as plain text (default)
    fn format_text(&self, matches: &[SearchMatch], query: &str, path: &Path) -> String {
        let mut output = String::new();

        if self.include_metadata {
            output.push_str(&format!("Query: {query}\n"));
            output.push_str(&format!("Path: {}\n", path.display()));
            output.push_str(&format!("Total matches: {}\n\n", matches.len()));
        }

        for (i, m) in matches.iter().enumerate() {
            output.push_str(&format!("[{}]\n", i + 1));

            if self.include_context {
                for (num, line) in &m.context_before {
                    output.push_str(&format!("  {num} │ {line}\n"));
                }
            }

            // Highlight the match
            let line_len = m.line.len();
            let column_start = m.column_start.min(line_len);
            let column_end = m.column_end.min(line_len);

            let before = if column_start < line_len {
                &m.line[..column_start]
            } else {
                ""
            };
            let matched = &m.matched_text;
            let after = if column_end < line_len {
                &m.line[column_end..]
            } else {
                ""
            };

            output.push_str(&format!(
                "→ {} │ {}{}{}\n",
                m.line_number, before, matched, after
            ));

            if self.include_context {
                for (num, line) in &m.context_after {
                    output.push_str(&format!("  {num} │ {line}\n"));
                }
            }
            output.push('\n');
        }

        output
    }

    /// Format as XML
    fn format_xml(&self, matches: &[SearchMatch], query: &str, path: &Path) -> String {
        let mut output = String::new();
        output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        output.push_str("<search-results>\n");

        if self.include_metadata {
            output.push_str("  <metadata>\n");
            output.push_str(&format!("    <query>{}</query>\n", escape_xml(query)));
            output.push_str(&format!(
                "    <path>{}</path>\n",
                escape_xml(&path.to_string_lossy())
            ));
            output.push_str(&format!(
                "    <total-matches>{}</total-matches>\n",
                matches.len()
            ));
            output.push_str("  </metadata>\n");
        }

        output.push_str("  <matches>\n");

        for (i, m) in matches.iter().enumerate() {
            output.push_str(&format!("    <match index=\"{}\">\n", i + 1));
            output.push_str(&format!(
                "      <line-number>{}</line-number>\n",
                m.line_number
            ));
            output.push_str(&format!("      <line>{}</line>\n", escape_xml(&m.line)));
            output.push_str(&format!(
                "      <matched-text>{}</matched-text>\n",
                escape_xml(&m.matched_text)
            ));
            output.push_str(&format!(
                "      <column-start>{}</column-start>\n",
                m.column_start
            ));
            output.push_str(&format!(
                "      <column-end>{}</column-end>\n",
                m.column_end
            ));
            output.push_str("    </match>\n");
        }

        output.push_str("  </matches>\n");
        output.push_str("</search-results>\n");

        output
    }

    /// Format as HTML
    fn format_html(&self, matches: &[SearchMatch], query: &str, path: &Path) -> String {
        let mut output = String::new();
        output.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        output.push_str("<meta charset=\"UTF-8\">\n");
        output.push_str("<title>rfgrep Search Results</title>\n");
        output.push_str("<style>\n");
        output.push_str("body { font-family: monospace; margin: 20px; }\n");
        output.push_str(
            ".match { margin: 10px 0; padding: 10px; border-left: 3px solid #007acc; }\n",
        );
        output.push_str(".line-number { color: #666; }\n");
        output.push_str(".matched-text { background-color: #ffff00; font-weight: bold; }\n");
        output.push_str(".context { color: #888; }\n");
        output.push_str(
            ".metadata { background-color: #f5f5f5; padding: 10px; margin-bottom: 20px; }\n",
        );
        output.push_str("</style>\n</head>\n<body>\n");

        if self.include_metadata {
            output.push_str("<div class=\"metadata\">\n");
            output.push_str("<h2>Search Results</h2>\n");
            output.push_str(&format!(
                "<p><strong>Query:</strong> {}</p>\n",
                escape_html(query)
            ));
            output.push_str(&format!(
                "<p><strong>Path:</strong> {}</p>\n",
                escape_html(&path.to_string_lossy())
            ));
            output.push_str(&format!(
                "<p><strong>Total Matches:</strong> {}</p>\n",
                matches.len()
            ));
            output.push_str("</div>\n");
        }

        for (i, m) in matches.iter().enumerate() {
            output.push_str("<div class=\"match\">\n");
            output.push_str(&format!("<h3>Match {}</h3>\n", i + 1));

            // Highlight the match
            let line_len = m.line.len();
            let column_start = m.column_start.min(line_len);
            let column_end = m.column_end.min(line_len);

            let before = if column_start < line_len {
                &m.line[..column_start]
            } else {
                ""
            };
            let matched = &m.matched_text;
            let after = if column_end < line_len {
                &m.line[column_end..]
            } else {
                ""
            };

            output.push_str("<div>");
            output.push_str(&format!(
                "<span class=\"line-number\">→ {:>4}</span> │ {}{}{}",
                m.line_number,
                escape_html(before),
                format!(
                    "<span class=\"matched-text\">{}</span>",
                    escape_html(matched)
                ),
                escape_html(after)
            ));
            output.push_str("</div>\n");
            output.push_str("</div>\n");
        }

        output.push_str("</body>\n</html>\n");

        output
    }

    /// Format as Markdown
    fn format_markdown(&self, matches: &[SearchMatch], query: &str, path: &Path) -> String {
        let mut output = String::new();

        output.push_str("# rfgrep Search Results\n\n");

        if self.include_metadata {
            output.push_str(&format!("**Query:** `{query}`\n"));
            output.push_str(&format!("**Path:** `{}`\n", path.display()));
            output.push_str(&format!("**Total Matches:** {}\n\n", matches.len()));
        }

        for (i, m) in matches.iter().enumerate() {
            output.push_str(&format!("## Match {}\n\n", i + 1));

            // Highlight the match
            let line_len = m.line.len();
            let column_start = m.column_start.min(line_len);
            let column_end = m.column_end.min(line_len);

            let before = if column_start < line_len {
                &m.line[..column_start]
            } else {
                ""
            };
            let matched = &m.matched_text;
            let after = if column_end < line_len {
                &m.line[column_end..]
            } else {
                ""
            };

            output.push_str("**Match:**\n");
            output.push_str("```\n");
            output.push_str(&format!(
                "→ {:>4} │ {}{}{}\n",
                m.line_number, before, matched, after
            ));
            output.push_str("```\n\n");
        }

        output
    }
}

/// Escape XML special characters
fn escape_xml(s: &str) -> String {
    s.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;")
}

/// Escape HTML special characters
fn escape_html(s: &str) -> String {
    s.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&#39;")
}
