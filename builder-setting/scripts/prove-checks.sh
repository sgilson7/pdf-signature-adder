#!/usr/bin/env bash
# Show that each Builder check can fail.
#
# A check that has only been seen passing has not been shown to check
# anything. For each planted defect below, this script copies
# examples/minimal into a scratch directory, plants the defect, runs the one
# check meant to catch it, and requires that check to FAIL with the message
# that names the defect. A planted defect that passes is a broken check, and
# this script exits non-zero.
#
#   ./scripts/prove-checks.sh
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/examples/minimal"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/builder-prove.XXXXXX")"
export CARGO_TARGET_DIR="$WORK/target"   # shared, so each copy rebuilds little
trap 'rm -rf "$WORK"' EXIT

PY="${BUILDER_PYTHON:-}"
if [ -z "$PY" ]; then
  for c in "$SRC/.venv/bin/python" "$ROOT/template/.venv/bin/python" "$ROOT/.venv/bin/python"; do
    if [ -x "$c" ]; then PY="$c"; break; fi
  done
  PY="${PY:-python3}"
fi
case "$PY" in /*) ;; *) PY="$(cd "$(dirname "$PY")" && pwd)/$(basename "$PY")" ;; esac

pass=0; fail=0
fresh() {
  rm -rf "$WORK/p"; mkdir -p "$WORK/p"
  (cd "$SRC" && tar --exclude=./target --exclude=./dist --exclude=./.venv -cf - .) | (cd "$WORK/p" && tar -xf -)
  # the example's journal sits beside it, as a project's does
  rm -rf "$WORK/p-journal"; cp -R "$ROOT/examples/minimal-journal" "$WORK/p-journal"
  # build.sh reads the module from target/; point it at the shared target dir
  ln -s "$CARGO_TARGET_DIR" "$WORK/p/target"
}

# expect_failure NAME EXPECTED-TEXT COMMAND...
expect_failure() {
  local name="$1" expected="$2"; shift 2
  local out status=0
  out="$(cd "$WORK/p" && "$@" 2>&1)" || status=$?
  if [ "$status" -ne 0 ] && grep -qF -- "$expected" <<<"$out"; then
    printf '  caught   %s\n' "$name"; pass=$((pass + 1))
  else
    printf '  MISSED   %s (exit %s; expected text: %s)\n' "$name" "$status" "$expected"
    printf '%s\n' "$out" | tail -15 | sed 's/^/           | /'
    fail=$((fail + 1))
  fi
}

boundary() { cargo test -q -p tool_core --test boundary; }
browser() { ./scripts/build.sh >/dev/null && BUILDER_ENGINES=chromium BUILDER_IDLE=1 "$PY" tests/browser/check.py; }
browser_planted() { BUILDER_ENGINES=chromium BUILDER_IDLE=1 "$PY" tests/browser/check.py; }

echo "Planted defects, each against the check meant to catch it:"

fresh
cat >> "$WORK/p/crates/core/src/counter.rs" <<'EOF'
pub fn half(x: u32) -> f64 { x as f64 / 2.0 }
EOF
expect_failure "a float in the core" "(float)" boundary

fresh
cat >> "$WORK/p/crates/core/src/counter.rs" <<'EOF'
pub fn shout() { println!("hello"); }
EOF
expect_failure "printing from the core" "(input/output)" boundary

fresh
cat >> "$WORK/p/crates/core/src/counter.rs" <<'EOF'
pub fn index() -> std::collections::HashMap<u32, u32> { Default::default() }
EOF
expect_failure "a hash map in the core" "(hash map)" boundary

fresh
printf '\n[dev-dependencies]\nwasm-bindgen = "=0.2.127"\n' >> "$WORK/p/crates/core/Cargo.toml"
expect_failure "a dependency outside the allowlist" "dependencies outside the allowlist" boundary

fresh
"$PY" - "$WORK/p/crates/shim/src/lib.rs" <<'EOF'
import sys; p = sys.argv[1]; s = open(p).read()
s = s.replace("tool_core::api::act(settings_json, state_json, action)",
              'if action == "add" { tool_core::api::act(settings_json, state_json, "reset") } else { tool_core::api::act(settings_json, state_json, action) }')
open(p, "w").write(s)
EOF
expect_failure "a decision in the shim" "the shim makes decisions" boundary

fresh
"$PY" - "$WORK/p/web/app.js" <<'EOF'
import sys; p = sys.argv[1]; s = open(p).read()
s = s.replace('el("add").disabled = !view.can_add;', 'el("add").disabled = view.count >= 10;')
open(p, "w").write(s)
EOF
expect_failure "a copy of a core rule in the page" "number in the page" boundary

fresh
printf '<img src="https://example.invalid/pixel.gif" alt="">\n' >> "$WORK/p/web/index.html"
expect_failure "an absolute URL in the page" "absolute URL" boundary

fresh
"$PY" - "$WORK/p/web/index.html" <<'EOF'
import re, sys; p = sys.argv[1]; s = open(p).read()
s = re.sub(r'  <meta http-equiv="Content-Security-Policy"[^>]*>\n', "", s)
open(p, "w").write(s)
EOF
expect_failure "a page with no Content-Security-Policy" "no Content-Security-Policy" boundary

fresh
cat >> "$WORK/p/crates/shim/src/lib.rs" <<'EOF'

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = navigator, js_name = sendBeacon)]
    fn beacon(url: &str) -> bool;
}

#[wasm_bindgen]
pub fn ping() -> bool {
    beacon("/collect")
}
EOF
expect_failure "a WebAssembly import that reaches sendBeacon" "reaches sendBeacon" \
  bash -c './scripts/build.sh >/dev/null && "$0" scripts/wasm_imports.py dist' "$PY"

# The next two get past the source scans on purpose (the URL is assembled at
# run time and the error is raised after the build), so only the browser
# check stands between them and a deploy.
fresh
(cd "$WORK/p" && ./scripts/build.sh >/dev/null 2>&1)
printf '\nfetch(["https:", "", "example.invalid", "x"].join("/")).catch(() => {});\n' >> "$WORK/p/dist/app.js"
expect_failure "an off-origin fetch the page's CSP blocks" "FAIL no Content-Security-Policy violation" browser_planted

fresh
(cd "$WORK/p" && ./scripts/build.sh >/dev/null 2>&1)
printf '\nfetch(["https:", "", "example.invalid", "x"].join("/")).catch(() => {});\n' >> "$WORK/p/dist/app.js"
"$PY" - "$WORK/p/dist/index.html" <<'EOF'
import re, sys; p = sys.argv[1]; s = open(p).read()
s = re.sub(r'  <meta http-equiv="Content-Security-Policy"[^>]*>\n', "", s)
open(p, "w").write(s)
EOF
expect_failure "the same fetch with the CSP stripped from the built page" "FAIL no off-origin request" browser_planted

fresh
(cd "$WORK/p" && ./scripts/build.sh >/dev/null 2>&1)
printf '\nconsole.error("planted");\n' >> "$WORK/p/dist/app.js"
expect_failure "a console error" "FAIL no console error" browser_planted

fresh
"$PY" - "$WORK/p/crates/core/src/counter.rs" <<'EOF'
import sys; p = sys.argv[1]; s = open(p).read()
s = s.replace("(state.count + s.step).min(s.max)", "state.count + s.step")
open(p, "w").write(s)
EOF
expect_failure "a broken rule (no stop at the maximum), core suite" "adding_stops_at_the_maximum" \
  cargo test -q -p tool_core --lib
expect_failure "the same broken rule, browser check" "FAIL Add steps by 3 and stops at 10" browser


# ---- Added with the second round of reference defaults ----

fresh
mkdir -p "$WORK/p/data/shaders"
printf '#version 300 es\nprecision highp float;\nout vec4 o;\nvoid main() { o = vec4(0.82, 0.45, 1.0, 1.0); }\n' > "$WORK/p/data/shaders/scene.frag"
expect_failure "a tuning number in a shader" "number in a shader" boundary

fresh
mkdir -p "$WORK/p/web/lib"
printf 'export const stopsAt = 10;\n' > "$WORK/p/web/lib/rule.js"
expect_failure "a copy of a rule in a nested page file" "number in the page" boundary

fresh
printf '\nfunction leave(next) { window.location.href = next; }\n' >> "$WORK/p/web/app.js"
expect_failure "a page that navigates on its own" "the page navigates" boundary

fresh
printf '\nif (!navigator.webdriver) document.body.requestPointerLock();\n' >> "$WORK/p/web/app.js"
expect_failure "a page that behaves differently under automation" "behave the same under the browser check" boundary

fresh
printf '\nlocalStorage.setItem("count", "0");\n' >> "$WORK/p/web/app.js"
expect_failure "browser storage in the page" "browser storage in the page" \
  cargo test -q -p tool_core --test no_storage

copy_test() { cargo test -q -p tool_core --test copy; }

fresh
"$PY" - "$WORK/p/data/counter.json" <<'EOF2'
import json, sys; p = sys.argv[1]; d = json.load(open(p))
d["add_label"] = "Add to the tally"
json.dump(d, open(p, "w"), indent=2)
EOF2
expect_failure "a word the glossary rules out" "is ruled out for" copy_test

fresh
"$PY" - "$WORK/p/data/counter.json" <<'EOF2'
import json, sys; p = sys.argv[1]; d = json.load(open(p))
d["intro"] = "Let us delve into counting."
json.dump(d, open(p, "w"), indent=2)
EOF2
expect_failure "a stock phrase in a string" "(TONE.md)" copy_test

fresh
"$PY" - "$WORK/p/GLOSSARY.md" <<'EOF2'
import sys; p = sys.argv[1]; s = open(p).read()
s = s.replace("How much one press of Add or Remove changes the count.", "Each press of Add or Remove changes the count by 3.")
open(p, "w").write(s)
EOF2
expect_failure "a rule's number typed into the glossary" "{placeholder}" copy_test

fresh
mkdir -p "$WORK/p/web/vendor"
printf 'export const lib = 0;\n' > "$WORK/p/web/vendor/lib.js"
(cd "$WORK/p/web/vendor" && printf '%s  lib.js\n' "$(printf 'something else' | shasum -a 256 | cut -d' ' -f1)" > SHA256SUMS)
expect_failure "a vendored file that does not match its checksum" "does not match its SHA256SUMS" ./scripts/build.sh

fresh
printf '[{"name": "a break no test sees", "file": "web/style.css", "find": "box-sizing", "replace": "box-sizing", "test": "adding_stops_at_the_maximum"}]\n' > "$WORK/p/tests/breaks.json"
expect_failure "a break its test does not catch" "still passes with the behaviour broken" "$PY" scripts/break_tests.py

fresh
printf '[]\n' > "$WORK/p/tests/breaks.json"
expect_failure "an empty list of breaks" "the proof would prove nothing" "$PY" scripts/break_tests.py

notebook() { "$PY" scripts/notebook_check.py; }

fresh
"$PY" - "$WORK/p/SECOND-ORDER.md" <<'EOF2'
import sys; p = sys.argv[1]; s = open(p).read()
old = "| R1.1 | | | | | | |"
assert old in s
s = s.replace(old, "| R1.1 | finding | The count starts at 0. | It does. | Nothing. | | finished |")
open(p, "w").write(s)
EOF2
expect_failure "a notebook row with a status the procedure does not use" "has status 'finished'" notebook

fresh
"$PY" - "$WORK/p/SECOND-ORDER.md" <<'EOF2'
import sys; p = sys.argv[1]; s = open(p).read()
old = "| R1.1 | | | | | | |"
assert old in s
s = s.replace(old, "| R1.1 | worklist | The labels were written by the agent. | No teacher has read them. | Listed for the person. | | person |")
open(p, "w").write(s)
EOF2
expect_failure "a person's notebook row that the handoff does not name" "R1.1 is the person's to do" notebook

fresh
"$PY" - "$WORK/p/SECOND-ORDER.md" "$WORK/p/HANDOFF.md" <<'EOF2'
import sys; p, h = sys.argv[1:]; s = open(p).read()
old = "| R1.1 | | | | | | |"
assert old in s
s = s.replace(old, "| 13 | worklist | The labels were written by the agent. | No teacher has read them. | Listed for the person. | | person |")
open(p, "w").write(s)
open(h, "a").write("\nPress 13 times to reach the maximum.\n")
EOF2
expect_failure "a notebook row numbered outside the rounds" "13 is not a round number" notebook

fresh
"$PY" - "$WORK/p/PLAN.md" <<'EOF2'
import sys; p = sys.argv[1]; s = open(p).read()
old = "| 1 | | | |"
assert old in s
s = s.replace(old, "| 1 | The count stops at the maximum. | The brief says so. | adding_stops_at_the_maximum |")
open(p, "w").write(s)
EOF2
expect_failure "a plan with decisions and an empty notebook" "SECOND-ORDER.md has no rows, but PLAN.md holds decisions" notebook

journal() { env -u CI "$PY" scripts/journal_check.py; }

fresh
rm -rf "$WORK/p-journal"
expect_failure "a started build with no journal" "the journal is missing" journal

fresh
"$PY" - "$WORK/p-journal/index.html" <<'EOF2'
import sys; p = sys.argv[1]; s = open(p).read()
old = 'data-kind="reading"'
assert old in s
s = s.replace(old, 'data-kind="plan"')
open(p, "w").write(s)
EOF2
expect_failure "a journal with no opening entry" "the journal has no opening entry" journal

fresh
"$PY" - "$WORK/p-journal/index.html" <<'EOF2'
import sys; p = sys.argv[1]; s = open(p).read()
old = 'data-kind="reading">'
assert old in s
s = s.replace(old, 'data-kind="reading" data-blank>')
open(p, "w").write(s)
EOF2
expect_failure "an opening entry left blank" "still the template's blank one" journal

fresh
"$PY" - "$WORK/p/SECOND-ORDER.md" "$WORK/p/HANDOFF.md" <<'EOF2'
import sys; p, h = sys.argv[1:]; s = open(p).read()
old = "| R1.1 | | | | | | |"
assert old in s
s = s.replace(old, "| R1.1 | worklist | The labels were written by the agent. | No teacher has read them. | Listed for the person. | | person |")
open(p, "w").write(s)
open(h, "a").write("\nOpen for the person: R1.1.\n")
EOF2
expect_failure "a person's notebook row the journal does not name" "R1.1 is the person's to do and the journal does not name it" journal

fresh
"$PY" - "$WORK/p-journal/index.html" <<'EOF2'
import sys; p = sys.argv[1]; s = open(p).read()
old = "<h4>Where I stop</h4>"
assert old in s
s = s.replace(old, "<p>Recorded as R4.2 and answered in T9.</p>" + old)
open(p, "w").write(s)
EOF2
expect_failure "a journal that cites a notebook row that does not exist" "the journal cites R4.2" journal
expect_failure "a journal that cites a triage row that does not exist" "the journal cites T9" journal

fresh
(cd "$WORK/p-journal" && git init -q && git add -A && git -c user.name=prove -c user.email=prove@invalid commit -qm journal)
"$PY" - "$WORK/p-journal/index.html" <<'EOF2'
import re, sys; p = sys.argv[1]; s = open(p).read()
s2 = re.sub(r'\n    <article class="entry" id="brief".*?</article>', "", s, flags=re.S)
assert s2 != s
open(p, "w").write(s2)
EOF2
expect_failure "a journal entry removed since the journal's last commit" "entry brief was in the journal's last commit and is gone" journal

# The style pack and the art pipeline live in template/, not the example.
mkdir -p "$WORK/leak/template/styles/default" "$WORK/leak/template/web"
cp "$ROOT/template/styles/default/palette.json" "$WORK/leak/template/styles/default/"
pack_hex="$("$PY" -c 'import json,sys; d=json.load(open(sys.argv[1])); print(next(v for k,v in d.items() if not k.startswith("_")))' "$ROOT/template/styles/default/palette.json")"
printf 'body { color: %s; }\n' "$pack_hex" > "$WORK/leak/template/web/style.css"
rm -rf "$WORK/p"; mkdir -p "$WORK/p"
expect_failure "a default-style colour in the template's CSS" "default-style token" \
  "$PY" "$ROOT/scripts/check-style-leak.py" "$WORK/leak"

rm -rf "$WORK/art"; mkdir -p "$WORK/art"
(cd "$ROOT/template" && tar --exclude=__pycache__ -cf - tools tests/art_check.py data/art.json styles) | (cd "$WORK/art" && tar -xf -)
printf '\nSTRAY = "#123456"\n' >> "$WORK/art/tools/art/artlib.py"
expect_failure "a colour literal in the art code" "colour literal" "$PY" "$WORK/art/tests/art_check.py"

echo
echo "caught $pass, missed $fail"
if [ $((pass + fail)) -eq 0 ]; then
  echo "no planted defects ran: the proof would prove nothing" >&2
  exit 1
fi
[ "$fail" -eq 0 ]
