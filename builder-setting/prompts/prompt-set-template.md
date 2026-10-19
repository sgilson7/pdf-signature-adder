# Prompt set template

A prompt set turns one request into everything a fresh agent session needs to build the tool in Builder: a brief, the plan instruction, the notebook, the corrective dialogue, the checks and the handoff. Each section carries the AI-literacy labels the procedure uses. `sets/C-feedback-drafts.md` is a filled-in example.

Give the set once, unchanged, to a fresh session whose working directory holds only a copy of `template/`. Record the start and finish time, the commits, the core test count and the result of `./scripts/check.sh`.

```markdown
# Prompt set [letter]: [tool in a few words]

## Brief (B1 establish context; C1 frame the problem and set the rules)

You are building a static browser tool for [who]. This directory is a
copy of the Builder template: read CLAUDE.md and BUILDER-SPEC.md first,
and keep to them.

[What the tool does, in the order a person uses it. Name every rule a
test will need: what is allowed, what is rejected, what is computed.]

[What stays on the device, and what is never stored.]

Rules:
(a) The core depends only on [crates].
(b) The shim decides nothing.
(c) The page keeps no copy of a rule: [name the rules most likely to leak].
(d) Nothing runs anywhere but static hosting, and the page makes no
    request to another origin.
(e) The core uses no floats, hash maps, or clocks.
(f) Every new test is broken once and watched failing before it is kept.

What you may not decide alone: [network requests, third-party services,
models, storage, any way for student information to leave the device].
If something seems to need one, stop and say so.

Write [the content] yourself as clearly marked sample content: [size and
shape]. The page must say on screen that it is sample content.

[Either: Write PLAN.md and stop until I have read it. Or: The plan is
approved in advance; continue through the agent's stages without
waiting.] Commit after each working step. Do not push and do not deploy.

## Plan (B2 require step-by-step justification)

Before writing code, write PLAN.md: each decision with the reason for it
and the test that will show it holds, in build order. Fill in
MEASUREMENTS.md. Then build in that order, and put each core rule's test
in the same change as the rule.

## Notebook (B3 build in self-questioning; C3 interrogate the claim)

Keep SECOND-ORDER.md while you work: one row per assumption, surprise,
or open item, with a kind (divergence, finding, or worklist item for the
[person]) and a status (done, open, or person). Before you finish, list
three assumptions about how [the person] will use the tool and check each
against this brief.

## Corrective dialogue (B4 plan the corrective dialogue)

When a test or check fails, write down the file and line, what was
expected, and what happened, and fix the rule in the core, never in the
page.

## Checks (C4 verify against hand-computed cases)

Include core tests whose expected values you worked out by hand: [the
cases]. Include tests that [malformed input] is rejected with a message,
never a crash. Keep the template's boundary tests passing. In
tests/browser/interactions.py, drive the tool the way [the person] would:
[the steps, with the values to compare].

Done means: ./scripts/check.sh passes, including the browser check in
Chromium, Firefox and WebKit; README.md says what the tool does, how to
build, test and serve it, and that the sample content is sample content.

## Handoff (C5 consolidate in your own words)

Finish with HANDOFF.md: what was built, what passed, what is open, and
what [the person] should check by hand before using the tool.
```
