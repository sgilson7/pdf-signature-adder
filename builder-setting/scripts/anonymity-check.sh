#!/usr/bin/env bash
# List every line that would identify the author in an anonymous artifact.
#
#   ./scripts/anonymity-check.sh <exported directory>
#
# Exits non-zero if it finds one. Run it on an export, not on this
# repository, which is public under its author's name on purpose.
set -uo pipefail
DIR="${1:?usage: anonymity-check.sh <directory>}"
PATTERN='Gilson|sgilson|samgilson|ncsu|North Carolina|NC State|orcid\.org|github\.com/sgilson7|sgilson7\.github\.io'
if grep -rniE "$PATTERN" "$DIR" --exclude-dir=.git --exclude-dir=target --exclude-dir=dist --exclude-dir=.venv; then
  echo "IDENTIFYING LINES FOUND: remove or neutralise each one above."
  exit 1
fi
echo "No identifying line found for: $PATTERN"
