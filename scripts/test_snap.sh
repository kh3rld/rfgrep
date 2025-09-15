#!/usr/bin/env bash
set -euo pipefail

# Smoke test for a locally built rfgrep snap.
# Usage:
#   scripts/test_snap.sh <path-to-snap>
# If not provided, the script will try to auto-detect the first rfgrep_*.snap in repo root.

SNAP_FILE="${1:-}"
if [[ -z "$SNAP_FILE" ]]; then
  SNAP_FILE=$(ls -1 rfgrep_*_*.snap 2>/dev/null | head -n1 || true)
fi

if [[ -z "$SNAP_FILE" || ! -f "$SNAP_FILE" ]]; then
  echo "Snap file not found. Pass it explicitly: scripts/test_snap.sh <rfgrep_*.snap>" >&2
  exit 1
fi

echo "==> Installing snap: $SNAP_FILE"
# Remove any existing installation to avoid channel constraints during --dangerous install
if snap list | grep -q '^rfgrep\s'; then
  sudo snap remove rfgrep || true
fi
sudo snap install --dangerous "$SNAP_FILE"

echo "==> Running basic checks..."
# Ensure the command resolves and prints a version
snap run rfgrep --version || { echo "ERROR: rfgrep --version failed" >&2; exit 1; }

# Create a tiny temp workspace and search a known pattern
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT
printf '%s\n' "hello world" > "$TMPDIR/a.txt"

# Exercise the search subcommand (as used in the codebase)
snap run rfgrep search "hello" "$TMPDIR" >/dev/null || { echo "ERROR: basic search failed" >&2; exit 1; }

echo "==> OK: rfgrep snap installed and basic search works."
