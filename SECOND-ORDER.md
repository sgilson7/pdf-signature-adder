# Second-order notebook

Stage 6, kept by the agent as the work happens, not afterwards. `B3 Build in self-questioning`.

The code and tests are the work; this file is the record about the work. It is where a decision the agent made alone, an assumption that turned out wrong, or something only the person can do becomes visible to the person. With the plan approved in advance, it is the main place those decisions surface before the person uses the tool. [`docs/procedure.md`](https://github.com/sgilson7/builder-setting/blob/main/docs/procedure.md#the-second-order-notebook) says why.

One row per assumption, surprise or open item. **Kind** is *divergence* (what was assumed, what turned out to be true, what changed), *finding*, or *worklist* (the person's to do). Number rows by round (R1.1, R1.2, then R2.1 after the first triage) so a later round does not renumber an earlier one. **Rule** is the rule the row produced for later sessions, and where it now lives (`CLAUDE.md`, a test, `tests/breaks.json`), or blank. **Status** is *done*, *open*, or *person*; every *person* row is named in `HANDOFF.md`. `scripts/notebook_check.py` checks the form of each row and that the notebook is not empty once `PLAN.md` holds a decision, not what the rows say.

| # | Kind | Assumed or found | What is true | What changed | Rule | Status |
| --- | --- | --- | --- | --- | --- | --- |
| R1.1 | divergence | `prompts/start-from-description.md` puts the project in a new sibling folder and leaves the reference repository untouched. | The person asked for Builder to be cloned *into* this folder with its `.git` removed. | `template/` was copied to this folder's root, which is the project; `builder-setting/` stays inside as an unmodified reference copy. It will be in the public repository (MIT, public already). The journal sits at the default `../PDF-Signature-Adder-journal/`. | | done |
| R1.2 | finding | The baseline `./scripts/check.sh` would run on an untouched copy. | `BRIEF.md` was filled while the check ran, so step 8 (journal) failed: a brief with a description and no journal. Steps 1–7 passed (38 tests). Step 9, run alone afterwards, passed in Chromium, Firefox and WebKit. | Baseline recorded as steps 1–7 plus step 9, with step 8's failure explained. | | done |
| R1.3 | divergence | The journal opens on its *All entries* page. | The person asked for the analysis on the first tab and the mockups on the second. | The journal's tabs are ordered *How I read this*, *Mockups*, *All entries*, and it opens on *How I read this*. | | done |
| R1.4 | finding | The privacy kit and the mobile kit can both be copied. | The privacy kit's CSP sets `manifest-src 'none'`; the mobile kit's home-screen manifest needs `manifest-src 'self'`. Its kit README says the brief decides. | Default: privacy wins and there is no manifest; Safari's Add to Home Screen still works without one. Put to the person as PS-Q8. | | open |
| R1.5 | finding | pdf.js would load its worker and font data when needed. | The privacy kit's check fails on *any* request after the page is ready, same-origin included. pdf.js fetches its worker, standard fonts and CMaps lazily, after ready. | Plan: import the worker module before ready and run pdf.js on the main thread (`globalThis.pdfjsWorker`), and let it fall back to system fonts. This keeps `worker-src 'none'`. To be measured in stage 5. | | open |

## Strings written by the agent

Each reader-visible string the agent wrote, with its key, for the person to read.

| Key | String |
| --- | --- |
