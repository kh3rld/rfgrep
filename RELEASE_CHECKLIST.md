# Release Checklist

This checklist ensures a smooth and professional release process for rfgrep.

## Pre-Release Preparation

### 1. Code Quality
- [ ] Run `cargo clippy --all-targets --all-features`
- [ ] Run `cargo fmt --all -- --check`
- [ ] Run `cargo test --all-features`
- [ ] Run `cargo bench` to ensure performance is maintained
- [ ] Check for any TODO/FIXME comments that should be addressed

### 2. Documentation Updates
- [ ] Update `CHANGELOG.md` with new features and fixes
- [ ] Update version numbers in all man pages
- [ ] Update `README.md` if needed
- [ ] Update `INSTALLATION_GUIDE.md` with new version
- [ ] Verify all man pages are properly formatted
- [ ] Test man page installation: `cd man && make install-user`

### 3. Testing
- [ ] Run `./test_completions.sh` to verify shell completions
- [ ] Run `./test_man_pages.sh` to verify man pages
- [ ] Test installation on different platforms
- [ ] Test all major shell completions (bash, zsh, fish, powershell)
- [ ] Verify interactive mode functionality
- [ ] Test all output formats (text, json, xml, html, markdown)

### 4. Version Management
- [ ] Update version in `Cargo.toml`
- [ ] Update version in all man pages
- [ ] Update version in `INSTALLATION_GUIDE.md`
- [ ] Update changelog links
- [ ] Ensure semantic versioning is followed

## Release Process

### 1. Create Release Branch
```bash
git checkout -b release/v0.2.0
git add .
git commit -m "Prepare release v0.2.0"
```

### 2. Final Testing
- [ ] Build release version: `cargo build --release`
- [ ] Test all commands work correctly
- [ ] Verify man pages are accessible
- [ ] Test shell completions in different shells
- [ ] Run performance benchmarks

### 3. Create Release
- [ ] Tag the release: `git tag -a v0.2.0 -m "Release v0.2.0"`
- [ ] Push tags: `git push origin v0.2.0`
- [ ] Create GitHub release with:
  - Release notes from changelog
  - Binary assets for major platforms
  - Source code archive

### 4. Publish to Crates.io
```bash
cargo publish
```

### 5. Update Documentation
- [ ] Update GitHub repository description if needed
- [ ] Update any external documentation
- [ ] Announce release on relevant channels

## Post-Release Tasks

### 1. Monitor
- [ ] Monitor for any issues reported
- [ ] Check crates.io for successful publication
- [ ] Verify GitHub release assets are correct

### 2. Prepare Next Development Cycle
- [ ] Create new unreleased section in changelog
- [ ] Update version to next development version
- [ ] Create new development branch

### 3. Communication
- [ ] Update project status badges
- [ ] Notify maintainers and contributors
- [ ] Update any external references

## Quality Assurance Checklist

### Code Quality
- [ ] No compiler warnings
- [ ] No clippy warnings
- [ ] Code is properly formatted
- [ ] All tests pass
- [ ] Benchmarks show acceptable performance

### Documentation Quality
- [ ] All man pages are accessible
- [ ] Shell completions work correctly
- [ ] Installation guide is up to date
- [ ] README reflects current features
- [ ] Changelog is comprehensive

### User Experience
- [ ] Help messages are clear and helpful
- [ ] Error messages are informative
- [ ] Progress indicators work correctly
- [ ] Interactive mode is responsive
- [ ] All output formats work as expected

### Security
- [ ] All dependencies are up to date
- [ ] No known security vulnerabilities
- [ ] Proper error handling for file operations
- [ ] Safe handling of user input

## Release Notes Template

```markdown
## rfgrep v0.2.0

### New Features
- Interactive search mode with real-time filtering
- Multiple search algorithms (Boyer-Moore, Regex, Simple)
- Multiple output formats (JSON, XML, HTML, Markdown)
- Comprehensive man pages for all commands
- Shell completion support for all major shells

### 🚀 Performance Improvements
- Adaptive memory management
- Enhanced parallel processing
- Optimized binary file detection
- Improved regex caching

### 🛠️ Developer Experience
- Professional installation system
- Automated testing scripts
- Comprehensive documentation
- CI/CD workflow integration

### Bug Fixes
- Fixed compilation issues with dependencies
- Resolved runtime errors in search algorithms
- Corrected man page formatting
- Fixed shell completion generation

### Documentation
- Complete man page system
- Installation guides for all platforms
- Troubleshooting guides
- Performance optimization tips

## Installation

```bash
cargo install rfgrep
```

## Quick Start

```bash
# Search for patterns
rfgrep search "pattern" --extensions rs

# Interactive search
rfgrep interactive "pattern"

# List files with details
rfgrep list --extensions rs --detailed
```

## Breaking Changes

None in this release.

## Migration Guide

No migration required - this is a feature release with full backward compatibility.
```

## Emergency Procedures

### If Release Fails
1. Immediately revert the version bump
2. Investigate and fix the issue
3. Re-run all tests
4. Create a new release with proper fixes

### If Critical Bug is Found
1. Create hotfix branch
2. Fix the issue
3. Create patch release
4. Notify users of the issue and fix

### If Security Issue is Found
1. Immediately create security advisory
2. Fix the issue in private
3. Create security release
4. Notify all users immediately 