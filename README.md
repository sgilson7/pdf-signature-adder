# PDF Signature Adder

Sign a PDF on your iPad or laptop with a drawn or pasted signature, and save the signed copy.

**Use it:** <https://sgilson7.github.io/pdf-signature-adder/>

Open a PDF, draw your signature with an Apple Pencil, a finger or the mouse (or paste or open a picture of it), tap the page where it goes, and save. On an iPad the signed copy goes to Files, or to Mail or AirDrop through Share.

## What happens to your file

The PDF and the signature stay in the browser tab. The page loads once from GitHub Pages and then makes no request of any kind, and it keeps nothing after you close the tab. The browser check fails if the page sends a request after it is ready or leaves anything in browser storage, and `scripts/prove-privacy.sh` plants five leaks to show the check catches each one.

## What the signed copy is

The signed copy is your original file, byte for byte, with the signatures appended as an incremental update, the way Acrobat and Preview add annotations. Text stays selectable, and links and form fields are untouched. A drawn signature is written as vector paths; a picture is written as an image with a transparency mask.

The signature is a picture of your handwriting. It is not a cryptographic digital signature and does not certify anything.

The tool refuses a PDF it cannot amend safely, and says why: one that is encrypted or locked against editing, one certified against any change, or one too damaged to read.

## How it is built

A [Builder](https://github.com/sgilson7/builder-setting) project. Every rule (reading the PDF, where a signature sits on a rotated or cropped page, smoothing strokes, writing the update) is in a Rust core with no input or output, tested in seconds. A WebAssembly shim forwards calls and decides nothing. The page draws what the core describes, and vendored [pdf.js](https://mozilla.github.io/pdf.js/) 4.10.38 paints the pages.

```bash
./scripts/check.sh                          # format, lint, tests, build, audits, browser check
python3 scripts/break_tests.py              # each behaviour test fails with its behaviour broken
./scripts/prove-privacy.sh                  # each planted leak is caught
python3 scripts/smoke.py IN_DIR OUT_DIR     # sign real PDFs and have Poppler check each copy
```

`BRIEF.md` holds the request word for word, `PLAN.md` the build order, `SECOND-ORDER.md` what was decided and found along the way, and `HANDOFF.md` where things stand. `builder-setting/` is a copy of Builder kept for reference.

## Licence

MIT. pdf.js is Apache-2.0; see `LICENSES.md`.
