#!/bin/bash
set -uo pipefail

# Robustness and Stress Test Suite for rfgrep
# Tests: large files, rapid execution, parallel processing, memory usage, edge cases
# Usage: bash tests/robustness_stress.sh

LOGFILE="robustness_test_log.txt"
SUMMARY="robustness_test_summary.txt"
TESTDIR="robustness_test_env"
RFGREP="${RFGREP_BIN:-./target/release/rfgrep}"

rm -rf "$TESTDIR" "$LOGFILE" "$SUMMARY"
mkdir -p "$TESTDIR"
touch "$LOGFILE" "$SUMMARY"

pass=0
fail=0

echo "==== rfgrep Robustness & Stress Test Suite ====" | tee -a "$LOGFILE"
echo "Test run: $(date)" | tee -a "$LOGFILE"
echo "rfgrep binary: $RFGREP" | tee -a "$LOGFILE"

# Helper: run a test and log everything
test_case() {
  local desc="$1"; shift
  local cmd=("$@")
  echo -e "\n--- TEST: $desc ---" | tee -a "$LOGFILE"
  echo "> ${cmd[*]}" | tee -a "$LOGFILE"
  local out_file err_file
  out_file=$(mktemp)
  err_file=$(mktemp)
  time timeout 30s "${cmd[@]}" > "$out_file" 2> "$err_file"
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

# 1. Large File Tests
echo "Creating large test files..." | tee -a "$LOGFILE"

# Create a 10MB file with repeated patterns
echo "Creating 10MB test file..." | tee -a "$LOGFILE"
for i in {1..1000}; do
  echo "This is line $i with some test pattern that we will search for repeatedly" >> "$TESTDIR/large_file.txt"
  echo "Another line with different content and more test patterns scattered throughout" >> "$TESTDIR/large_file.txt"
  echo "Line $i continues with more test data and patterns to find" >> "$TESTDIR/large_file.txt"
done

# Create a file with many different extensions
echo "Creating multi-extension test files..." | tee -a "$LOGFILE"
for ext in txt md rs py js html css json xml yaml toml ini cfg; do
  echo "test pattern in $ext file" > "$TESTDIR/test.$ext"
  echo "another test pattern" >> "$TESTDIR/test.$ext"
done

# Create deeply nested directory structure
echo "Creating nested directory structure..." | tee -a "$LOGFILE"
mkdir -p "$TESTDIR/deep/nested/structure/with/many/levels"
echo "test pattern in deep file" > "$TESTDIR/deep/nested/structure/with/many/levels/deep_file.txt"

# Create files with special characters
echo "Creating files with special characters..." | tee -a "$LOGFILE"
echo "test pattern with spaces and special chars: !@#$%^&*()" > "$TESTDIR/file with spaces.txt"
echo "test pattern with unicode: 🚀🌟✨" > "$TESTDIR/unicode_file.txt"
echo "test pattern with quotes: 'single' and \"double\"" > "$TESTDIR/quotes_file.txt"

# 2. Large File Processing Tests
test_case "Large file search (10MB)" "$RFGREP" "$TESTDIR" search "test pattern" --recursive
test_case "Large file with regex" "$RFGREP" "$TESTDIR" search "test.*pattern" regex --recursive
test_case "Large file with word boundaries" "$RFGREP" "$TESTDIR" search "test" word --recursive

# 3. Memory and Performance Tests
test_case "Memory usage with large files" "$RFGREP" "$TESTDIR" --max-size 100 search "pattern" --recursive
test_case "Skip binary files performance" "$RFGREP" "$TESTDIR" --skip-binary search "pattern" --recursive
test_case "Dry run with large dataset" "$RFGREP" "$TESTDIR" --dry-run search "pattern" --recursive

# 4. Rapid Execution Tests
echo "Running rapid execution tests..." | tee -a "$LOGFILE"
for i in {1..10}; do
  test_case "Rapid execution $i" "$RFGREP" "$TESTDIR" search "test" --extensions txt
done

# 5. Parallel Execution Tests
echo "Running parallel execution tests..." | tee -a "$LOGFILE"
for i in {1..5}; do
  "$RFGREP" "$TESTDIR" search "test" --extensions txt > "/dev/null" 2>&1 &
done
wait
test_case "Parallel execution completed" echo "Parallel tests finished"

# 6. Complex Argument Combinations
test_case "Complex search with all options" "$RFGREP" "$TESTDIR" --verbose --skip-binary --max-size 50 search "test.*pattern" regex --recursive --extensions txt,md --copy
test_case "Complex list with all options" "$RFGREP" "$TESTDIR" --verbose --skip-binary --max-size 50 list --recursive --extensions txt,md
test_case "Multiple extensions filter" "$RFGREP" "$TESTDIR" search "test" --extensions txt,md,rs,py,js,html,css,json,xml,yaml,toml,ini,cfg
test_case "Deep directory search" "$RFGREP" "$TESTDIR" search "test" --recursive

# 7. Edge Cases and Stress Tests
test_case "Empty directory search" "$RFGREP" "$TESTDIR/empty" search "test"
test_case "Very long pattern" "$RFGREP" "$TESTDIR" search "$(printf 'a%.0s' {1..1000})"
test_case "Special characters in pattern" "$RFGREP" "$TESTDIR" search "!@#$%^&*()" 
test_case "Unicode pattern" "$RFGREP" "$TESTDIR" search "🚀🌟✨"
test_case "Quotes in pattern" "$RFGREP" "$TESTDIR" search "'single' and \"double\""

# 8. File System Edge Cases
test_case "Symlink handling" ln -sf "$TESTDIR/large_file.txt" "$TESTDIR/symlink.txt" && "$RFGREP" "$TESTDIR" search "test" --recursive
test_case "Hidden files" "$RFGREP" "$TESTDIR" search "test" --recursive
test_case "Files with dots" echo "test pattern" > "$TESTDIR/file.with.dots.txt" && "$RFGREP" "$TESTDIR" search "test" --recursive

# 9. Resource Usage Tests
echo "Testing resource usage..." | tee -a "$LOGFILE"
test_case "Memory usage monitoring" /usr/bin/time -v "$RFGREP" "$TESTDIR" search "test" --recursive 2>&1 | head -20
test_case "CPU usage under load" "$RFGREP" "$TESTDIR" search "test" --recursive --extensions txt,md,rs,py,js,html,css,json,xml,yaml,toml,ini,cfg

# 10. Error Recovery Tests
test_case "Recovery from malformed input" "$RFGREP" "$TESTDIR" search "[" regex
test_case "Recovery from invalid path" "$RFGREP" "/nonexistent/path" search "test"
test_case "Recovery from permission denied" "$RFGREP" "/root" search "test" 2>/dev/null || true

# 11. Concurrent Access Tests
echo "Testing concurrent access..." | tee -a "$LOGFILE"
for i in {1..3}; do
  "$RFGREP" "$TESTDIR" search "test" --extensions txt > "/dev/null" &
  "$RFGREP" "$TESTDIR" list --extensions txt > "/dev/null" &
  "$RFGREP" "$TESTDIR" search "pattern" --extensions md > "/dev/null" &
done
wait
test_case "Concurrent access completed" echo "Concurrent tests finished"

# 12. Extreme Pattern Tests
test_case "Very complex regex" "$RFGREP" "$TESTDIR" search "test.*pattern.*with.*many.*groups" regex
test_case "Backreference regex" "$RFGREP" "$TESTDIR" search "(test).*\\1" regex
test_case "Quantifier stress" "$RFGREP" "$TESTDIR" search "t{1,100}e{1,100}s{1,100}t{1,100}" regex

# 13. Output Format Stress Tests
test_case "JSON output stress" "$RFGREP" "$TESTDIR" search "test" --output-format json
test_case "Large result set" "$RFGREP" "$TESTDIR" search "test" --recursive --extensions txt,md,rs,py,js,html,css,json,xml,yaml,toml,ini,cfg

# 14. Logging Stress Tests
test_case "Verbose logging stress" "$RFGREP" "$TESTDIR" --verbose --log "$TESTDIR/stress.log" search "test" --recursive
test_case "Log file rotation" "$RFGREP" "$TESTDIR" --log "$TESTDIR/rotated.log" search "test" --recursive

echo -e "\n==== Robustness Test Summary ====" | tee -a "$LOGFILE"
echo "PASS: $pass" | tee -a "$LOGFILE"
echo "FAIL: $fail" | tee -a "$LOGFILE"
cat "$SUMMARY" | tee -a "$LOGFILE"

# Cleanup
rm -rf "$TESTDIR"

if [[ $fail -eq 0 ]]; then
  echo "ALL ROBUSTNESS TESTS PASSED" | tee -a "$LOGFILE"
  exit 0
else
  echo "SOME ROBUSTNESS TESTS FAILED" | tee -a "$LOGFILE"
  exit 1
fi
