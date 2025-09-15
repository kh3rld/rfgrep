#!/usr/bin/env bash
set -euo pipefail

# Helper to publish a built rfgrep snap to the Snap Store.
# Usage:
#   scripts/publish_snap.sh <path-to-snap> <channel>
# Example:
#   scripts/publish_snap.sh rfgrep_0.2.1_amd64.snap edge
#   scripts/publish_snap.sh rfgrep_0.2.1_amd64.snap stable
#
# Authentication:
# - Run `snapcraft login` interactively once, OR
# - Provide a login export file path in $SNAPCRAFT_LOGIN_FILE (from `snapcraft export-login`), OR
# - Provide exported credentials via $SNAPCRAFT_STORE_CREDENTIALS and we will pipe it to `snapcraft login --with -`.

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <path-to-snap> <channel>" >&2
  exit 1
fi

SNAP_FILE=$1
CHANNEL=$2

if ! command -v snapcraft >/dev/null 2>&1; then
  echo "Error: snapcraft is not installed. Install it via: sudo snap install snapcraft --classic" >&2
  exit 1
fi

# Attempt non-interactive login if credentials provided
if [[ -n "${SNAPCRAFT_LOGIN_FILE:-}" ]]; then
  echo "==> Logging in with login file: $SNAPCRAFT_LOGIN_FILE"
  snapcraft login --with "$SNAPCRAFT_LOGIN_FILE" || true
elif [[ -n "${SNAPCRAFT_STORE_CREDENTIALS:-}" ]]; then
  echo "==> Logging in with credentials from SNAPCRAFT_STORE_CREDENTIALS env var"
  printf "%s" "$SNAPCRAFT_STORE_CREDENTIALS" | snapcraft login --with - || true
else
  echo "==> No non-interactive credentials provided. Assuming you are already logged in (snapcraft login)."
fi

if [[ ! -f "$SNAP_FILE" ]]; then
  echo "Error: snap file not found: $SNAP_FILE" >&2
  exit 1
fi

echo "==> Uploading $SNAP_FILE to channel: $CHANNEL"
snapcraft upload --release="$CHANNEL" "$SNAP_FILE"

echo "==> Upload complete. Check the review status in the Snap Store dashboard."
