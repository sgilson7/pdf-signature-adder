# Instructions for a coding agent in the Builder reference repository

**If the person describes a tool they want built, do not edit this repository.** Follow [`prompts/start-from-description.md`](prompts/start-from-description.md): copy `template/` to a new sibling directory, write the brief there from their words, and build there. Before writing the brief, read that project's `CLAUDE.md` (or `AGENTS.md`) in full. Its rules govern the build, including the second-order notebook, and an agent started in this repository is not given them automatically, because the project is outside this directory.

The rules below are for work on the reference repository itself.

`CLAUDE.md` holds the same rules. A project copied from `template/` has its own `AGENTS.md`.

1. `template/` is what people copy. `examples/minimal/` is a copy of it with one small rule. Files listed in `scripts/check-sync.sh` must stay byte-identical between the two, and `BUILDER-SPEC.md` at the root must match `template/BUILDER-SPEC.md`.
2. Keep the three categories apart in every document: Builder requirements, reference defaults, project decisions. Do not promote a convention from one tool into a requirement.
3. Run `./scripts/check.sh` before handing off. It checks the template, the example, the sync, the links, and that each planted defect is caught.
4. A new check is not finished until `scripts/prove-checks.sh` plants a defect it must catch and sees it fail.
5. Do not describe a check as proving more than it does. The browser check describes one path in three engines; the mutation score describes how much current behaviour the tests pin down.
6. Do not name the paper's title or venue in this repository while it is under review.
7. Do not push, tag or publish a release on your own judgement.
8. The default style pack's colours and name stay inside `template/styles/`. `scripts/check-style-leak.py` enforces it.
9. A kit under `kits/` is optional. It says which reference default it changes, and a project that copies it records that change.
