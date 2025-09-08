//! Advanced output formatting and management system
pub mod formatter;
pub mod formats;
pub mod streaming;

use crate::cli::{OutputFormat as CliOutputFormat, ColorChoice};
use crate::processor::SearchMatch;
use std::path::Path;

pub use formatter::OutputFormatter;
pub use formats::*;

/// Output manager that coordinates different output formats
pub struct OutputManager {
    formatters: std::collections::HashMap<String, Box<dyn OutputFormatterTrait>>,
}

/// Trait for output formatters
pub trait OutputFormatterTrait: Send + Sync {
    fn format(&self, matches: &[SearchMatch], query: &str, path: &Path) -> String;
    fn name(&self) -> &str;
    fn supports_streaming(&self) -> bool { false }
}

impl OutputManager {
    /// Create a new output manager
    pub fn new() -> crate::error::Result<Self> {
        let mut manager = Self {
            formatters: std::collections::HashMap::new(),
        };

        // Register built-in formatters
        manager.register_formatter("text", Box::new(TextFormatter::new()));
        manager.register_formatter("json", Box::new(JsonFormatter::new()));
        manager.register_formatter("xml", Box::new(XmlFormatter::new()));
        manager.register_formatter("html", Box::new(HtmlFormatter::new()));
        manager.register_formatter("markdown", Box::new(MarkdownFormatter::new()));
        manager.register_formatter("csv", Box::new(CsvFormatter::new()));
        manager.register_formatter("yaml", Box::new(YamlFormatter::new()));
        manager.register_formatter("junit", Box::new(JunitFormatter::new()));

        Ok(manager)
    }

    /// Register a custom formatter
    pub fn register_formatter(&mut self, name: &str, formatter: Box<dyn OutputFormatterTrait>) {
        self.formatters.insert(name.to_string(), formatter);
    }

    /// Format search results
    pub fn format_results(
        &self,
        matches: &[SearchMatch],
        query: &str,
        path: &Path,
        format: CliOutputFormat,
        ndjson: bool,
        color: ColorChoice,
    ) -> crate::error::Result<String> {
        let formatter_name = match format {
            CliOutputFormat::Text => "text",
            CliOutputFormat::Json => "json",
            CliOutputFormat::Xml => "xml",
            CliOutputFormat::Html => "html",
            CliOutputFormat::Markdown => "markdown",
        };

        let formatter = self.formatters.get(formatter_name)
            .ok_or_else(|| crate::error::RfgrepError::Other(format!("Unknown format: {formatter_name}")))?;

        let mut output = formatter.format(matches, query, path);

        // Apply color if needed
        if matches!(color, ColorChoice::Always) || 
           (matches!(color, ColorChoice::Auto) && is_terminal::is_terminal(&std::io::stdout())) {
            output = self.apply_color(&output, query);
        }

        Ok(output)
    }

    /// Create a formatter instance
    pub fn create_formatter(&self, format: OutputFormat, ndjson: bool) -> OutputFormatter {
        OutputFormatter::new(format, ndjson)
    }

    /// Copy results to clipboard
    pub fn copy_to_clipboard(&self, content: &str) -> crate::error::Result<()> {
        let can_use_clipboard = std::env::var("DISPLAY").is_ok() || 
                               std::env::var("WAYLAND_DISPLAY").is_ok();

        if can_use_clipboard {
            match arboard::Clipboard::new() {
                Ok(mut clipboard) => {
                    clipboard.set_text(content.to_string())
                        .map_err(|e| crate::error::RfgrepError::Other(format!("Clipboard error: {e}")))?;
                    println!("\n{}", "Results copied to clipboard!".green());
                }
                Err(e) => {
                    self.fallback_to_file(content)?;
                    eprintln!("Clipboard init failed: {e}");
                }
            }
        } else {
            self.fallback_to_file(content)?;
        }

        Ok(())
    }

    /// Fallback to writing to a temporary file
    fn fallback_to_file(&self, content: &str) -> crate::error::Result<()> {
        let tmp = std::env::temp_dir().join("rfgrep_results.txt");
        std::fs::write(&tmp, content)
            .map_err(|e| crate::error::RfgrepError::Io(e))?;
        println!("\n{} {}", "Results written to".green(), tmp.display());
        Ok(())
    }

    /// Apply color highlighting to output
    fn apply_color(&self, output: &str, query: &str) -> String {
        use colored::*;
        
        output
            .lines()
            .map(|line| {
                if line.contains(query) {
                    line.replace(query, &query.yellow().bold().to_string())
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get available formatters
    pub fn get_available_formatters(&self) -> Vec<&str> {
        self.formatters.keys().map(|k| k.as_str()).collect()
    }
}

/// Output format enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
    Xml,
    Html,
    Markdown,
    Csv,
    Yaml,
    Junit,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "text" | "txt" => Some(Self::Text),
            "json" => Some(Self::Json),
            "xml" => Some(Self::Xml),
            "html" => Some(Self::Html),
            "markdown" | "md" => Some(Self::Markdown),
            "csv" => Some(Self::Csv),
            "yaml" | "yml" => Some(Self::Yaml),
            "junit" | "junit-xml" => Some(Self::Junit),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
            Self::Xml => "xml",
            Self::Html => "html",
            Self::Markdown => "markdown",
            Self::Csv => "csv",
            Self::Yaml => "yaml",
            Self::Junit => "junit",
        }
    }
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Text
    }
}
