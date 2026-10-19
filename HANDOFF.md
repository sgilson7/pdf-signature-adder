# Handoff

Stage 9, written by the agent. `C5 Consolidate in your own words`. A fresh session starts from this file, not from memory.

## What this is

A static page that signs a PDF the reader already has: open it, draw a signature (Pencil, finger, mouse) or paste or open a picture of one, place it, and save a signed copy. The signed copy is the original bytes plus an appended incremental update. Nothing leaves the browser tab. Live at <https://sgilson7.github.io/pdf-signature-adder/>, source at <https://github.com/sgilson7/pdf-signature-adder>, listed at <https://sgilson7.github.io/projects/pdf-signature-adder/>.

## Rules that hold it up

| Rule | Breaks silently if | Guarded by |
| --- | --- | --- |
| PS-1 original bytes untouched | the writer ever rebuilds the file | `crates/core/tests/sign.rs`; the browser check compares the downloaded copy's prefix |
| PS-2 geometry in the core | the page computes a position | boundary scan; `place.rs` tests per rotation; `scripts/smoke.py` (Poppler) |
| PS-3 no request after ready, nothing stored | pdf.js is set up to fetch fonts, CMaps or its worker lazily | privacy additions in `tests/browser/check.py`; `scripts/prove-privacy.sh` |
| PS-5 refusals | a new reader path skips the checks in `Document::open` | `pdf/mod.rs` tests; `tests/breaks.json` |
| PS-6 read-back | `write::read_back` is bypassed | `sign.rs` |
| pdf.js on the main thread | `globalThis.pdfjsWorker` is set after the first `getDocument` | the browser check fails on any worker |

## Shape of the code

| Where | What |
| --- | --- |
| `crates/core/src/pdf/` | `object.rs` syntax, `filter.rs` Flate and predictors, `doc.rs` reader and refusals, `write.rs` the update and read-back, `fixture.rs` PDFs built for tests |
| `crates/core/src/place.rs` | display space, the `cm` matrix per rotation, clamping, resizing |
| `crates/core/src/ink.rs`, `picture.rs` | drawn strokes; pasted pixels |
| `crates/core/src/api.rs` | the `Handle`: `ui()`, `act()`, and the byte-carrying calls |
| `crates/shim/src/lib.rs` | `Tool`, forwarding only |
| `web/app.js` | regions redrawn on change; pdf.js painting; pointer, file, clipboard, download, share |
| `data/` | `strings.json` (every reader-visible string), `layout.json` |

## Commands

| Command | Passes when it prints |
| --- | --- |
| `./scripts/check.sh` (set `BUILDER_PYTHON` to a Python with Playwright 1.63) | `BUILDER CHECK PASSED` |
| `./scripts/count-tests.sh` | `116 passed, 0 failed, 1 ignored` |
| `python3 scripts/break_tests.py` | `13 of 13 breaks caught` |
| `./scripts/prove-privacy.sh` | `PRIVACY PROOF PASSED` |
| `python3 scripts/smoke.py IN_DIR OUT_DIR` (outside the repo) | `SMOKE PASSED` |
| `./scripts/publish.sh` then `BUILDER_ORIGIN=https://sgilson7.github.io/pdf-signature-adder python3 tests/browser/check.py` | `published dist/ built from …`; `BROWSER CHECK PASSED` |

## What will bite within the hour

- No `.venv` here: this session used `~/Documents/Mp4ToTranscript/.venv` through `BUILDER_PYTHON`. Run `./scripts/setup.sh` for a local one.
- Building a copy of the project with `CARGO_TARGET_DIR` pointed at this repository's `target/` replaces the boundary-test binary with one that reads the copy's (deleted) folder; `cargo clean -p tool_core` fixes it (R1.12).
- WebKit screenshots log CSP errors from Playwright's own injected stylesheet. Do not loosen `style-src` (R1.11).
- The website's working tree holds the person's uncommitted Packet Alley edit, already live; `make deploy` builds from the working tree, so it stays live. Do not commit or revert it.

## The journal

`../PDF-Signature-Adder-journal/index.html`, its own repository, never pushed; published privately at <https://claude.ai/artifact/Xc7eZsY9VkYwQBvwGcM8ta>. Newest entry: *your fixes*, T1 confirmed, 2026-10-19.

## Open for the person

No `SECOND-ORDER.md` row has status *person* or *open*. T1 (Pencil scrolled the page) was confirmed fixed on the person's iPad (R2.1, done). What only the person can check (stage 7): open a real form on the iPad, sign it with the Pencil, save it, and open the copy in Files, Preview and Acrobat; paste a signature from Photos; read the strings in `data/strings.json`.

## Mistakes so far

- Tests: two expected values worked wrong by hand (R1.6); one test that could not catch the rule it named (R1.10); a privacy proof whose copy never built (R1.12). Each produced a test or a script change.
- Design: corner handles too large for a small signature (R1.7), now an inner-half rule with its own test.
- Measurement: a pixel diff blind to dark ink on a dark slide (R1.8); the smoke run now draws in red.

## What passed

2026-10-19, commit `8f4a2e3`: `./scripts/check.sh` printed `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit, webkit-ipad); `count-tests.sh` printed `116 passed, 0 failed, 1 ignored`; the browser check against the live address passed in the same four engines. Website: `make check`, `make packet` and `make gate` passed before `make deploy` (build 5ac9bfcd).

## The next action

The person signs a real PDF on their iPad and reports anything that differs as a TRIAGE row.
