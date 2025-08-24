# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-08-05

### Added
- **Interactive Search Mode**: Real-time search with filtering and navigation
  - Interactive command-line interface with keyboard shortcuts
  - Real-time result filtering and refinement
  - Context viewing and result navigation
  - Search statistics and performance metrics
- **Advanced Search Algorithms**: Multiple search algorithm support
  - Boyer-Moore algorithm for fast plain text search
  - Regex algorithm for pattern matching
  - Simple linear search as fallback option
  - Unified search algorithm trait for extensibility
- **Multiple Output Formats**: Support for various output formats
  - Text format with colored highlighting (default)
  - JSON format for programmatic processing
  - XML format for structured data
  - HTML format for web display
  - Markdown format for documentation
- **Adaptive Memory Management**: Intelligent memory usage optimization
  - Dynamic memory mapping thresholds based on system resources
  # Changelog
  All notable changes to this project will be documented in this file.

  The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
  and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

  ## [0.2.1] - 2025-08-24

  ### Added
  - Structured search results: `search_file` now returns `SearchMatch` objects that include the file `path`, `line_number`, `matched_text`, and surrounding context (`context_before` / `context_after`).

  ### Changed
  - Removed `anyhow` across the crate and standardized on `crate::error::Result<T>` and `RfgrepError` as the canonical error type.
  - JSON output now serializes structured `SearchMatch` objects for machine-friendly results.
  - CI workflow (`.github/workflows/ci.yml`) rewritten to run a matrix of tests (Linux/macOS/Windows), enforce formatting and clippy, build platform artifacts, and provide an optional integration harness under Xvfb.

  ### Fixed
  - Clippy and style fixes across the codebase; ran `cargo fmt` and resolved warnings.
  - Updated scripts (e.g. `scripts/run_benchmarks.rs`) to avoid reintroducing `anyhow`.

  ### Docs
  - Tidied man pages (prefer `--path` in `man/rfgrep.1`) and updated release notes.

  ## [0.2.0] - 2025-08-05

  ### Added
  - **Interactive Search Mode**: Real-time search with filtering and navigation
    - Interactive command-line interface with keyboard shortcuts
    - Real-time result filtering and refinement
    - Context viewing and result navigation
    - Search statistics and performance metrics
  - **Advanced Search Algorithms**: Multiple search algorithm support
    - Boyer-Moore algorithm for fast plain text search
    - Regex algorithm for pattern matching
    - Simple linear search as fallback option
    - Unified search algorithm trait for extensibility
  - **Multiple Output Formats**: Support for various output formats
    - Text format with colored highlighting (default)
    - JSON format for programmatic processing
    - XML format for structured data
    - HTML format for web display
    - Markdown format for documentation
  - **Adaptive Memory Management**: Intelligent memory usage optimization
    - Dynamic memory mapping thresholds based on system resources
    - Adaptive chunk sizing for parallel processing
    - Memory usage monitoring and optimization
    - Configurable performance settings
  - **Comprehensive Man Pages**: Professional documentation system
    - Main man page (`rfgrep.1`) with complete overview
    - Command-specific man pages for all subcommands
    - Detailed examples and performance tips
    - Troubleshooting guides and best practices
  - **Shell Completion Support**: Tab completion for all major shells
    - Bash completion with command and option completion
    - Zsh completion with descriptions and fuzzy matching
    - Fish completion with built-in support
    - PowerShell completion for cross-platform support
    - Elvish completion for modern shell experience
  - **Enhanced CLI Interface**: Improved command-line experience
    - Detailed help messages with examples
    - Better error handling and user feedback
    - Progress indicators and status updates
    - Verbose logging and debugging options
  - **Installation and Testing Tools**: Professional deployment system
    - Makefile for easy man page installation
    - Automated testing scripts for completions and man pages
    - Comprehensive installation guide
    - Verification and troubleshooting tools

  ### Changed
  - **Performance Optimizations**: Improved search and processing speed
    - Enhanced memory mapping for large files
    - Optimized parallel processing with adaptive chunking
    - Better binary file detection and skipping
    - Improved regex caching and compilation
    - **Error Handling**: More robust error management
    - Better error messages and user feedback
    - Graceful handling of file system errors
    - Improved logging and debugging capabilities
    - **Documentation**: Enhanced user experience
    - Updated README with comprehensive installation instructions
    - Added troubleshooting guides and performance tips
    - Improved help messages and examples
    - Better cross-references between man pages

  ### Fixed
  - **Compilation Issues**: Resolved dependency and build problems
    - Fixed indicatif dependency version conflicts
    - Resolved serde_json import issues
    - Fixed man page formatting and syntax errors
    - Corrected regex pattern escaping in examples
  - **Runtime Errors**: Improved stability and reliability
    - Fixed index out of bounds in Boyer-Moore algorithm
    - Resolved interactive mode display issues
    - Fixed memory management edge cases
    - Corrected completion script generation

  ## [0.1.0] - 2025-06-23

  ### Added
  - Initial implementation of recursive file search functionality
  - Core features:
    - Recursive directory traversal
    - Regex/text/whole-word search modes
    - File extension filtering
    - Binary file detection
    - Size-based filtering
  - Cross-platform support (Windows, macOS, Linux)
  - GitHub Actions CI/CD pipeline
  - Comprehensive documentation
  - Man pages and shell completions

  [Unreleased]: https://github.com/kh3rld/rfgrep/compare/v0.2.1...HEAD
  [0.2.1]: https://github.com/kh3rld/rfgrep/compare/v0.2.0...v0.2.1
  [0.2.0]: https://github.com/kh3rld/rfgrep/compare/v0.1.0...v0.2.0
  [0.1.0]: https://github.com/kh3rld/rfgrep/releases/tag/v0.1.0
