# Plan

Stage 2, written by the agent from `BRIEF.md` and the answers of 2026-10-19 (all defaults; `DECISIONS.md`). `B2 Require step-by-step justification`.

Approved in advance: the person said "go with all defaults, build the tool and deploy".

| # | Decision | Reason | Test that shows it holds |
| --- | --- | --- | --- |
| 1 | `num.rs`: decimal text ("312.5", "-0.25") parses to thousandths (`i64`), and thousandths format back as the shortest PDF decimal. | Pointer positions and PDF reals arrive as text; the core has no floats (D2). | Hand-worked cases in `num.rs`, including rounding, signs, exponents refused. |
| 2 | `pdf/` reads a PDF: a lexer for PDF objects, classic cross-reference tables and cross-reference streams, `/Prev` and `/XRefStm` chains, object streams, Flate with PNG predictors (`miniz_oxide`). Newest entry for an object wins. | Most PDFs written since 2010 keep page objects in compressed object streams (PS-Q3). | Fixtures built byte by byte in tests: classic table, xref stream with predictor 12, object stream, an update chain with `/Prev`. |
| 3 | `pdf/` walks the page tree with inherited `MediaBox`, `CropBox`, `Rotate` and `Resources`, and refuses: `/Encrypt`; a DocMDP certification with `/P 1`; a broken cross-reference or a missing page tree; a page tree with a loop. Each refusal has its own message. | PS-5, PS-Q11. | One fixture per refusal; the message is pinned. |
| 4 | `place.rs`: the page's display box (crop box, rotated) in thousandths of a point; a pointer in CSS pixels maps to it; a placement's box maps to a PDF `cm` matrix for rotation 0, 90, 180 and 270. Placing, dragging, resizing by a corner with the shape kept, clamping to the page, arrow-key nudges. | PS-2: signatures land in the wrong place on rotated and cropped scans. | Matrices worked out by hand for each rotation and an offset crop box. |
| 5 | `ink.rs`: the pad is 3000 × 1000 units; pointer events become strokes; a pen seen means touches are ignored; points closer than a threshold are dropped; strokes become Catmull-Rom Béziers, emitted as SVG path data (for the page) and PDF path operators (for the file). Three weights, two inks. | PS-Q7, PS-Q9, PS-Q12. | Hand-worked Bézier control points; palm rejection; a dot becomes a visible mark. |
| 6 | `picture.rs`: RGBA pixels in; light pixels made clear (optional), empty margins trimmed (optional), size capped; RGB and alpha planes out. | Paste and Open image (PS-Q10: SVG is drawn to pixels by the page first). | Hand-made 4 × 3 images. |
| 7 | `write.rs`: an incremental update. Per signed page: a new version of the page object whose `/Contents` is `[q, old…, Q + draw]` and whose `/Resources` is the resolved, merged copy with the new XObjects; one Form XObject per drawn signature, one Image + SMask per picture, Flate-compressed. A classic table, or an xref stream when the original ends in one; `/Prev` to the old `startxref`; `/Root`, `/Info`, `/ID` carried. Then the core reads its output back with step 2's reader and checks every signed page now draws its signature. | PS-1, PS-6. | `sign.rs`: output prefix equals input; the re-read finds the new objects at their offsets; a second update on top of the first also reads. |
| 8 | `api.rs`: a `Handle` the shim holds (the PDF stays in the core); `ui()` describes the screen; one `act(name, value)` door; byte-carrying calls `open_pdf`, `picture`, `save`. | R4 with the template's state round trip changed: a PDF cannot travel as JSON on every pointer move. | `api.rs` tests drive whole flows: open, draw, place, save. |
| 9 | Page: generic regions rendered from the view model, re-rendered only when that region's JSON changed; vendored pdf.js 4.10.38 on the main thread, loaded before ready, painting at sizes the core gives; Pointer Events with coalesced samples; Download and Share. | D3 (no numbers in the page) and PS-7. | Boundary scans; browser interactions open a fixture PDF, draw, place and save, then check the saved file's first bytes equal the fixture's. |
| 10 | Privacy kit (strict CSP, no request after ready, nothing stored, planted-leak proof) and the mobile kit's touch rules and an iPad pass. No manifest. | PS-3, PS-8, PS-Q8. | Browser check in Chromium, Firefox, WebKit and `webkit-ipad`; `scripts/prove-privacy.sh`. |
| 11 | `scripts/smoke.py`: sign a folder of real PDFs through the core, render each output with Poppler's `pdftocairo`, check the prefix and that Poppler reads it without error. Outside `check.sh`; the person's PDFs never enter the repository. | PS-9 needs real files; the tests use built ones. | Its own run on local PDFs, reported in the journal. |
| 12 | Publish to `sgilson7/pdf-signature-adder` on GitHub Pages, run the browser check against the live address, then list it on the website with `make deploy`. | The brief's grant. | `BUILDER_ORIGIN=… tests/browser/check.py`; the website's own gate. |

## Open questions for the person

None blocking. The person's use on an iPad (stage 7) is the check that counts.

## Status

- [x] Read by the person on: 2026-10-19 (approved in advance: "go with all defaults, build the tool and deploy")
