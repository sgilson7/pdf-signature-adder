# What each check establishes

`./scripts/check.sh` runs nine steps. Each one establishes something specific, and each leaves something for another check or a person. The right-hand column is the part to remember.

| Check | Command | Establishes | Does not establish |
| --- | --- | --- | --- |
| Format and lint | `cargo fmt --check`; `cargo clippy -D warnings` | the code is formatted and clippy finds nothing | anything about behaviour |
| Core suite | `cargo test --workspace` | the rules hold on the cases the tests state | that the stated cases are the intended behaviour; that is the brief's and the person's |
| Boundary tests | `crates/core/tests/boundary.rs`, inside `cargo test` | no I/O name and no float, hash map or clock in core source; no dependency outside the allowlist; no branch, loop or `?` in the shim; no numeric literal, arithmetic, network API, navigation or `navigator.webdriver` in page JavaScript at any depth under `web/`; no number other than 0 and 1 in `data/shaders/`; no absolute URL in `web/` except the SVG namespace; a CSP on every page | a rule hidden in the page by other means (a string comparison, a lookup table); vendored files (pinned by checksum instead); the scans read source text, not meaning |
| Copy and storage tests | `copy.rs`, `no_storage.rs`, `glossary.rs` | glossary entries are complete and type no rule's number; no string uses a word a glossary entry rules out; no string, definition or README line uses a phrase `TONE.md` lists; the page names no browser storage API | that the words are right for the field, or that the prose is good; a list of three for rhythm passes |
| Release build | `scripts/build.sh` | the module builds with the pinned `wasm-bindgen`; every vendored file matches its `SHA256SUMS`; every internal URL carries the build's content hash | that the vendored code is safe; only that it is the code that was pinned |
| WebAssembly import audit | `scripts/wasm_imports.py dist` | the module's imports, each followed into the generated glue and any snippet, name no network API; the glue holds one `fetch`, which loads the module; when `wasm-tools` is installed, its import list matches the script's | the page's own JavaScript; vendored modules the page loads separately |
| Art checks | `tests/art_check.py` | the pipeline's checks catch each kind of bad picture; the art code holds no colour literal | how the pictures look; rendering itself needs TeX and is a person-run step |
| Notebook form | `scripts/notebook_check.py` | every row of `SECOND-ORDER.md` has a round number (R1.1) used once, a kind and a status; every row with status *person* is named in `HANDOFF.md`; once `PLAN.md` holds a decision, the notebook has a row. It prints, without failing, how many commits that changed code also changed the notebook | that the notebook records everything that happened, or that rows were written at the moment; the commit count is evidence for the person, not a pass mark |
| Design journal | `scripts/journal_check.py` | once the brief holds a description, the journal it names exists (on CI a missing journal is reported, not failed); it has a filled opening entry; every entry has an id and a kind; every *person* notebook row is named in it; every notebook and triage row it cites exists; no entry in its last commit is gone | that an entry is right, that a picture shows what its caption says, or that the agent left nothing out ([journal.md](journal.md)) |
| Browser check | `tests/browser/check.py` | in Chromium, Firefox and WebKit, on load, a 10 s idle and the project's interactions: no uncaught error, console error, CSP violation, off-origin request, WebSocket, popup, worker or service worker; the interactions' hand-computed values were drawn | features the interactions never start; behaviour on another day, browser version or network |

## Show that a check can fail

A check that has only been seen passing has not been shown to check anything. `scripts/prove-checks.sh` copies `examples/minimal`, plants one defect at a time, runs the check meant to catch it, and requires that check to fail with the message that names the defect. The planted defects include a float, a `println!` and a hash map in the core, a dependency outside the allowlist, a branch in the shim, a copy of a rule in the page, an absolute URL, a missing CSP, an import that reaches `sendBeacon`, an off-origin fetch assembled at run time (once with the CSP in place and once without it), a console error, a broken rule seen by both the core suite and the browser check, a notebook row the handoff does not name, and a started build with no journal, a blank opening entry, a journal citing a row that does not exist, and a journal entry removed since its last commit.

One finding from writing that script: with the CSP in place, the browser blocks a forbidden fetch before it becomes a request, so a check that only listens for requests sees nothing. The browser check therefore records CSP violations as their own failure.

For each new behaviour test in a project, do the same by hand: break the behaviour once, watch the test fail, restore it, and note the message in `SECOND-ORDER.md`. A test that needed a workaround to pass is a *finding* row, and the workaround is the finding.

## Break each test once

`scripts/break_tests.py` runs the rows of `tests/breaks.json`: each one replaces a piece of a rule, runs the test meant to cover it, requires a failure, and restores the file. It fails when a break leaves its test passing, when a break's text is no longer in its file, and when the list is empty. Long Time Comin' ran 132 breaks and found tests that passed while broken "because a second line kept the behaviour"; Status Trends found its own proof script printing PASSED over an empty list, which is why an empty list now fails. The reference repository runs the template's and the example's breaks in its check; a project runs its own after adding tests and before a handoff.

## Probes are not checks

A probe prints numbers: per-tick cost, a balance distribution over 300 bot runs, frames per second in headless Chromium. It asserts loose bounds at most, carries its run command in its doc comment, and its output goes to `MEASUREMENTS.md`. Keep probes out of `check.sh`; a probe that gates a build on a number from one machine fails on the next.

## Things the browser check cannot see

- **CI draws with a CPU.** Headless Chromium on CI renders WebGL with SwiftShader. Long Time Comin' failed on CI because a scripted walk took more than 40 seconds at 5 frames a second. Interactions wait on a selector or a `data-*` attribute the page sets, never a fixed time; a 3D page may detect a software renderer and draw less (`kits/realtime/`).
- **Pointer lock under automation.** A page that skips pointer lock when `navigator.webdriver` is set sends the check down a path no reader takes; the boundary scan refuses that branch. Where the lock is refused anyway, input falls back to movement without capture (`kits/realtime/`).
- **Visibility.** The check asserts text. Lowballer and Packet Alley each found a panel whose `hidden` attribute a `display: grid` rule overrode; the template's CSS carries `[hidden] { display: none !important; }`, and an interaction that matters should assert `is_visible`.
- **A phone.** `BUILDER_ENGINES="chromium webkit-iphone"` adds WebKit with an iPhone profile and touch. Emulation cannot show real multitouch, fullscreen or install.
- **The CSP refuses Playwright's string-form `wait_for_function`** in some engines. Wait on a selector.

The page publishes its state for tests as `data-*` attributes on `body` (`data-ready`, `data-screen`, and a project's own), which every one of the eight games surveyed used.

## Mutation testing

Mutation testing deliberately makes small changes to the code and reruns the tests. A planted change is caught when those tests fail.

```bash
cargo install cargo-mutants --locked
./scripts/mutants.sh
```

It is kept out of `check.sh` because it takes minutes. The share caught measures how much current behaviour the tests pin down. It is not a bug rate, and it says nothing about whether that behaviour is the intended one. Missed mutants cluster by file; Level Twenty and Status Trends used the clusters to choose the next tests, and Status Trends went from 634 of 859 caught to 754 of 880. Each missed mutant is one of three things: an equivalent change that cannot be detected, a missing test for a rule the brief states, or a boundary the brief has not decided. The first needs nothing; the second needs a test; the third needs the person.

## Deployed pages

After the person deploys, run the same browser check against the live origin:

```bash
BUILDER_ORIGIN=https://example.github.io/my-tool python3 tests/browser/check.py
```

A deploy is not finished until that passes.

## What stays with a person

Whether the tool does what the brief asked, whether its content is right, whether the boundaries the tests pin down are the intended ones, and whether the tool is fit for a class. The checks narrow what a person has to read; they do not replace the reading.
