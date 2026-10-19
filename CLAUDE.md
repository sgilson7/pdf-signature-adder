# Instructions for a coding agent in a Builder project

`AGENTS.md` holds the same rules for agents that read that file instead.

## Where code goes

1. Read `BRIEF.md`, `BUILDER-SPEC.md` and `GLOSSARY.md` before changing code.
2. The core (`crates/core`) owns every application decision and performs no input or output. Scripts and tools that need a rule read the core's output; they do not recompute it.
3. The shim (`crates/shim`) translates values and decides no application behaviour.
4. The page (`web/`) draws what the core returns and does not reproduce a core rule. The core describes the screen (`ui`) and takes settings, screens and flags through one function (`act`). Presentation numbers go in `data/*.json`; shaders go in `data/shaders/` and take rule values as uniforms.

## How to work

5. Run `cargo test` after each meaningful core change.
6. For each new behaviour test, add a row to `tests/breaks.json` and run `python3 scripts/break_tests.py`: the test must fail with its behaviour broken. Work expected values out by hand; never compare a value with the function under test. When a test needs a workaround to pass, the workaround is the finding: record it.
7. Keep `SECOND-ORDER.md` while you work: one row per assumption, surprise, divergence from the plan, or item the person must do, written when you notice it. It is how the person sees what you decided alone. A row that produced a rule names where the rule now lives; a row with status *person* is named in `HANDOFF.md`.
8. A scripted edit that replaces text asserts that it matched. A count in a document comes from `./scripts/count-tests.sh`.
9. Do not deploy unless `BRIEF.md`'s Autonomy paragraph says you may. Deploying is otherwise the person's decision (stage 8).
10. Before handoff, run `./scripts/check.sh` and record the result in `HANDOFF.md`.

## Words

11. Use each term in the sense `GLOSSARY.md` gives it, and never a word from its `Not:` line. A new term gets an entry before it appears in a string.
12. Reader-visible text follows `TONE.md`. A string that looks wrong is reported in `SECOND-ORDER.md`, not repaired. A string you write yourself is listed at the foot of `SECOND-ORDER.md` for the person to read. A list you generate that a reader will see (names, labels, word lists) is a worklist row for the person.

## The design journal

13. Keep the design journal unless `BRIEF.md`'s Journal section says `none`. Start it with `python3 tools/journal/new_journal.py`; it lives beside the project (by default `../<this folder>-journal/`) as its own repository and is never pushed. `tools/journal/ENTRIES.md` says what each kind of entry holds.
14. Before writing `PLAN.md`, write the journal's opening entry: read Builder and the brief back to the person, with numbered rules and what holds each, the question prefix, where you stop, and your questions. When the person answers, each rule that applies to every later session goes under "Rules agreed for this project" below, and each rule the core can enforce becomes a test.
15. Every decision put to the person, visible change, divergence from the plan and deploy gets a journal entry when it happens, citing its rows (T4, R2.3, XX-Q5). Never remove an entry. `scripts/journal_check.py` checks the form; commit in the journal's repository after each change.

## Rules agreed for this project

Agreed 2026-10-19 (journal `#opening`, `#agreed`). Each names what holds it.

- **PS-1** The saved PDF begins with the original's bytes, unchanged; the signature is an appended incremental update. Held by `crates/core/tests/sign.rs`.
- **PS-2** Every position and size is decided in the core, in thousandths of a PDF point, through rotation and crop box. The page positions overlays from strings the core returns. Held by `place.rs` tests and the boundary scan.
- **PS-3** Nothing leaves the device and nothing is kept: no request of any kind after the page is ready, no cookies or storage. Held by the privacy additions to `tests/browser/check.py`, `no_storage.rs` and `scripts/prove-privacy.sh`.
- **PS-4** "Signature" means the mark a person writes; no string says or implies digital signing, certification or verification. Held by `GLOSSARY.md`'s `Not:` lines and `copy.rs`.
- **PS-5** A PDF the tool cannot amend safely is refused with a message naming the reason. Held by `pdf` module tests.
- **PS-6** The core reads its own output back before offering it. Held by `write.rs` and `sign.rs` tests.
- **PS-7** pdf.js only paints, is vendored and pinned by checksum, and loads before ready. Held by `scripts/build.sh` and the browser check.
- **PS-8** Every control works by touch, Pencil, mouse and keyboard; touch targets are at least the size in `data/layout.json`. Held by the iPad pass of the browser check.
- **R1.11** Do not add `'unsafe-inline'` to `style-src` to quiet console errors seen while taking Playwright screenshots in WebKit: the screenshot tool injects that stylesheet, not the page.
- **PS-9** The saved file opens in other readers with the signature where it was placed. Held by the person's use (stage 7); `scripts/smoke.py` renders outputs with Poppler as a second reader.

## Stopping

Write the journal's opening entry, then `PLAN.md` from the brief, and stop until the person has read it, unless `BRIEF.md`'s Autonomy paragraph approves the plan in advance. When a rule in `BUILDER-SPEC.md` seems to block the work, stop and say so in a worklist row; do not loosen a check to make it pass. When a request is refused on safety grounds, leave the feature out and say so; do not reword the request to get past the refusal.
