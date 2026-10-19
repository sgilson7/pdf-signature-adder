#!/usr/bin/env bash
# Optional audit, outside check.sh (kits/privacy): every planted leak must be
# caught by the core tests or the browser check. Takes minutes.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PY="${BUILDER_PYTHON:-$ROOT/.venv/bin/python}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

leaks=$(python3 "$ROOT/scripts/plant_leak.py" --list) || { echo "could not list the leaks"; exit 1; }
# An empty list would pass by proving nothing.
[ -n "$leaks" ] || { echo "no leaks to plant: the proof would prove nothing"; exit 1; }

copy() {
  # The copy keeps its own target/ between leaks, so only the first build is slow.
  rsync -a --delete --exclude .git --exclude .venv --exclude target --exclude builder-setting "$ROOT/" "$WORK/copy/"
}

# The unplanted copy must pass both checks, or a "caught" below proves nothing.
copy
(cd "$WORK/copy" && cargo test -q --test boundary --test no_storage >/dev/null 2>&1) \
  || { echo "the clean copy fails its core tests: the proof cannot run"; exit 1; }
(cd "$WORK/copy" && ./scripts/build.sh >/dev/null 2>&1) || { echo "the clean copy does not build"; exit 1; }
(cd "$WORK/copy" && BUILDER_ENGINES=chromium BUILDER_IDLE=1 "$PY" tests/browser/check.py 2>&1 | grep -q "BROWSER CHECK PASSED") \
  || { echo "the clean copy fails the browser check: the proof cannot run"; exit 1; }
echo "  clean copy passes"

missed=0
for leak in $leaks; do
  copy
  python3 "$ROOT/scripts/plant_leak.py" "$WORK/copy" "$leak" || { echo "could not plant $leak"; missed=1; continue; }
  caught=""
  (cd "$WORK/copy" && cargo test -q --test boundary --test no_storage >/dev/null 2>&1) || caught="core tests"
  if (cd "$WORK/copy" && ./scripts/build.sh >/dev/null 2>&1); then
    out=$(cd "$WORK/copy" && BUILDER_ENGINES=chromium BUILDER_IDLE=1 "$PY" tests/browser/check.py 2>&1)
    grep -q "BROWSER CHECK PASSED" <<<"$out" || caught="${caught:+$caught; }browser check"
  else
    echo "  $leak: the planted copy does not build"; missed=1; continue
  fi
  if [ -n "$caught" ]; then echo "  caught $leak by $caught"; else echo "  MISSED $leak"; missed=1; fi
done
[ "$missed" = 0 ] && echo "PRIVACY PROOF PASSED" || { echo "PRIVACY PROOF FAILED"; exit 1; }
