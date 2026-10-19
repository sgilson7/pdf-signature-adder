# Mapping to the paper

A paper describing Builder and the pilot behind it is under review. Its title and venue are withheld here until it can be cited; this page maps its terms and checks to the files and commands in this repository so that a reader of the paper can find each one.

## Terms

| Paper's term | Here |
| --- | --- |
| Builder, a shared technical setting | this repository; the contract is `BUILDER-SPEC.md` |
| core: a Rust crate that holds the tool's logic and performs no input or output | `template/crates/core`; requirement R1–R4 |
| shim: a crate that uses `wasm-bindgen` to generate the bindings | `template/crates/shim`; requirement R5 |
| page: plain JavaScript that draws what the core computed, served as static files | `template/web`; requirement R6 |
| instruction file the agent reads at the start of every session | `template/CLAUDE.md`, `template/AGENTS.md` |
| nine-stage procedure and its labels | `docs/procedure.md`; the stage files in `template/` |
| second-order notebook: the agent's record about the build, kept as the work happens | `template/SECOND-ORDER.md`; [`docs/procedure.md`](procedure.md#the-second-order-notebook); checked for form by `scripts/notebook_check.py` |
| default load-and-idle path | `tests/browser/check.py`: load, wait for `data-ready`, idle 10 s |
| prompt set | `prompts/prompt-set-template.md`; `prompts/sets/` |

## The repository rules (a)–(f)

| Paper | Builder category | Here | Enforced by |
| --- | --- | --- | --- |
| (a) the core uses only listed crates; graphics, browser access and networking stay outside it | requirement (R2, R3) | `ALLOWED_DEPENDENCIES`, `IO_IDENTIFIERS` | `core_dependencies_are_on_the_allowlist`, `core_source_has_no_io_and_no_nondeterminism` |
| (b) the shim translates values while application decisions remain in the core | requirement (R5) | shim source | `shim_decides_nothing`. The paper reports no automated check for (b) across its seven tools; both fresh prompt-set sessions added one. The template ships it. |
| (c) the page renders core output without duplicating rules | requirement (R6) | page source | `page_keeps_no_rule_and_names_no_other_origin` (a source scan; see `docs/checks.md` for what it misses) |
| (d) project code runs from the static host; a browser gate fails on console errors or requests leaving the origin | requirement (R7) | `tests/browser/check.py` | the browser check, three engines |
| (e) integers and ordered structures so the same inputs reproduce the same world | reference default (D2) | `NONDETERMINISM_IDENTIFIERS` | `core_source_has_no_io_and_no_nondeterminism` |
| (f) each new test is broken once and observed failing | procedure (stage 5) | `CLAUDE.md` rule 6 | by hand in a project; `scripts/prove-checks.sh` for the setting's own checks |
| Content-Security-Policy in a meta element, which must allow WebAssembly and cannot stop navigation | reference default (D4) | `web/index.html` | `every_page_sets_the_content_security_policy`; CSP violations fail the browser check |

## The five checks on the finished artifact

| Paper's check | What it exposes | Here |
| --- | --- | --- |
| imports + glue | compiled reach | `scripts/wasm_imports.py dist` |
| browser audit | network activity on load and idle | `tests/browser/check.py` |
| core suite | tested core behaviour | `cargo test --workspace` |
| planted changes (mutation testing) | test-suite strength | `scripts/mutants.sh` (cargo-mutants), outside `check.sh` |
| rule tests | whether a written rule holds | `crates/core/tests/boundary.rs` |

The paper's measurements were made on the pilot's own repositories with their own scripts, which come with the paper's replication package. The scripts here are new implementations of the same checks for the template, and `scripts/prove-checks.sh` shows each one failing on a planted case.

## Prompt sets

The paper describes prompt sets that turn teacher requests into briefs for Builder, given once to a fresh agent session. `prompts/sets/C-feedback-drafts.md` is a prompt set written after the paper's two, from a third teacher request from the same prior work, and run from `template/` in this repository. Its session record is in the Feedback Drafts repository: `TIMING.md`, `PLAN.md`, `SECOND-ORDER.md`, `HANDOFF.md` and the commit history.
