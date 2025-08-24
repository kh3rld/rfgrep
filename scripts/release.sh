#!/bin/bash

# rfgrep Release Script
# This script automates the release process for rfgrep

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to validate version format
validate_version() {
    local version=$1
    if [[ ! $version =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    print_error "Invalid version format. Expected format: vX.Y.Z (e.g., v0.2.1)"
        exit 1
    fi
}

# Function to check if git is clean
check_git_status() {
    if [[ -n $(git status --porcelain) ]]; then
        print_error "Git working directory is not clean. Please commit or stash changes."
        git status --porcelain
        exit 1
    fi
}

# Function to check if tag already exists
check_tag_exists() {
    local version=$1
    if git tag -l | grep -q "^$version$"; then
        print_error "Tag $version already exists."
        exit 1
    fi
}

# Function to run tests
run_tests() {
    print_status "Running tests..."
    cargo test --all-features --workspace --verbose
    print_success "Tests passed"
}

# Function to run clippy
run_clippy() {
    print_status "Running clippy..."
    cargo clippy --all-targets --all-features -- -D warnings
    print_success "Clippy passed"
}

# Function to check formatting
check_formatting() {
    print_status "Checking code formatting..."
    cargo fmt --all -- --check
    print_success "Code formatting is correct"
}

# Function to build release
build_release() {
    print_status "Building release binary..."
    cargo build --release
    print_success "Release binary built successfully"
}

# Function to test man pages
test_man_pages() {
    print_status "Testing man pages..."
    if [[ -f "test_man_pages.sh" ]]; then
        chmod +x test_man_pages.sh
        ./test_man_pages.sh
        print_success "Man pages test passed"
    else
        print_warning "test_man_pages.sh not found, skipping man pages test"
    fi
}

# Function to test completions
test_completions() {
    print_status "Testing shell completions..."
    if [[ -f "test_completions.sh" ]]; then
        chmod +x test_completions.sh
        ./test_completions.sh
        print_success "Completions test passed"
    else
        print_warning "test_completions.sh not found, skipping completions test"
    fi
}

# Function to update version in files
update_version() {
    local version=$1
    local version_number=${version#v}
    
    print_status "Updating version to $version_number in Cargo.toml..."
    sed -i "s/^version = \".*\"/version = \"$version_number\"/" Cargo.toml
    
    print_status "Updating version in man pages..."
    find man -name "*.1" -exec sed -i "s/rfgrep [0-9]\+\.[0-9]\+\.[0-9]\+/rfgrep $version_number/g" {} \;
    
    print_success "Version updated in all files"
}

# Function to create git tag
create_tag() {
    local version=$1
    
    print_status "Creating git tag $version..."
    git add .
    git commit -m "chore: bump version to $version"
    git tag -a "$version" -m "Release $version"
    print_success "Git tag $version created"
}

# Function to push to remote
push_to_remote() {
    local version=$1
    
    print_status "Pushing to remote..."
    git push origin main
    git push origin "$version"
    print_success "Pushed to remote successfully"
}

# Function to create release package
create_release_package() {
    local version=$1
    local version_number=${version#v}
    
    print_status "Creating release package..."
    
    # Create release directory
    mkdir -p release
    cp target/release/rfgrep release/
    cp -r man release/ 2>/dev/null || true
    cp *.md release/ 2>/dev/null || true
    cp *.sh release/ 2>/dev/null || true
    
    # Create archive
    local archive_name="rfgrep-${version_number}-linux-x86_64.tar.gz"
    tar -czf "$archive_name" -C release .
    
    # Generate checksum
    sha256sum "$archive_name" > "${archive_name}.sha256"
    
    print_success "Release package created: $archive_name"
    print_success "Checksum file created: ${archive_name}.sha256"
}

# Function to display help
show_help() {
    cat << EOF
rfgrep Release Script

Usage: $0 [OPTIONS] VERSION

Options:
    -h, --help      Show this help message
    -d, --dry-run   Run without making changes
    -t, --test      Run tests only
    -p, --package   Create release package only

Examples:
    $0 v0.2.1              # Full release process
    $0 -d v0.2.1          # Dry run
    $0 -t                  # Run tests only
    $0 -p v0.2.1          # Create package only

The script will:
1. Validate the version format
2. Check git status
3. Run tests and clippy
4. Update version in files
5. Create git tag
6. Push to remote
7. Create release package

Requirements:
- Rust toolchain
- Git
- tar, sha256sum
EOF
}

# Main function
main() {
    local dry_run=false
    local test_only=false
    local package_only=false
    local version=""
    
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_help
                exit 0
                ;;
            -d|--dry-run)
                dry_run=true
                shift
                ;;
            -t|--test)
                test_only=true
                shift
                ;;
            -p|--package)
                package_only=true
                shift
                ;;
            -*)
                print_error "Unknown option: $1"
                show_help
                exit 1
                ;;
            *)
                if [[ -z "$version" ]]; then
                    version=$1
                else
                    print_error "Multiple versions specified"
                    exit 1
                fi
                shift
                ;;
        esac
    done
    
    # Check if we're in a git repository
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        print_error "Not in a git repository"
        exit 1
    fi
    
    # Test only mode
    if [[ "$test_only" == true ]]; then
        print_status "Running tests only..."
        run_tests
        run_clippy
        check_formatting
        test_man_pages
        test_completions
        print_success "All tests passed!"
        exit 0
    fi
    
    # Package only mode
    if [[ "$package_only" == true ]]; then
        if [[ -z "$version" ]]; then
            print_error "Version required for package mode"
            exit 1
        fi
        validate_version "$version"
        build_release
        create_release_package "$version"
        exit 0
    fi
    
    # Full release mode
    if [[ -z "$version" ]]; then
        print_error "Version is required"
        show_help
        exit 1
    fi
    
    validate_version "$version"
    
    print_status "Starting release process for $version"
    
    # Pre-release checks
    check_git_status
    check_tag_exists "$version"
    
    # Run tests
    run_tests
    run_clippy
    check_formatting
    test_man_pages
    test_completions
    
    if [[ "$dry_run" == true ]]; then
        print_warning "Dry run mode - no changes will be made"
        print_status "Would update version to $version"
        print_status "Would create git tag $version"
        print_status "Would push to remote"
        print_status "Would create release package"
        exit 0
    fi
    
    # Update version
    update_version "$version"
    
    # Build release
    build_release
    
    # Create git tag
    create_tag "$version"
    
    # Push to remote
    push_to_remote "$version"
    
    # Create release package
    create_release_package "$version"
    
    print_success "Release $version completed successfully!"
    print_status "Next steps:"
    print_status "1. Check GitHub Actions for automated release"
    print_status "2. Review the release on GitHub"
    print_status "3. Update any external documentation"
}

# Run main function with all arguments
main "$@" 