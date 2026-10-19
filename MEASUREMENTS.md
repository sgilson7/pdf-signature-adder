# Measurements

Stage 4, recorded by the agent before building and again at handoff. `B2 Require step-by-step justification`.

The state measured rather than recalled, so later changes can be compared with it. A probe prints numbers and asserts loose bounds at most; record its output here with the command that produced it.

| What | Command | Result | When |
| --- | --- | --- | --- |
| Toolchain | `rustc --version; wasm-bindgen --version` | rustc 1.99.0, wasm-bindgen 0.2.127 | 2026-10-19, before any edit |
| Core tests | `./scripts/count-tests.sh` | 38 passed (template) | 2026-10-19, before any edit |
| Full check | `./scripts/check.sh` | steps 1–7 passed; step 8 failed for want of a journal (R1.2); step 9 passed alone in Chromium, Firefox, WebKit | 2026-10-19, before any edit |
| Core tests | `./scripts/count-tests.sh` | 116 passed, 0 failed, 1 ignored (the smoke run) | 2026-10-19, at handoff |
| WebAssembly size | printed by `./scripts/build.sh` | 489 KB (miniz_oxide included) | 2026-10-19, at handoff |
| Breaks caught | `python3 scripts/break_tests.py` | 13 of 13 | 2026-10-19, at handoff |
| Leaks caught | `./scripts/prove-privacy.sh` | 5 of 5, after the clean copy passed | 2026-10-19, at handoff |
| Real PDFs | `python3 scripts/smoke.py IN OUT` | 14 of 14 local PDFs and 5 rotated or cropped fixtures signed; Poppler read each and found the signature centred with its bottom at 0.80 | 2026-10-19, at handoff |

## Not measured

Devices, browsers and conditions no measurement here covers.

- A real iPad: the Files picker, Apple Pencil pressure and palm rejection, the Safari download location, the share sheet, and a clipboard paste. The browser check's `webkit-ipad` is an emulated touch screen.
- Large scans (50 MB and up) on an older iPad: painting time and canvas memory.
- Acrobat and Preview opening a signed copy; Poppler is the only second reader measured.
- Encrypted, XFA and damaged PDFs from the wild; the refusals are tested on built fixtures only.
