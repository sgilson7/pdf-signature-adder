# Start from a brief

Use after `BRIEF.md` is written, in a fresh session in the project directory. Stages 1–2. `C1`, `B1`, `B2`.

```text
Read CLAUDE.md, BUILDER-SPEC.md and BRIEF.md, in that order.

Unless BRIEF.md's Journal section says none, start the design journal
(python3 tools/journal/new_journal.py) and write its opening entry, as
tools/journal/ENTRIES.md describes, before the plan.

Write PLAN.md from the brief: each decision in build order, with the
reason for it and the test that will show it holds. Put every rule in
crates/core. Name any rule you think belongs in the shim or the page,
and say why, as an open question rather than a decision.

List the questions the brief leaves open, numbered, at the end of
PLAN.md. Then stop. Do not write code until I have read the plan.
```

If the brief approves the plan in advance, replace the last line with:

```text
The plan is approved in advance. Write PLAN.md and MEASUREMENTS.md,
then build in the plan's order. Commit after each working step. Keep
SECOND-ORDER.md as you go. Do not push and do not deploy. Finish with
./scripts/check.sh and HANDOFF.md.
```
