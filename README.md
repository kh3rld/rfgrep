# rfgrep  

A command-line utility for recursively searching and listing files with advanced filtering capabilities. Built in Rust.

[<img alt="crates.io" src="https://img.shields.io/crates/v/rfgrep.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/rfgrep)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-rfgrep-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/rfgrep)
[![CI](https://github.com/kh3rld/rfgrep/actions/workflows/ci.yml/badge.svg)](https://github.com/kh3rld/rfgrep/actions/workflows/ci.yml)
[![License](https://img.shields.io/github/license/kh3rld/rfgrep)](https://github.com/kh3rld/rfgrep/blob/main/LICENSE)

## Features

- **Advanced Search**
  - Regex, plain text, and whole-word matching
  - Recursive directory traversal
  - Binary file detection
  - Extension filtering
  - Size limits

- **File Listing**
  - Detailed/simple output formats
  - Extension statistics
  - Size filtering
  - Hidden file handling

- **Utilities**
  - Clipboard copy support
  - Dry-run mode
  - Logging to file
  - Progress indicators

<!-- ## Performance

![Benchmark Results](https://github.com/kh3rld/rfgrep/raw/main/benches/comparison.png)

Latest benchmarks (Linux x86_64):
```bash
# Updated automatically by CI
cat benches/latest.txt -->

## Installation

Assuming you have [Rust installed][Rust], run:

[Rust]: https://www.rust-lang.org/

### Via Cargo

```bash
cargo install rfgrep
```

### From GitHub
```bash
cargo install --git https://github.com/kh3rld/rfgrep.git
```

### From Source

```bash
git clone https://github.com/kh3rld/rfgrep.git
cargo build --release
```

### Installing Man Pages

After installing rfgrep, you can install the comprehensive man pages:

#### System-wide Installation (requires sudo)
```bash
cd man
sudo make install
```

#### User Installation (no sudo required)
```bash
cd man
make install-user
```

Then add to your shell profile (`.bashrc`, `.zshrc`, etc.):
```bash
export MANPATH=$MANPATH:$HOME/.local/share/man
```

### Installing Shell Completions

rfgrep supports tab completion for all major shells:

#### Bash
```bash
# Generate and source completion
rfgrep completions bash >> ~/.bashrc
source ~/.bashrc
```

#### Zsh
```bash
# Generate completion file
rfgrep completions zsh > ~/.zsh/completions/_rfgrep
# Add to .zshrc
echo 'fpath=(~/.zsh/completions $fpath)' >> ~/.zshrc
autoload -U compinit && compinit
```

#### Fish
```bash
# Generate and install
rfgrep completions fish --install --user
```

#### PowerShell
```bash
# Generate and import
rfgrep completions powershell > rfgrep-completion.ps1
. rfgrep-completion.ps1
```

## Usage

### Basic Search

```bash
rfgrep search "pattern"
```

### Search with Options

```bash
rfgrep search "pattern" \
    --mode regex \
    --extensions rs,toml \
    --max-size 5 \
    --skip-binary \
    --copy
```

### File Listing

```bash
# Simple list
rfgrep list

# Detailed view
rfgrep list --long --recursive

# With filters
rfgrep list --extensions rs,toml --max-size 10 --show-hidden
```

## Documentation

### Man Pages

After installation, comprehensive man pages are available:

```bash
# Main man page
man rfgrep

# Command-specific man pages
man rfgrep-search
man rfgrep-interactive
man rfgrep-list
man rfgrep-completions
```

The man pages include:
- Complete command reference
- Detailed option descriptions
- Practical examples
- Performance tips
- Troubleshooting guides

### Shell Completions

Once installed, tab completion provides:
- Command completion (`rfgrep <TAB>`)
- Option completion (`rfgrep search --<TAB>`)
- Extension completion (`--extensions <TAB>`)
- File path completion (`src/<TAB>`)

### Troubleshooting

#### Man Pages Not Found
```bash
# Check if man pages are installed
ls ~/.local/share/man/man1/rfgrep*

# Add to shell profile if needed
echo 'export MANPATH=$MANPATH:$HOME/.local/share/man' >> ~/.bashrc
```

#### Completions Not Working
```bash
# Regenerate completions
rfgrep completions bash > ~/.bash_completion.d/rfgrep

# Reload shell configuration
source ~/.bashrc

# For zsh, ensure completion directory exists
mkdir -p ~/.zsh/completions
rfgrep completions zsh > ~/.zsh/completions/_rfgrep

# For fish, install to user directory
rfgrep completions fish --install --user
```

#### Performance Issues
```bash
# Use dry-run to preview
rfgrep search "pattern" --dry-run

# Skip binary files
rfgrep search "pattern" --skip-binary

# Limit file size
rfgrep search "pattern" --max-size 10

# Use specific extensions
rfgrep search "pattern" --extensions rs,py,js
```

#### Shell-Specific Troubleshooting

**Bash:**
```bash
# Check if completion is loaded
complete -p | grep rfgrep

# Manual installation
rfgrep completions bash >> ~/.bashrc
source ~/.bashrc
```

**Zsh:**
```bash
# Check completion directory
ls ~/.zsh/completions/_rfgrep

# Reload completions
autoload -U compinit && compinit
```

**Fish:**
```bash
# Check if completion is installed
ls ~/.config/fish/completions/rfgrep.fish

# Manual installation
rfgrep completions fish > ~/.config/fish/completions/rfgrep.fish
```

## Command Reference

### Global Options

| Option       | Description                     |
|--------------|---------------------------------|
| `--log PATH` | Write logs to specified file    |
| `--path DIR` | Base directory (default: `.`)   |

### Search Command

| Option             | Description                         |
|--------------------|-------------------------------------|
| `--mode MODE`      | Search mode: regex/text/word        |
| `--extensions EXT` | Comma-separated file extensions     |
| `--max-size MB`    | Skip files larger than specified MB |
| `--skip-binary`    | Skip binary files                   |
| `--dry-run`        | Preview files without processing    |
| `--copy`           | Copy results to clipboard           |

### List Command

| Option             | Description                         |
|--------------------|-------------------------------------|
| `--extensions EXT` | Comma-separated file extensions     |
| `--long`           | Detailed output format              |
| `--recursive`      | Recursive directory traversal       |
| `--show-hidden`    | Include hidden files/directories    |
| `--max-size MB`    | Skip files larger than specified MB |
| `--skip-binary`    | Skip binary files                   |

## Examples

1. Find all Rust files containing "HashMap":

```bash
rfgrep search "HashMap" --extensions rs
```

1. List all Markdown files under 1MB:

```bash
rfgrep list --extensions md --max-size 1
```

1. Search with regex and copy to clipboard:

```bash
rfgrep search "fn\s+\w+\s*\(" --mode regex --copy
```

## Performance Tips

- Use `--skip-binary` to avoid unnecessary file checks
- Limit scope with `--extensions` and `--max-size`
- For large directories, `--dry-run` first to preview

## Advanced Usage

### Interactive Mode
```bash
# Start interactive search
rfgrep interactive "pattern"

# Interactive search with specific algorithm
rfgrep interactive "pattern" --algorithm boyer-moore

# Interactive search in specific file types
rfgrep interactive "pattern" --extensions rs,py
```

### Output Formats
```bash
# JSON output for programmatic processing
rfgrep search "pattern" --output-format json

# XML output for structured data
rfgrep search "pattern" --output-format xml

# HTML output for web display
rfgrep search "pattern" --output-format html

# Markdown output for documentation
rfgrep search "pattern" --output-format markdown
```

### Search Algorithms
```bash
# Boyer-Moore (fast for plain text)
rfgrep search "pattern" --algorithm boyer-moore

# Regular expression
rfgrep search "pattern" --algorithm regex

# Simple linear search
rfgrep search "pattern" --algorithm simple
```

## Verification

### Test Man Pages
```bash
# Verify man pages are accessible
man rfgrep
man rfgrep-search
man rfgrep-interactive
man rfgrep-list
man rfgrep-completions
```

### Test Shell Completions
```bash
# Bash: Type 'rfgrep ' and press TAB
rfgrep <TAB>

# Zsh: Type 'rfgrep ' and press TAB
rfgrep <TAB>

# Fish: Type 'rfgrep ' and press TAB
rfgrep <TAB>
```

### Test Basic Functionality
```bash
# Test search functionality
rfgrep search "test" --extensions rs

# Test list functionality
rfgrep list --extensions rs

# Test interactive mode
rfgrep interactive "test" --extensions rs
```

### Automated Testing
```bash
# Test shell completions
./test_completions.sh

# Test man pages
./test_man_pages.sh
```

## Contributing

Contributions are welcome! Please open an issue or PR for any:
- Bug reports
- Feature requests
- Performance improvements
