# Releases

## Versions

Version 0.1.0 is the first public version of the template, the minimal example and the checks. A version is a git tag on the commit whose `./scripts/check.sh` passed in CI.

Version 0.2.0 (13 October 2026) folds in what thirteen of the author's projects and Peter's two games found:

- the description-first procedure for people who do not write code (`prompts/start-from-description.md`), and the root `CLAUDE.md` routing a described tool into a new project;
- a view model and one action function in the template's core, and a generic page renderer (from Peter's Mandible);
- `GLOSSARY.md`, `TONE.md` and the copy tests; `no_storage.rs`;
- boundary scans for nested page files, navigation, automation checks and numbers in shaders;
- cache-busting, checksum verification and `build.txt` in the build; a test profile, `rustfmt.toml` and a pinned toolchain;
- `scripts/break_tests.py`, `count-tests.sh` and `publish.sh`; a phone engine in the browser check;
- the stage files reshaped (numbered triage, a seven-part handoff, `DECISIONS.md`, `LICENSES.md`);
- the art pipeline and the default style pack, kept in `template/styles/` by a leak check;
- optional kits for online play, canvas rendering, real-time input, mobile and strict privacy;
- 27 planted defects, up from 14.

No tag is made for it; tagging is the author's action.

Since 0.2.0, not yet in a version:

- the reason for the second-order notebook, and how a row becomes a rule, in `docs/procedure.md`;
- a **Rule** column in `SECOND-ORDER.md` and an "Open for the person" section in `HANDOFF.md`;
- `scripts/notebook_check.py`, step 7 of a project's check: round numbers, kinds, statuses, person rows in the handoff, and no empty notebook once the plan holds decisions, with four planted defects (31 in all);
- the root `CLAUDE.md` and the description procedure tell an agent started in the reference repository to read the new project's instructions, and the person to start later sessions in the project;
- the design journal (D19): `tools/journal/` (the page, `ENTRIES.md`, `new_journal.py`), a `design-journal` skill, a Journal section in `BRIEF.md`, rules 13 to 15 and "Rules agreed for this project" in `CLAUDE.md` and `AGENTS.md`, a stage 0 opening entry in the procedure and both start prompts, `docs/journal.md`, an example journal for `examples/minimal`, and `scripts/journal_check.py`, step 8 of a project's check, with seven planted defects (38 in all).

## The live example

`scripts/publish-example.sh` builds `examples/minimal` and force-pushes it to the `gh-pages` branch, which GitHub Pages serves. Run it after `./scripts/check.sh` passes, then run the example's browser check against the live origin with `BUILDER_ORIGIN` set. Publishing is the person's action. A project publishes with its own `scripts/publish.sh`, which refuses uncommitted changes, runs the check, and pushes `dist/` with the message "Built from <commit>". The build stamps a content hash on every internal URL, so a returning reader gets the new module, not one GitHub Pages cached for ten minutes.

## A frozen snapshot for the paper

The paper should refer to one immutable snapshot. When the camera-ready version is due:

1. Run `./scripts/check.sh` and confirm CI passed on the commit.
2. Tag the commit with an annotated tag whose name says it is the paper's snapshot, and create a GitHub release from that tag.
3. Archive the release (for example through Zenodo's GitHub integration) so that it has a DOI.
4. Put the DOI and the tag in `CITATION.cff`, `CITATION.bib` and the paper.

Choosing the tag's name, creating the release and archiving it are the author's actions.

## An anonymous artifact for review

A venue with double-anonymous review may need an artifact that does not identify the authors. This repository is public under its author's account, so it is not anonymous, and the review version of the paper should not link to it. Instead:

1. Make an anonymised copy of a fixed commit with a service built for the purpose (such as anonymous.4open.science), or export the tree without its git history.
2. Run `scripts/anonymity-check.sh <directory>` on the export. It lists every line that names the author, the account, the institution or a personal site, and exits non-zero if it finds one.
3. Remove or neutralise each line it lists: `LICENSE`, `CITATION.cff`, `CITATION.bib`, the clone URL in `README.md`, the Feedback Drafts link, and anything else it reports.
4. Check that the anonymised copy still passes `./scripts/check.sh`.
