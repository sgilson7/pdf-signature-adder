# Decisions

Dated entries. Each quotes the person's request word for word, then says what the agent decided and where it is recorded. Grants of autonomy go here too ("you may publish"), with their scope: this run only, or until withdrawn.

## 2026-10-19

> I give you approval to create a public github repo, a public github pages for deployment, and "make deploy" level approval for updating sgilson7 website with the new tool.

The agent decided: the grant covers creating `sgilson7/pdf-signature-adder` (public), publishing it to GitHub Pages with `./scripts/publish.sh`, and running `make deploy` in `~/Documents/website` to list the tool. Recorded in `BRIEF.md`'s Autonomy paragraph. Scope: until withdrawn.

## 2026-10-19 (answers to the opening entry)

> this is excellent, go with all defaults, build the tool and deploy

The agent decided: every question takes the default given in the journal (`#opening`, `#decisions`):

- PS-Q1: repository `sgilson7/pdf-signature-adder`, title "PDF Signature Adder", address `sgilson7.github.io/pdf-signature-adder/`.
- PS-Q2: the core appends an incremental update; the original bytes are never rewritten.
- PS-Q3: `miniz_oxide` joins the core's allowlist.
- PS-Q4: the website entry uses the drafted name and tagline, under Research tools.
- PS-Q5: nothing kept between visits; "Save as image file" instead.
- PS-Q6: signature only.
- PS-Q7: draw on a pad, then place.
- PS-Q8: no manifest; the privacy kit's CSP holds.
- PS-Q9: even line width in three weights.
- PS-Q10: SVG drawn to pixels at four times its placed size.
- PS-Q11: refuse encrypted PDFs and PDFs certified against changes; allow already-signed PDFs with a note.
- PS-Q12: black and blue ink.

The plan is now approved in advance; the person asked to build and deploy.
