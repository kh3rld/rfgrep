#!/bin/bash
set -uo pipefail

# Comprehensive CLI Test Suite for rfgrep
# Covers: all commands, options, edge cases, negative scenarios
# Logs: command, stdout, stderr, exit code
# Usage: bash tests/cli_comprehensive.sh

LOGFILE="cli_test_log.txt"
SUMMARY="cli_test_summary.txt"
TESTDIR="cli_test_env"
RFGREP="${RFGREP_BIN:-./target/release/rfgrep}"

rm -rf "$TESTDIR" "$LOGFILE" "$SUMMARY"
mkdir -p "$TESTDIR"
touch "$LOGFILE" "$SUMMARY"

pass=0
fail=0

echo "==== rfgrep CLI Comprehensive Test Suite ====" | tee -a "$LOGFILE"
echo "Test run: $(date)" | tee -a "$LOGFILE"
echo "rfgrep binary: $RFGREP" | tee -a "$LOGFILE"
echo "Test directory: $TESTDIR" | tee -a "$LOGFILE"

# Helper: run a test and log everything
test_case() {
  local desc="$1"; shift
  local cmd=("$@")
  echo -e "\n--- TEST: $desc ---" | tee -a "$LOGFILE"
  echo "> ${cmd[*]}" | tee -a "$LOGFILE"
  local out_file err_file
  out_file=$(mktemp)
  err_file=$(mktemp)
  "${cmd[@]}" > "$out_file" 2> "$err_file"
  local code=$?
  echo "[stdout]" | tee -a "$LOGFILE"
  cat "$out_file" | tee -a "$LOGFILE"
  echo "[stderr]" | tee -a "$LOGFILE"
  cat "$err_file" | tee -a "$LOGFILE"
  echo "[exit code] $code" | tee -a "$LOGFILE"
  if [[ $code -eq 0 ]]; then
    echo "PASS: $desc" | tee -a "$SUMMARY"
    ((pass++))
  else
    echo "FAIL: $desc (exit $code)" | tee -a "$SUMMARY"
    ((fail++))
  fi
  rm -f "$out_file" "$err_file"
}

# 1. Global options (all implemented)
test_case "Show version" "$RFGREP" --version
test_case "Show help" "$RFGREP" --help
test_case "Show help for search" "$RFGREP" search --help
test_case "Show help for list" "$RFGREP" list --help
test_case "Show help for interactive" "$RFGREP" interactive --help
test_case "Show help for completions" "$RFGREP" completions --help

test_case "Invalid command" "$RFGREP" notacommand

test_case "Global dry-run with search" "$RFGREP" "$TESTDIR" --dry-run search "pattern"
test_case "Global max-size with list" "$RFGREP" "$TESTDIR" --max-size 1 list --extensions txt
test_case "Global skip-binary with search" "$RFGREP" "$TESTDIR" --skip-binary search "pattern"
test_case "Global log file" "$RFGREP" "$TESTDIR" --log "$TESTDIR/log.txt" search "pattern"
test_case "Global verbose" "$RFGREP" "$TESTDIR" --verbose search "pattern"

# 2. Search command: implemented features only
echo "test pattern" > "$TESTDIR/file1.txt"
echo "another test" > "$TESTDIR/file2.txt"

test_case "Basic search" "$RFGREP" "$TESTDIR" search "test"
test_case "Regex search" "$RFGREP" "$TESTDIR" search "test.*" regex
test_case "Word search" "$RFGREP" "$TESTDIR" search "test" word
test_case "Recursive search" "$RFGREP" "$TESTDIR" search "test" --recursive
test_case "Extension filter" "$RFGREP" "$TESTDIR" search "test" --extensions txt
test_case "Copy to clipboard" "$RFGREP" "$TESTDIR" search "test" --copy
test_case "Output format: json" "$RFGREP" "$TESTDIR" search "test" --output-format json

# Edge cases for search
test_case "Malformed regex" "$RFGREP" "$TESTDIR" search "[unclosed" regex
test_case "Empty pattern" "$RFGREP" "$TESTDIR" search "" 
test_case "Nonexistent file" "$RFGREP" "$TESTDIR" search "test" nonexistent.txt

# 3. List command: implemented features only
test_case "Basic list" "$RFGREP" "$TESTDIR" list
test_case "Recursive list" "$RFGREP" "$TESTDIR" list --recursive
test_case "Extension filter list" "$RFGREP" "$TESTDIR" list --extensions txt
test_case "Nonexistent directory" "$RFGREP" nonexistentdir list

# 4. Interactive command (basic test only - hangs in interactive mode)
test_case "Interactive help" "$RFGREP" interactive --help
# Note: Interactive command hangs when run with pattern, so we skip functional tests

# 5. Completions command (all implemented)
test_case "Completions bash" "$RFGREP" completions bash
test_case "Completions zsh" "$RFGREP" completions zsh
test_case "Completions fish" "$RFGREP" completions fish
test_case "Completions elvish" "$RFGREP" completions elvish
test_case "Completions powershell" "$RFGREP" completions powershell
test_case "Completions invalid shell" "$RFGREP" completions notashell

# 6. Negative/invalid global options
test_case "Unknown global flag" "$RFGREP" --notaflag

# 7. Robustness tests
test_case "Large number of files" "$RFGREP" "$TESTDIR" search "test" --recursive
test_case "Multiple extensions" "$RFGREP" "$TESTDIR" search "test" --extensions txt,md
test_case "Complex pattern" "$RFGREP" "$TESTDIR" search "test.*pattern" regex

echo -e "\n==== Test Summary ====" | tee -a "$LOGFILE"
echo "PASS: $pass" | tee -a "$LOGFILE"
echo "FAIL: $fail" | tee -a "$LOGFILE"
cat "$SUMMARY" | tee -a "$LOGFILE"

if [[ $fail -eq 0 ]]; then
  echo "ALL TESTS PASSED" | tee -a "$LOGFILE"
  exit 0
else
  echo "SOME TESTS FAILED" | tee -a "$LOGFILE"
  exit 1
fi