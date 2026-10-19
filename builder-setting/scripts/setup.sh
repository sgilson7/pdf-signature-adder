#!/usr/bin/env bash
# One-time setup for the reference repository's check: one Python virtual
# environment with Playwright and three browser engines, shared by the
# template and the example.
set -euo pipefail
cd "$(dirname "$0")/.."
python3 -m venv .venv
.venv/bin/pip install --quiet --upgrade pip
.venv/bin/pip install --quiet "playwright==1.63.0"
.venv/bin/python -m playwright install ${PLAYWRIGHT_WITH_DEPS:+--with-deps} chromium firefox webkit
