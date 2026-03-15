#!/usr/bin/env bash
# Reset brute-force login lockout (dev only).
# The daemon must be running with NYLON_DEV=1 for this to work.
#
# Usage:
#   ./scripts/reset-lockout.sh              # default: localhost:9450
#   ./scripts/reset-lockout.sh 192.168.1.1  # custom host

set -euo pipefail

HOST="${1:-localhost}"
URL="http://${HOST}:9450/api/v1/auth/reset-lockout"

echo "Resetting login lockout at ${URL} ..."

RESP=$(curl -s -w "\n%{http_code}" -X POST "$URL")
CODE=$(echo "$RESP" | tail -1)
BODY=$(echo "$RESP" | head -1)

if [ "$CODE" = "200" ]; then
    echo "Done — all lockouts cleared."
else
    echo "Failed (HTTP ${CODE}): ${BODY}"
    echo ""
    echo "Make sure the daemon is running with NYLON_DEV=1:"
    echo "  NYLON_DEV=1 cargo run -p nylon-wall-daemon"
    exit 1
fi
