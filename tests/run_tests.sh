#!/bin/bash
set -eo pipefail

TEST_DATA_DIR="bench_data"

if [ ! -d "$TEST_DATA_DIR" ]; then
  echo "Error: Test data directory '$TEST_DATA_DIR' not found."
  echo "Please generate test data first."
  exit 1
fi

echo "Running README.md example test cases..."

echo "[Test 1/16] Basic Search"
rfgrep search "pattern" "$TEST_DATA_DIR"
echo "------------------------------------"

echo "[Test 2/16] Search with Options"
rfgrep search "pattern" --mode regex --extensions txt,dat --max-size 5 --skip-binary --copy "$TEST_DATA_DIR"
echo "------------------------------------"

echo "[Test 3/16] File Listing - Simple"
rfgrep list "$TEST_DATA_DIR"
echo "------------------------------------"

echo "[Test 4/16] File Listing - Detailed and Recursive"
rfgrep list --long --recursive "$TEST_DATA_DIR"
echo "------------------------------------"

echo "[Test 5/16] File Listing - With Filters"
rfgrep list --extensions txt,dat --max-size 10 --show-hidden "$TEST_DATA_DIR"
echo "------------------------------------"

echo "[Test 6/16] Example Search - 'HashMap' in .txt files"
rfgrep search "HashMap" --extensions txt "$TEST_DATA_DIR"
echo "------------------------------------"

echo "[Test 7/16] Example List - .txt files under 1MB"
rfgrep list --extensions txt --max-size 1 "$TEST_DATA_DIR"
echo "------------------------------------"

echo "[Test 8/16] Example Search - Regex and Copy"
rfgrep search "file\d+" --mode regex --copy "$TEST_DATA_DIR"
echo "------------------------------------"

echo "Running tests for Command Reference options..."

LOG_FILE_SEARCH="search_test.log"
rm -f "$LOG_FILE_SEARCH"
echo "[Test 9/16] Global Option --log with search"
rfgrep search "some_content_for_log_test" --log "$LOG_FILE_SEARCH" "$TEST_DATA_DIR"
if [ -f "$LOG_FILE_SEARCH" ]; then
  echo "Log file '$LOG_FILE_SEARCH' created."
  rm -f "$LOG_FILE_SEARCH"
else
  echo "Error: Log file '$LOG_FILE_SEARCH' not created."
fi
echo "------------------------------------"

LOG_FILE_LIST="list_test.log"
rm -f "$LOG_FILE_LIST"
echo "[Test 10/16] Global Option --log with list"
rfgrep list --log "$LOG_FILE_LIST" "$TEST_DATA_DIR"
if [ -f "$LOG_FILE_LIST" ]; then
  echo "Log file '$LOG_FILE_LIST' created."
  rm -f "$LOG_FILE_LIST"
else
  echo "Error: Log file '$LOG_FILE_LIST' not created."
fi
echo "------------------------------------"

echo "[Test 11/16] Global Option --path with search"
rfgrep search "another_pattern_for_path_test" --path "$TEST_DATA_DIR"
echo "------------------------------------"

echo "[Test 12/16] Global Option --path with list"
rfgrep list --path "$TEST_DATA_DIR"
echo "------------------------------------"

echo "[Test 13/16] Search Command --mode text"
rfgrep search "specific text pattern" --mode text "$TEST_DATA_DIR"
echo "------------------------------------"

echo "[Test 14/16] Search Command --mode word"
rfgrep search "pattern" --mode word "$TEST_DATA_DIR"
echo "------------------------------------"

echo "[Test 15/16] Search Command --dry-run"
rfgrep search "pattern_for_dry_run" --dry-run "$TEST_DATA_DIR"
echo "------------------------------------"

echo "[Test 16/16] List Command --skip-binary"
rfgrep list --skip-binary "$TEST_DATA_DIR"
echo "------------------------------------"

echo "All test cases completed."
