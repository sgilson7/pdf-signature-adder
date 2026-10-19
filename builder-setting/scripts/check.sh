#!/usr/bin/env bash
# The reference repository's full check. CI runs this same file.
#
# 1. the template and the example agree on the files a project inherits
# 2. every relative link in the Markdown resolves
# 3. the default style pack stays inside template/styles/
# 4. the blank template passes its own check, and each of its breaks is caught
# 5. the minimal example passes its own check, and each of its breaks is caught
# 6. each planted defect is caught by the check meant to catch it
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
step() { printf '\n\033[1m#### %s\033[0m\n' "$*"; }

PY="${BUILDER_PYTHON:-}"
if [ -z "$PY" ]; then
  if [ -x "$ROOT/.venv/bin/python" ]; then PY="$ROOT/.venv/bin/python"; else PY=python3; fi
fi
export BUILDER_PYTHON="$PY"

step "sync: template and example"
./scripts/check-sync.sh

step "links"
"$PY" scripts/check-links.py

step "style leak: scripts/check-style-leak.py"
"$PY" scripts/check-style-leak.py

step "template: ./scripts/check.sh"
(cd template && ./scripts/check.sh)

step "template: scripts/break_tests.py"
(cd template && "$PY" scripts/break_tests.py)

step "examples/minimal: ./scripts/check.sh"
(cd examples/minimal && ./scripts/check.sh)

step "examples/minimal: scripts/break_tests.py"
(cd examples/minimal && "$PY" scripts/break_tests.py)

step "planted defects: scripts/prove-checks.sh"
./scripts/prove-checks.sh

printf '\n\033[1mREFERENCE REPOSITORY CHECK PASSED\033[0m\n'
