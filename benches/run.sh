#!/bin/bash
set -eo pipefail

TEST_DIR="bench_data"
RESULTS="results/$(date +%Y-%m-%d)"
mkdir -p "$RESULTS"

generate_data() {
  mkdir -p "$TEST_DIR"

  for i in {1..10000}; do
    head -c $((RANDOM % 10000 + 1000)) /dev/urandom > "$TEST_DIR/file$i.txt"
  done
  
  mkdir -p "$TEST_DIR/medium"
  for i in {1..100}; do
    head -c $((RANDOM % 900000 + 100000)) /dev/urandom > "$TEST_DIR/medium/file$i.dat"
  done
  
  mkdir -p "$TEST_DIR/large"
  for i in {1..5}; do
    head -c $((RANDOM % 40000000 + 10000000)) /dev/urandom > "$TEST_DIR/large/file$i.bin"
  done
}

run_benchmarks() {
  echo "Warming up..."
  rfgrep search "xyz123" "$TEST_DIR" >/dev/null 2>&1 || true

  hyperfine \
    --warmup 3 \
    --export-json "$RESULTS/search.json" \
    --export-markdown "$RESULTS/search.md" \
    "rfgrep search 'pattern1' '$TEST_DIR'" \
    "grep -r 'pattern1' '$TEST_DIR'" \
    "rg 'pattern1' '$TEST_DIR'" \
    "fd -X grep 'pattern1' '$TEST_DIR'"

  hyperfine \
    --export-json "$RESULTS/extensions.json" \
    "rfgrep search 'pattern' --extensions txt '$TEST_DIR'" \
    "rg 'pattern' -g '*.txt' '$TEST_DIR'"

  hyperfine \
    --export-json "$RESULTS/binary.json" \
    "rfgrep search 'pattern' --skip-binary '$TEST_DIR'" \
    "rg 'pattern' --binary '$TEST_DIR'"
}

profile_memory() {
  valgrind --tool=massif --stacks=yes \
    --massif-out-file="$RESULTS/massif.out" \
    ./target/release/rfgrep search "pattern" "$TEST_DIR"
  
  ms_print "$RESULTS/massif.out" > "$RESULTS/memory.txt"
}

profile_io() {
  strace -c -f -o "$RESULTS/strace.txt" \
    ./target/release/rfgrep search "pattern" "$TEST_DIR"
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