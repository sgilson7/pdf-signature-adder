# Minimal Builder example: a bounded counter

The smallest tool that exercises every part of Builder. Add, Remove and Reset change a count that stays between 0 and a maximum set in `data/counter.json` (10, in steps of 3).

| Part | Where |
| --- | --- |
| the rule | `crates/core/src/counter.rs`: adding stops at the maximum, removing stops at zero, and the view says which buttons are enabled |
| fast tests, worked out by hand | the `tests` module in `counter.rs` and `api.rs` |
| the boundary tests | `crates/core/tests/boundary.rs` (same file as the template) |
| the shim | `crates/shim/src/lib.rs`: two functions, one call each |
| the page | `web/`: draws the count, the labels and the note, and disables what the core says to disable |
| a browser interaction | `tests/browser/interactions.py`: 0, 3, 6, 9, 10, then Remove to 7 and Reset to 0 |
| the design journal | `../minimal-journal/`: the opening entry and the brief entry; a project's journal is its own repository, this one sits in the reference repository so its check runs here |

```bash
./scripts/setup.sh     # once
./scripts/check.sh     # format, lint, tests, build, import audit, three browsers
python3 scripts/serve.py
```

`../../scripts/prove-checks.sh` plants defects in a copy of this directory to show that each check fails when it should.
