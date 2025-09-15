# rfgrep Man Pages

This directory contains comprehensive man pages for the `rfgrep` utility and its subcommands.

## Man Pages Included

- **rfgrep.1** - Main man page for the rfgrep utility
- **rfgrep-search.1** - Detailed documentation for the search command
- **rfgrep-interactive.1** - Documentation for interactive search mode
- **rfgrep-list.1** - Documentation for the list command
- **rfgrep-completions.1** - Documentation for shell completion generation

## Installation

### System-wide Installation (requires sudo)

```bash
cd man
sudo make install
```

### User Installation (no sudo required)

```bash
cd man
make install-user
```

Then add to your shell profile (`.bashrc`, `.zshrc`, etc.):

```bash
export MANPATH=$MANPATH:$HOME/.local/share/man
```

## Usage

After installation, you can view the man pages:

```bash
# Main man page
man rfgrep

# Command-specific man pages
man rfgrep-search
man rfgrep-interactive
man rfgrep-list
man rfgrep-completions
```

## Development

### Preview Man Pages

```bash
cd man
make preview
```

### Check Syntax

```bash
cd man
make check
```

### Validate Format

```bash
cd man
make validate
```

### Clean Generated Files

```bash
cd man
make clean
```

## Man Page Features

### Comprehensive Documentation
- Complete command reference
- Detailed option descriptions
- Practical examples
- Performance tips
- Troubleshooting guides

### Cross-references
- Links between related man pages
- References to similar tools (grep, ripgrep, find)
- See also sections for additional resources

### Examples
- Real-world usage examples
- Output format demonstrations
- Performance optimization tips
- Common use cases

### Troubleshooting
- Common issues and solutions
- Performance optimization
- Error handling
- Debugging tips

## Man Page Structure

Each man page follows the standard Unix man page format:

1. **NAME** - Command name and brief description
2. **SYNOPSIS** - Command syntax
3. **DESCRIPTION** - Detailed explanation
4. **OPTIONS** - Command-line options
5. **EXAMPLES** - Usage examples
6. **EXIT STATUS** - Return codes
7. **NOTES** - Additional information
8. **SEE ALSO** - Related commands

## Contributing

When updating man pages:

1. Follow the standard man page format
2. Include comprehensive examples
3. Add cross-references to related pages
4. Test with `make check` and `make validate`
5. Update the version number in the header

## Formatting

The man pages use standard troff formatting:

- `.TH` - Title header
- `.SH` - Section headers
- `.TP` - Tagged paragraphs
- `.B` - Bold text
- `.I` - Italic text
- `.RS/.RE` - Relative start/end
- `.PP` - Paragraph break

## Distribution

These man pages are included in the rfgrep distribution and can be installed alongside the binary. They provide comprehensive documentation for all rfgrep features and commands. 