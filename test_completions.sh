#!/bin/bash

# Test script for rfgrep shell completions
# This script tests the completion functionality for different shells

echo "Testing rfgrep shell completions..."

# Test bash completion
echo "=== Testing Bash Completion ==="
bash_completion=$(./target/release/rfgrep completions bash)
if [ $? -eq 0 ]; then
    echo "✅ Bash completion generated successfully"
    echo "Generated completion script length: $(echo "$bash_completion" | wc -l) lines"
else
    echo "❌ Bash completion generation failed"
fi

# Test zsh completion
echo "=== Testing Zsh Completion ==="
zsh_completion=$(./target/release/rfgrep completions zsh)
if [ $? -eq 0 ]; then
    echo "✅ Zsh completion generated successfully"
    echo "Generated completion script length: $(echo "$zsh_completion" | wc -l) lines"
else
    echo "❌ Zsh completion generation failed"
fi

# Test fish completion
echo "=== Testing Fish Completion ==="
fish_completion=$(./target/release/rfgrep completions fish)
if [ $? -eq 0 ]; then
    echo "✅ Fish completion generated successfully"
    echo "Generated completion script length: $(echo "$fish_completion" | wc -l) lines"
else
    echo "❌ Fish completion generation failed"
fi

# Test powershell completion
echo "=== Testing PowerShell Completion ==="
powershell_completion=$(./target/release/rfgrep completions powershell)
if [ $? -eq 0 ]; then
    echo "✅ PowerShell completion generated successfully"
    echo "Generated completion script length: $(echo "$powershell_completion" | wc -l) lines"
else
    echo "❌ PowerShell completion generation failed"
fi

# Test elvish completion
echo "=== Testing Elvish Completion ==="
elvish_completion=$(./target/release/rfgrep completions elvish)
if [ $? -eq 0 ]; then
    echo "✅ Elvish completion generated successfully"
    echo "Generated completion script length: $(echo "$elvish_completion" | wc -l) lines"
else
    echo "❌ Elvish completion generation failed"
fi

echo ""
echo "=== Completion Features Tested ==="

# Check for specific completion features in bash
if echo "$bash_completion" | grep -q "search interactive list completions help"; then
    echo "✅ Bash: Command completion available"
else
    echo "❌ Bash: Command completion missing"
fi

if echo "$bash_completion" | grep -q "boyer-moore regex simple"; then
    echo "✅ Bash: Algorithm completion available"
else
    echo "❌ Bash: Algorithm completion missing"
fi

if echo "$bash_completion" | grep -q "text json xml html markdown"; then
    echo "✅ Bash: Output format completion available"
else
    echo "❌ Bash: Output format completion missing"
fi

# Check for specific completion features in zsh
if echo "$zsh_completion" | grep -q "search:Search for patterns"; then
    echo "✅ Zsh: Command descriptions available"
else
    echo "❌ Zsh: Command descriptions missing"
fi

if echo "$zsh_completion" | grep -q "boyer-moore.*Boyer-Moore algorithm"; then
    echo "✅ Zsh: Algorithm descriptions available"
else
    echo "❌ Zsh: Algorithm descriptions missing"
fi

# Check for specific completion features in fish
if echo "$fish_completion" | grep -q "search.*Search for patterns"; then
    echo "✅ Fish: Command descriptions available"
else
    echo "❌ Fish: Command descriptions missing"
fi

if echo "$fish_completion" | grep -q "boyer-moore.*Boyer-Moore algorithm"; then
    echo "✅ Fish: Algorithm descriptions available"
else
    echo "❌ Fish: Algorithm descriptions missing"
fi

echo ""
echo "=== Installation Instructions ==="
echo "To install completions:"
echo ""
echo "Bash:"
echo "  ./target/release/rfgrep completions bash >> ~/.bashrc"
echo "  source ~/.bashrc"
echo ""
echo "Zsh:"
echo "  mkdir -p ~/.zsh/completions"
echo "  ./target/release/rfgrep completions zsh > ~/.zsh/completions/_rfgrep"
echo "  echo 'fpath=(~/.zsh/completions \$fpath)' >> ~/.zshrc"
echo "  autoload -U compinit && compinit"
echo ""
echo "Fish:"
echo "  ./target/release/rfgrep completions fish > ~/.config/fish/completions/rfgrep.fish"
echo ""
echo "PowerShell:"
echo "  ./target/release/rfgrep completions powershell > rfgrep-completion.ps1"
echo "  . rfgrep-completion.ps1"

echo ""
echo "✅ All completion tests completed!" 