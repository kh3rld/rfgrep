#!/usr/bin/env bash
# Wrapper so existing test harness that calls `rfgrep` (with PATH default) can run
# It forwards all args but prefixes with bench_data if the first arg doesn't look like a subcommand
# If the first arg is a subcommand, leave as-is.

BIN="$PWD/target/debug/rfgrep"
if [ ! -x "$BIN" ]; then
  echo "Built binary not found at $BIN" >&2
  exit 1
fi

# If first argument looks like a subcommand (search, list, interactive, completions, help), run directly
case "$1" in
  search|list|interactive|completions|help|-h|--help)
    exec "$BIN" "$@"
    ;;
  *)
    # Otherwise assume first arg is a path and pass through
    exec "$BIN" "$@"
    ;;
esac
