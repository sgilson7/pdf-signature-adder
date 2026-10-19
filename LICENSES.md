# Licences

Every file in this repository that is not code written for it: its source, its author, its terms. The code is under the repository's licence.

## Assets

| Path | Source | Terms |
| --- | --- | --- |
| `tests/browser/sample.pdf` | written by `crates/core/src/pdf/fixture.rs` | this repository's licence |
| `builder-setting/` | Builder, cloned from github.com/sgilson7/builder-setting at the person's request, `.git` removed | MIT (`builder-setting/LICENSE`) |

## Vendored code

| Path | Project and version | Author | Licence | Checksum |
| --- | --- | --- | --- | --- |
| `web/vendor/pdfjs/pdf.mjs`, `pdf.worker.mjs` | pdf.js (`pdfjs-dist`) 4.10.38, the same files PDF Redactor vendors; compared byte for byte with the package on unpkg on 2026-10-19 | Mozilla and contributors | Apache-2.0 (`web/vendor/pdfjs/LICENSE`) | `web/vendor/pdfjs/SHA256SUMS` |

## Code taken from other repositories

| From | What | Licence |
| --- | --- | --- |
| `kits/privacy` in Builder | the shape of `scripts/plant_leak.py`, `scripts/prove-privacy.sh` and the browser check's privacy additions | MIT |

A file with no licence, a likeness, a recording or any third-party picture is a worklist row in `SECOND-ORDER.md` for the person to decide, and stays out of git until they do.
