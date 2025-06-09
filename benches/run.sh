#!/bin/bash
set -eo pipefail

TEST_DIR="bench_data"
RESULTS="results/$(date +%Y-%m-%d)"

# Ensure rfgrep binary is built and executable
if [ ! -x ./target/release/rfgrep ]; then
  echo "Error: rfgrep binary not found or not executable at ./target/release/rfgrep"
  echo "Please build it with 'cargo build --release' before running this script."
  exit 1
fi

mkdir -p "$RESULTS"

generate_data() {
  mkdir -p "$TEST_DIR"

  echo "Generating small files..."
  for i in {1..10000}; do
    head -c $((RANDOM % 10000 + 1000)) /dev/urandom > "$TEST_DIR/file$i.txt"
  done
  
  echo "Generating medium files..."
  mkdir -p "$TEST_DIR/medium"
  for i in {1..100}; do
    head -c $((RANDOM % 900000 + 100000)) /dev/urandom > "$TEST_DIR/medium/file$i.dat"
  done
  
  echo "Generating large files..."
  mkdir -p "$TEST_DIR/large"
  for i in {1..5}; do
    head -c $((RANDOM % 40000000 + 10000000)) /dev/urandom > "$TEST_DIR/large/file$i.bin"
  done
}

run_benchmarks() {
  echo "Warming up..."
  ./target/release/rfgrep "$TEST_DIR" search "xyz123" >/dev/null 2>&1 || true

  hyperfine --ignore-failure \
    --warmup 3 \
    --export-json "$RESULTS/search.json" \
    --export-markdown "$RESULTS/search.md" \
    "./target/release/rfgrep '$TEST_DIR' search 'pattern1'" \
    "grep -r 'pattern1' '$TEST_DIR'" \
    "rg 'pattern1' '$TEST_DIR'" \
    "fd -X grep 'pattern1' '$TEST_DIR'"

  hyperfine --ignore-failure \
    --export-json "$RESULTS/extensions.json" \
    "./target/release/rfgrep '$TEST_DIR' search 'pattern' --extensions txt" \
    "rg 'pattern' -g '*.txt' '$TEST_DIR'"

  hyperfine --ignore-failure \
    --export-json "$RESULTS/binary.json" \
    "./target/release/rfgrep '$TEST_DIR' search 'pattern' --skip-binary" \
    "rg 'pattern' --binary '$TEST_DIR'"
}

profile_memory() {
  echo "Profiling memory usage with valgrind massif..."
  valgrind --tool=massif --stacks=yes \
    --massif-out-file="$RESULTS/massif.out" \
    ./target/release/rfgrep "$TEST_DIR" search "pattern"
  
  ms_print "$RESULTS/massif.out" > "$RESULTS/memory.txt"
}

profile_io() {
  echo "Profiling I/O with strace..."
  strace -c -f -o "$RESULTS/strace.txt" \
    ./target/release/rfgrep "$TEST_DIR" search "pattern"
}

main() {
  generate_data
  run_benchmarks
  profile_memory
  profile_io
  
  echo "Results saved to $RESULTS/"
  tree "$RESULTS"
}

main
