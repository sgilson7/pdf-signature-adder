#!/usr/bin/env bash
# Publish examples/minimal to the gh-pages branch, which GitHub Pages serves
# at https://<owner>.github.io/builder-setting/. A person's action (stage 8):
# run it after ./scripts/check.sh passes, then run the browser check against
# the live origin:
#
#   BUILDER_ORIGIN=https://<owner>.github.io/builder-setting \
#     examples/minimal/.venv/bin/python examples/minimal/tests/browser/check.py
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REMOTE="$(git -C "$ROOT" remote get-url origin)"
COMMIT="$(git -C "$ROOT" rev-parse --short HEAD)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/builder-pages.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

(cd "$ROOT/examples/minimal" && ./scripts/build.sh >/dev/null)
cp -R "$ROOT/examples/minimal/dist/." "$WORK/"
touch "$WORK/.nojekyll"
cd "$WORK"
git init -q -b gh-pages
git add -A
git -c user.name="$(git -C "$ROOT" config user.name)" -c user.email="$(git -C "$ROOT" config user.email)" \
  commit -q -m "Publish examples/minimal from $COMMIT"
git push -q -f "$REMOTE" gh-pages
echo "pushed examples/minimal from $COMMIT to gh-pages"
