# rfgrep  

A command-line utility for recursively searching and listing files with advanced filtering capabilities. Built in Rust.

[![CI](https://github.com/kh3rld/rfgrep/actions/workflows/ci.yml/badge.svg)](https://github.com/kh3rld/rfgrep/actions/workflows/ci.yml)

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

## Installation

### From Source

```bash
git clone https://github.com/kh3rld/rfgrep.git
cd rfgrep
cargo install --path .
```

### Via Cargo

```bash
cargo install rfgrep
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

## Command Reference

### Global Options

| Option       | Description                     |
|--------------|---------------------------------|
| `--log PATH` | Write logs to specified file    |
| `--path DIR` | Base directory (default: `.`)   |

### Search Command

| Option            | Description                          |
|-------------------|--------------------------------------|
| `--mode MODE`     | Search mode: regex/text/word         |
| `--extensions EXT`| Comma-separated file extensions      |
| `--max-size MB`   | Skip files larger than specified MB  |
| `--skip-binary`   | Skip binary files                   |
| `--dry-run`       | Preview files without processing     |
| `--copy`          | Copy results to clipboard           |

### List Command

| Option            | Description                          |
|-------------------|--------------------------------------|
| `--extensions EXT`| Comma-separated file extensions      |
| `--long`          | Detailed output format              |
| `--recursive`     | Recursive directory traversal       |
| `--show-hidden`   | Include hidden files/directories    |
| `--max-size MB`   | Skip files larger than specified MB  |
| `--skip-binary`   | Skip binary files                   |

## Examples

1. Find all Rust files containing "HashMap":

```bash
rfgrep search "HashMap" --extensions rs
```

2. List all Markdown files under 1MB:

```bash
rfgrep list --extensions md --max-size 1
```

3. Search with regex and copy to clipboard:

```bash
rfgrep search "fn\s+\w+\s*\(" --mode regex --copy
```

## Performance Tips

- Use `--skip-binary` to avoid unnecessary file checks
- Limit scope with `--extensions` and `--max-size`
- For large directories, `--dry-run` first to preview

## Contributing

Contributions are welcome! Please open an issue or PR for any:
- Bug reports
- Feature requests
- Performance improvements
