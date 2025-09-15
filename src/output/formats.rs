//! Individual output format implementations
use crate::processor::SearchMatch;
use crate::output::{OutputFormatterTrait, OutputFormat};
use std::path::Path;
use serde_json::json;

/// Text formatter (default)
pub struct TextFormatter {
    include_metadata: bool,
    include_context: bool,
    use_color: bool,
}

impl TextFormatter {
    pub fn new() -> Self {
        Self {
            include_metadata: true,
            include_context: true,
            use_color: is_terminal::is_terminal(&std::io::stdout()),
        }
    }
}

impl OutputFormatterTrait for TextFormatter {
    fn format(&self, matches: &[SearchMatch], query: &str, path: &Path) -> String {
        let mut output = String::new();
        
        if self.include_metadata {
            output.push_str(&format!("Query: {query}\n"));
            output.push_str(&format!("Path: {}\n", path.display()));
            output.push_str(&format!("Total matches: {}\n\n", matches.len()));
        }

        for m in matches {
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

            if self.use_color {
                let highlighted = format!("\x1b[33m{matched}\x1b[0m");
                output.push_str(&format!(
                    "{}:{}:{}: {before}{highlighted}{after}\n",
                    m.path.display(),
                    m.line_number,
                    column_start + 1
                ));
            } else {
                output.push_str(&format!(
                    "{}:{}:{}: {before}{matched}{after}\n",
                    m.path.display(),
                    m.line_number,
                    column_start + 1
                ));
            }

            if self.include_context && (!m.context_before.is_empty() || !m.context_after.is_empty()) {
                output.push_str("-- context --\n");
                for (num, line) in &m.context_before {
                    output.push_str(&format!("  {num} │ {line}\n"));
                }
                output.push_str(&format!("→ {} │ {before}{matched}{after}\n", m.line_number));
                for (num, line) in &m.context_after {
                    output.push_str(&format!("  {num} │ {line}\n"));
                }
                output.push('\n');
            }
        }

        output
    }

    fn name(&self) -> &str { "text" }
}

/// JSON formatter
pub struct JsonFormatter {
    pretty: bool,
    ndjson: bool,
}

impl JsonFormatter {
    pub fn new() -> Self {
        Self {
            pretty: true,
            ndjson: false,
        }
    }
}

impl OutputFormatterTrait for JsonFormatter {
    fn format(&self, matches: &[SearchMatch], query: &str, path: &Path) -> String {
        self.format_with_timing(matches, query, path, 0.0)
    }

    fn format_with_timing(&self, matches: &[SearchMatch], query: &str, path: &Path, execution_time_ms: f64) -> String {
        if self.ndjson {
            let mut output = String::new();
            for m in matches {
                let mut match_obj = json!({
                    "query": query,
                    "path": m.path.to_string_lossy(),
                    "line_number": m.line_number,
                    "line": m.line,
                    "matched_text": m.matched_text,
                    "column_start": m.column_start,
                    "column_end": m.column_end,
                    "context_before": m.context_before,
                    "context_after": m.context_after,
                });
                
                if execution_time_ms > 0.0 {
                    match_obj["execution_time_ms"] = json!(execution_time_ms);
                }
                
                if let Ok(s) = serde_json::to_string(&match_obj) {
                    output.push_str(&s);
                    output.push('\n');
                }
            }
            output
        } else {
            let mut result = json!({
                "query": query,
                "path": path.to_string_lossy(),
                "total_matches": matches.len(),
                "matches": matches.iter().map(|m| json!({
                    "path": m.path.to_string_lossy(),
                    "line_number": m.line_number,
                    "line": m.line,
                    "matched_text": m.matched_text,
                    "column_start": m.column_start,
                    "column_end": m.column_end,
                    "context_before": m.context_before,
                    "context_after": m.context_after,
                })).collect::<Vec<_>>()
            });

            if execution_time_ms > 0.0 {
                result["execution_time_ms"] = json!(execution_time_ms);
            }

            if self.pretty {
                serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
            } else {
                serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string())
            }
        }
    }

    fn name(&self) -> &str { "json" }
    fn supports_streaming(&self) -> bool { true }
}

/// XML formatter
pub struct XmlFormatter;

impl XmlFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl OutputFormatterTrait for XmlFormatter {
    fn format(&self, matches: &[SearchMatch], query: &str, path: &Path) -> String {
        let mut output = String::new();
        output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        output.push_str("<search-results>\n");
        output.push_str("  <metadata>\n");
        output.push_str(&format!("    <query>{}</query>\n", escape_xml(query)));
        output.push_str(&format!("    <path>{}</path>\n", escape_xml(&path.to_string_lossy())));
        output.push_str(&format!("    <total-matches>{}</total-matches>\n", matches.len()));
        output.push_str("  </metadata>\n");
        output.push_str("  <matches>\n");

        for (i, m) in matches.iter().enumerate() {
            output.push_str(&format!("    <match index=\"{}\">\n", i + 1));
            output.push_str(&format!("      <path>{}</path>\n", escape_xml(&m.path.to_string_lossy())));
            output.push_str(&format!("      <line-number>{}</line-number>\n", m.line_number));
            output.push_str(&format!("      <line>{}</line>\n", escape_xml(&m.line)));
            output.push_str(&format!("      <matched-text>{}</matched-text>\n", escape_xml(&m.matched_text)));
            output.push_str(&format!("      <column-start>{}</column-start>\n", m.column_start));
            output.push_str(&format!("      <column-end>{}</column-end>\n", m.column_end));
            output.push_str("    </match>\n");
        }

        output.push_str("  </matches>\n");
        output.push_str("</search-results>\n");
        output
    }

    fn name(&self) -> &str { "xml" }
}

/// HTML formatter
pub struct HtmlFormatter;

impl HtmlFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl OutputFormatterTrait for HtmlFormatter {
    fn format(&self, matches: &[SearchMatch], query: &str, path: &Path) -> String {
        let mut output = String::new();
        output.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        output.push_str("<meta charset=\"UTF-8\">\n");
        output.push_str("<title>rfgrep Search Results</title>\n");
        output.push_str("<style>\n");
        output.push_str("body { font-family: monospace; margin: 20px; }\n");
        output.push_str(".match { margin: 10px 0; padding: 10px; border-left: 3px solid #007acc; }\n");
        output.push_str(".line-number { color: #666; }\n");
        output.push_str(".matched-text { background-color: #ffff00; font-weight: bold; }\n");
        output.push_str(".context { color: #888; }\n");
        output.push_str(".metadata { background-color: #f5f5f5; padding: 10px; margin-bottom: 20px; }\n");
        output.push_str("</style>\n</head>\n<body>\n");

        output.push_str("<div class=\"metadata\">\n");
        output.push_str("<h2>Search Results</h2>\n");
        output.push_str(&format!("<p><strong>Query:</strong> {}</p>\n", escape_html(query)));
        output.push_str(&format!("<p><strong>Path:</strong> {}</p>\n", escape_html(&path.to_string_lossy())));
        output.push_str(&format!("<p><strong>Total Matches:</strong> {}</p>\n", matches.len()));
        output.push_str("</div>\n");

        for (i, m) in matches.iter().enumerate() {
            output.push_str("<div class=\"match\">\n");
            output.push_str(&format!("<h3>Match {}</h3>\n", i + 1));

            let line_len = m.line.len();
            let column_start = m.column_start.min(line_len);
            let column_end = m.column_end.min(line_len);

            let before = if column_start < line_len {
                &m.line[..column_start]
            } else {
                ""
            };
            let matched_text = &m.matched_text;
            let after = if column_end < line_len {
                &m.line[column_end..]
            } else {
                ""
            };

            output.push_str("<div>");
            let matched_html = format!(
                "<span class=\"matched-text\">{}</span>",
                escape_html(matched_text)
            );
            output.push_str(&format!(
                "<span class=\"line-number\">→ {:>4}</span> │ {}{}{}",
                m.line_number,
                escape_html(before),
                matched_html,
                escape_html(after)
            ));
            output.push_str("</div>\n");
            output.push_str("</div>\n");
        }

        output.push_str("</body>\n</html>\n");
        output
    }

    fn name(&self) -> &str { "html" }
}

/// Markdown formatter
pub struct MarkdownFormatter;

impl MarkdownFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl OutputFormatterTrait for MarkdownFormatter {
    fn format(&self, matches: &[SearchMatch], query: &str, path: &Path) -> String {
        let mut output = String::new();
        output.push_str("# rfgrep Search Results\n\n");
        output.push_str(&format!("**Query:** `{query}`\n"));
        output.push_str(&format!("**Path:** `{}`\n", path.display()));
        output.push_str(&format!("**Total Matches:** {}\n\n", matches.len()));

        for (i, m) in matches.iter().enumerate() {
            output.push_str(&format!("## Match {}\n\n", i + 1));

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
                "→ {:>4} │ {before}{matched}{after}\n",
                m.line_number
            ));
            output.push_str("```\n\n");
        }

        output
    }

    fn name(&self) -> &str { "markdown" }
}

/// CSV formatter
pub struct CsvFormatter;

impl CsvFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl OutputFormatterTrait for CsvFormatter {
    fn format(&self, matches: &[SearchMatch], _query: &str, _path: &Path) -> String {
        let mut output = String::new();
        output.push_str("file,line_number,column_start,column_end,matched_text,line\n");

        for m in matches {
            let escaped_line = m.line.replace('"', "\"\"");
            let escaped_matched = m.matched_text.replace('"', "\"\"");
            
            output.push_str(&format!(
                "\"{}\",{},{},{},\"{}\",\"{}\"\n",
                m.path.display(),
                m.line_number,
                m.column_start,
                m.column_end,
                escaped_matched,
                escaped_line
            ));
        }

        output
    }

    fn name(&self) -> &str { "csv" }
}

/// YAML formatter
pub struct YamlFormatter;

impl YamlFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl OutputFormatterTrait for YamlFormatter {
    fn format(&self, matches: &[SearchMatch], query: &str, path: &Path) -> String {
        let result = serde_yaml::to_string(&json!({
            "query": query,
            "path": path.to_string_lossy(),
            "total_matches": matches.len(),
            "matches": matches.iter().map(|m| json!({
                "path": m.path.to_string_lossy(),
                "line_number": m.line_number,
                "line": m.line,
                "matched_text": m.matched_text,
                "column_start": m.column_start,
                "column_end": m.column_end,
                "context_before": m.context_before,
                "context_after": m.context_after,
            })).collect::<Vec<_>>()
        })).unwrap_or_else(|_| "{}".to_string());

        result
    }

    fn name(&self) -> &str { "yaml" }
}

/// JUnit XML formatter for CI integration
pub struct JunitFormatter;

impl JunitFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl OutputFormatterTrait for JunitFormatter {
    fn format(&self, matches: &[SearchMatch], query: &str, _path: &Path) -> String {
        let mut output = String::new();
        output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        output.push_str(&format!(
            "<testsuite name=\"rfgrep\" tests=\"{}\" failures=\"{}\" time=\"0.0\">\n",
            matches.len(),
            matches.len()
        ));

        for (i, m) in matches.iter().enumerate() {
            output.push_str(&format!(
                "  <testcase name=\"match_{}\" classname=\"{}\">\n",
                i + 1,
                escape_xml(&m.path.to_string_lossy())
            ));
            output.push_str("    <failure message=\"Pattern found\">\n");
            output.push_str(&format!(
                "Query: {}\nFile: {}\nLine {}: {}\nMatched: {}",
                escape_xml(query),
                escape_xml(&m.path.to_string_lossy()),
                m.line_number,
                escape_xml(&m.line),
                escape_xml(&m.matched_text)
            ));
            output.push_str("    </failure>\n");
            output.push_str("  </testcase>\n");
        }

        output.push_str("</testsuite>\n");
        output
    }

    fn name(&self) -> &str { "junit" }
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
