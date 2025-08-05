use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

#[derive(Parser)]
#[clap(
    name = "rfgrep",
    author = "Khalid Hussein <kh3rld.hussein@gmail.com>",
    version,
    about = "Recursive file grep utility with advanced filtering - search, list, and analyze text files with regex support",
    long_about = r#"
rfgrep - A powerful command-line utility for recursively searching and listing files with advanced filtering capabilities.

FEATURES:
  • Advanced Search: Regex, plain text, and whole-word matching
  • File Listing: Detailed/simple output formats with extension statistics
  • Performance: Parallel processing with memory mapping for large files
  • Filtering: Extension, size, and binary file filtering
  • Utilities: Clipboard copy, dry-run mode, and progress indicators

EXAMPLES:
  # Search for "HashMap" in Rust files
  rfgrep search "HashMap" --extensions rs

  # List all Markdown files under 1MB
  rfgrep list --extensions md --max-size 1

  # Search with regex and copy to clipboard
  rfgrep search "fn\s+\w+\s*\(" regex --copy

  # Recursive search with word boundaries
  rfgrep search "test" word --recursive --extensions rs

PERFORMANCE TIPS:
  • Use --skip-binary to avoid unnecessary file checks
  • Limit scope with --extensions and --max-size
  • Use --dry-run first to preview files
  • Enable --recursive for deep directory traversal
"#,
    after_help = r#"
For more information, visit: https://github.com/kh3rld/rfgrep
"#
)]
pub struct Cli {
    /// Base directory to search/list (default: current directory)
    #[clap(default_value = ".")]
    pub path: PathBuf,

    /// Enable verbose logging output
    #[clap(long, value_parser, default_value_t = false)]
    pub verbose: bool,

    /// Write logs to specified file
    #[clap(long, value_parser)]
    pub log: Option<PathBuf>,

    /// Preview files without processing (useful for testing)
    #[clap(long, value_parser, default_value_t = false)]
    pub dry_run: bool,

    /// Skip files larger than specified MB
    #[clap(long, value_parser)]
    pub max_size: Option<usize>,

    /// Skip binary files (improves performance)
    #[clap(long, value_parser, default_value_t = false)]
    pub skip_binary: bool,

    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Search for patterns in files with advanced filtering
    #[clap(after_help = r#"
SEARCH MODES:
  text    - Plain text search (default)
  word    - Whole word matching with boundaries
  regex   - Regular expression search

EXAMPLES:
  # Basic text search
  rfgrep search "error" --extensions rs

  # Word boundary search
  rfgrep search "test" word --recursive

  # Regex search for function definitions
  rfgrep search "fn\s+\w+\s*\(" regex --extensions rs

  # Search and copy results to clipboard
  rfgrep search "TODO" --copy --extensions rs,md

PERFORMANCE TIPS:
  • Use --skip-binary for faster processing
  • Limit file size with --max-size
  • Use --dry-run to preview files first
  • Combine --extensions with --recursive for targeted search
"#)]
    Search {
        /// Pattern to search for in files
        pattern: String,

        /// Search mode to use
        #[clap(value_parser, default_value_t = SearchMode::Text)]
        mode: SearchMode,

        /// Copy search results to clipboard
        #[clap(long, value_parser, default_value_t = false)]
        copy: bool,

        /// Output format for results
        #[clap(long, value_enum, default_value_t = OutputFormat::Text)]
        output_format: OutputFormat,

        /// Comma-separated list of file extensions to include
        #[clap(long, value_parser, use_value_delimiter = true)]
        extensions: Option<Vec<String>>,

        /// Recursively search subdirectories
        #[clap(short, long, value_parser, default_value_t = false)]
        recursive: bool,
    },

    /// Interactive search mode with real-time filtering
    #[clap(after_help = r#"
INTERACTIVE FEATURES:
  • Real-time search with live filtering
  • Keyboard navigation and commands
  • Result highlighting and selection
  • Save results to file
  • Memory-optimized processing

COMMANDS:
  n/new   - Start a new search
  f/filter - Filter current results
  c/clear - Clear all filters
  s/save  - Save results to file
  q/quit  - Exit interactive mode

EXAMPLES:
  # Start interactive search
  rfgrep interactive "error" --extensions rs

  # Interactive search with specific algorithm
  rfgrep interactive "test" --algorithm boyer-moore --recursive
"#)]
    Interactive {
        /// Pattern to search for in files
        pattern: String,

        /// Search algorithm to use
        #[clap(long, value_enum, default_value_t = InteractiveAlgorithm::BoyerMoore)]
        algorithm: InteractiveAlgorithm,

        /// Comma-separated list of file extensions to include
        #[clap(long, value_parser, use_value_delimiter = true)]
        extensions: Option<Vec<String>>,

        /// Recursively search subdirectories
        #[clap(short, long, value_parser, default_value_t = false)]
        recursive: bool,
    },
    /// List files with detailed information and statistics
    #[clap(after_help = r#"
OUTPUT FORMATS:
  Simple  - Just file paths (default)
  Long    - Detailed table with size, type, and binary info

EXAMPLES:
  # Simple file listing
  rfgrep list --extensions rs

  # Detailed listing with size info
  rfgrep list --long --extensions rs,md

  # Recursive listing with hidden files
  rfgrep list --recursive --show-hidden --extensions rs

  # List files under 1MB
  rfgrep list --max-size 1 --extensions rs

FEATURES:
  • Extension statistics and file counts
  • Binary file detection
  • Size filtering and formatting
  • Hidden file handling
  • Recursive directory traversal
"#)]
    List {
        /// Comma-separated list of file extensions to include
        #[clap(long, value_parser, use_value_delimiter = true)]
        extensions: Option<Vec<String>>,

        /// Show detailed output with size and type information
        #[clap(short, long, value_parser, default_value_t = false)]
        long: bool,

        /// Recursively list files in subdirectories
        #[clap(short, long, value_parser, default_value_t = false)]
        recursive: bool,

        /// Include hidden files and directories
        #[clap(long, value_parser, default_value_t = false)]
        show_hidden: bool,
    },
    /// Generate shell completion scripts for better CLI experience
    #[clap(after_help = r#"
SUPPORTED SHELLS:
  bash     - Bash shell completions
  zsh      - Zsh shell completions
  fish     - Fish shell completions
  powershell - PowerShell completions
  elvish   - Elvish shell completions

EXAMPLES:
  # Generate bash completions
  rfgrep completions bash > ~/.local/share/bash-completion/completions/rfgrep

  # Generate zsh completions
  rfgrep completions zsh > ~/.zsh/completions/_rfgrep

  # Generate fish completions
  rfgrep completions fish > ~/.config/fish/completions/rfgrep.fish

SETUP:
  Add the generated completion script to your shell's completion directory
  and restart your shell or source the completion file.
"#)]
    Completions {
        /// The shell to generate completions for
        #[clap(value_enum)]
        shell: Shell,
    },
}

#[derive(ValueEnum, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchMode {
    /// Plain text search (case-sensitive)
    #[default]
    Text,
    /// Whole word matching with word boundaries
    Word,
    /// Regular expression search
    Regex,
}

#[derive(ValueEnum, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InteractiveAlgorithm {
    /// Boyer-Moore algorithm for fast text search
    #[default]
    BoyerMoore,
    /// Regular expression search
    Regex,
    /// Simple text search
    Simple,
}

#[derive(ValueEnum, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    /// Plain text output (default)
    #[default]
    Text,
    /// JSON format for programmatic processing
    Json,
    /// XML format for structured data
    Xml,
    /// HTML format for web display
    Html,
    /// Markdown format for documentation
    Markdown,
}

impl fmt::Display for SearchMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SearchMode::Text => write!(f, "text"),
            SearchMode::Word => write!(f, "word"),
            SearchMode::Regex => write!(f, "regex"),
        }
    }
}
