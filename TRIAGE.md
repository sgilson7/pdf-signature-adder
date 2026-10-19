# Triage

Stage 7. One row per request after the person has used the build: a bug, a change, or a new idea, in the person's own words. Rows are numbered T1, T2 and so on, and each commit that answers one names it ("Fix the timer reset (T4)"). `C4 Verify against hand-computed cases`.

Compare at least one result with a case worked out by hand. **Severity** is *blocks use*, *wrong*, or *wanted*. **Cost** is the agent's estimate. **Disposition** is *done*, *open*, *declined* (with the reason), or *try* (built on a branch and published as a TRY commit that says how to undo it).

| # | Request, in the person's words | Expected | What happened | Severity | Cost | Disposition |
| --- | --- | --- | --- | --- | --- | --- |
| T1 | "works on laptop, failed on ipad with apple pencil. when i tried to sign, it would scroll the view in the background and would only get a single splotch of ink before it would scroll the background view" | A Pencil stroke on the pad draws a whole line and nothing behind the sheet moves. | On a real iPad, Safari began scrolling on the Pencil's first move, cancelled the pointer, and the stroke ended after one dot. The pad relied on `touch-action: none` on its SVG, which WebKit does not reliably honour, and the page behind the sheet was free to scroll. | blocks use | small | done: the pad's input moved to an HTML box with `touch-action: none`, touch events on it are cancelled, a placed signature holds a touch while bare page still scrolls, and an open dialog locks the page. Five browser checks added (R2.1). Awaiting the person's retry on the iPad. |
