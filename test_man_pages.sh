#!/bin/bash

# Test script for rfgrep man pages
# This script verifies that man pages are properly installed and accessible

echo "Testing rfgrep man pages..."

# Set MANPATH to include user directory
export MANPATH=$MANPATH:/home/eternity/.local/share/man

# Test main man page
echo "=== Testing Main Man Page ==="
if man rfgrep > /dev/null 2>&1; then
    echo "✅ Main man page (rfgrep) is accessible"
    man rfgrep | head -5
else
    echo "❌ Main man page (rfgrep) is not accessible"
fi

# Test search man page
echo "=== Testing Search Man Page ==="
if man rfgrep-search > /dev/null 2>&1; then
    echo "✅ Search man page (rfgrep-search) is accessible"
    man rfgrep-search | head -5
else
    echo "❌ Search man page (rfgrep-search) is not accessible"
fi

# Test interactive man page
echo "=== Testing Interactive Man Page ==="
if man rfgrep-interactive > /dev/null 2>&1; then
    echo "✅ Interactive man page (rfgrep-interactive) is accessible"
    man rfgrep-interactive | head -5
else
    echo "❌ Interactive man page (rfgrep-interactive) is not accessible"
fi

# Test list man page
echo "=== Testing List Man Page ==="
if man rfgrep-list > /dev/null 2>&1; then
    echo "✅ List man page (rfgrep-list) is accessible"
    man rfgrep-list | head -5
else
    echo "❌ List man page (rfgrep-list) is not accessible"
fi

# Test completions man page
echo "=== Testing Completions Man Page ==="
if man rfgrep-completions > /dev/null 2>&1; then
    echo "✅ Completions man page (rfgrep-completions) is accessible"
    man rfgrep-completions | head -5
else
    echo "❌ Completions man page (rfgrep-completions) is not accessible"
fi

# Check if man page files exist
echo "=== Checking Man Page Files ==="
MAN_DIR="$HOME/.local/share/man/man1"
if [ -d "$MAN_DIR" ]; then
    echo "✅ Man directory exists: $MAN_DIR"
    
    # Check for individual man page files
    for page in rfgrep rfgrep-search rfgrep-interactive rfgrep-list rfgrep-completions; do
        if [ -f "$MAN_DIR/$page.1.gz" ]; then
            echo "✅ $page.1.gz exists"
        elif [ -f "$MAN_DIR/$page.1" ]; then
            echo "✅ $page.1 exists (not compressed)"
        else
            echo "❌ $page.1.gz not found"
        fi
    done
else
    echo "❌ Man directory not found: $MAN_DIR"
fi

# Test man page content
echo "=== Testing Man Page Content ==="

# Test that main man page contains expected sections
if man rfgrep 2>/dev/null | grep -q "NAME"; then
    echo "✅ Main man page contains NAME section"
else
    echo "❌ Main man page missing NAME section"
fi

if man rfgrep 2>/dev/null | grep -q "SYNOPSIS"; then
    echo "✅ Main man page contains SYNOPSIS section"
else
    echo "❌ Main man page missing SYNOPSIS section"
fi

if man rfgrep 2>/dev/null | grep -q "DESCRIPTION"; then
    echo "✅ Main man page contains DESCRIPTION section"
else
    echo "❌ Main man page missing DESCRIPTION section"
fi

if man rfgrep 2>/dev/null | grep -q "EXAMPLES"; then
    echo "✅ Main man page contains EXAMPLES section"
else
    echo "❌ Main man page missing EXAMPLES section"
fi

# Test search man page content
if man rfgrep-search 2>/dev/null | grep -q "search"; then
    echo "✅ Search man page contains search command documentation"
else
    echo "❌ Search man page missing search command documentation"
fi

# Test interactive man page content
if man rfgrep-interactive 2>/dev/null | grep -q "interactive"; then
    echo "✅ Interactive man page contains interactive command documentation"
else
    echo "❌ Interactive man page missing interactive command documentation"
fi

echo ""
echo "=== Installation Verification ==="
echo "To verify man pages are properly installed:"
echo ""
echo "1. Check if man pages are in the correct location:"
echo "   ls -la ~/.local/share/man/man1/rfgrep*"
echo ""
echo "2. Verify MANPATH is set correctly:"
echo "   echo \$MANPATH"
echo ""
echo "3. Test man page access:"
echo "   man rfgrep"
echo "   man rfgrep-search"
echo "   man rfgrep-interactive"
echo "   man rfgrep-list"
echo "   man rfgrep-completions"
echo ""
echo "4. If man pages are not accessible, add to shell profile:"
echo "   echo 'export MANPATH=\$MANPATH:\$HOME/.local/share/man' >> ~/.bashrc"
echo "   source ~/.bashrc"

echo ""
echo "✅ All man page tests completed!" 