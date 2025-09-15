#!/usr/bin/env bash
set -euo pipefail

# Simple helper to build the rfgrep snap locally.
# Requirements: snapcraft installed. For local builds use LXD; for cloud builds use `snapcraft remote-build`.
# Usage:
#   scripts/build_snap.sh            # try local build with LXD (default)
#   scripts/build_snap.sh remote     # use snapcraft remote-build
#   scripts/build_snap.sh lxd        # force local LXD build

MODE=${1:-lxd}

echo "==> Building rfgrep snap (mode: ${MODE})"

if ! command -v snapcraft >/dev/null 2>&1; then
  echo "Error: snapcraft is not installed. Install it via: sudo snap install snapcraft --classic" >&2
  exit 1
fi

if [[ "$MODE" == "remote" ]]; then
  echo "==> Using snapcraft remote-build (requires a Snapcraft account)"
  snapcraft remote-build
else
  echo "==> Using snapcraft --use-lxd (requires LXD installed and initialized)"
  echo "If LXD is not installed, run: sudo snap install lxd && sudo lxd init"
  snapcraft --use-lxd
fi

echo "==> Build complete. Look for the generated .snap in the project root."
