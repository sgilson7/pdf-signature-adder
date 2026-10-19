# Privacy kit

A stricter profile for a tool that handles a reader's own data (a file they open, text they paste, answers they type) and promises that it stays on their device. The template's check already fails on any off-origin request. This profile also fails on a request to the tool's own origin after the page is ready, and on anything left in browser storage, and it comes with a script that plants leaks to show the checks catch them.

It comes from Status Trends, which analyses a file the reader opens and keeps nothing.

## What it changes

It tightens two defaults and loosens none.

| Default | Tightened to |
| --- | --- |
| D4 (the CSP) | also `font-src 'none'`, `frame-src 'none'`, `worker-src 'none'`, `manifest-src 'none'`, and a `<meta name="referrer" content="no-referrer">` element |
| D7 (the browser check) | also fails on any request after ready, same-origin included, and on anything in cookies, `localStorage`, `sessionStorage` or IndexedDB at the end |

Row for the project's change table:

```markdown
| D4, D7 | stricter CSP; the browser check fails on any request after ready and on any stored data | the tool handles the reader's own data and promises to keep it on the device (kits/privacy) | SECOND-ORDER.md |
```

D17 already bans browser storage names in page source (`no_storage.rs`). This kit adds the check that the page, as run, left nothing behind, which also catches storage reached by a name the scan does not know.

`manifest-src 'none'` conflicts with the mobile kit's manifest. A tool that needs both decides which in its brief.

## Why "no request after ready, even same-origin"

The off-origin check cannot see a page that sends the reader's data back to its own host, for example as a query string on a request for one of its own files. A static host's logs would then hold the data. A tool that fetches everything it needs before it reports ready has no reason to make another request, so any later request is a leak until shown otherwise. `blob:` and `data:` URLs stay inside the browser and are allowed.

This only works when the page really does fetch everything first. Load every file in `data/` before setting `<body data-ready="true">`.

## The CSP

```html
<meta http-equiv="Content-Security-Policy" content="default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self'; style-src 'self'; img-src 'self' data: blob:; font-src 'none'; object-src 'none'; frame-src 'none'; worker-src 'none'; manifest-src 'none'; base-uri 'none'; form-action 'none'">
<meta name="referrer" content="no-referrer">
```

Add a boundary test that each page carries these directives, in the style of `every_page_sets_the_content_security_policy`.

## Additions to `tests/browser/check.py`

Mark the moment the page is ready, record every request after it, and read storage at the end. In `run_engine`, beside the existing listeners:

```python
after_ready = []
ready = {"yes": False}


def watch_after_ready(request):
    if ready["yes"] and not request.url.startswith(("blob:", "data:")):
        after_ready.append(request.url)


context.on("request", watch_after_ready)
```

After `page.wait_for_selector("body[data-ready='true']", ...)`:

```python
ready["yes"] = True
```

After the interactions:

```python
check(not after_ready, f"no request of any kind after the page reported ready {after_ready}")
stored = page.evaluate("""async () => {
    const idb = indexedDB.databases ? (await indexedDB.databases()).length : 0;
    return {cookie: document.cookie, local: localStorage.length, session: sessionStorage.length, idb};
}""")
empty = {"cookie": "", "local": 0, "session": 0, "idb": 0}
check(stored == empty, f"nothing kept in cookies or browser storage {stored}")
```

An older browser may lack `indexedDB.databases`; the page then reports 0 for it, and the storage scan in `no_storage.rs` still covers the name.

## Proving the checks can fail

A privacy check that has only been seen passing has not been shown to check anything. Status Trends plants one leak at a time in a copy of the tool and requires the core tests or the browser check to catch each. Write two scripts.

`scripts/plant_leak.py` holds the leaks and plants one. Each leak sends or keeps the reader's data at the point the tool reads it. Status Trends used five:

| Leak | What it does |
| --- | --- |
| P1 | requests one of the tool's own files with the data in the query string |
| P2 | writes the data to `localStorage` |
| P3 | sets an image's `src` to another origin with the data in it |
| P4 | sends the data with `navigator.sendBeacon` |
| P5 | posts the data to another origin with `fetch` |

The script must refuse to run on the repository itself, and must assert that the line it inserts after was found, so a renamed line cannot make every plant a silent no-op. Build the off-origin addresses at run time (`"https:" + "//example.com"`) so the boundary scan's address check does not catch the leak before the browser check gets a chance to.

`scripts/prove-privacy.sh` runs them:

```bash
#!/usr/bin/env bash
# Optional audit, outside check.sh: every planted leak must be caught.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PY="${BUILDER_PYTHON:-$ROOT/.venv/bin/python}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

leaks=$(python3 "$ROOT/scripts/plant_leak.py" --list) || { echo "could not list the leaks"; exit 1; }
# An empty list would pass by proving nothing.
[ -n "$leaks" ] || { echo "no leaks to plant: the proof would prove nothing"; exit 1; }

missed=0
for leak in $leaks; do
  rm -rf "$WORK/copy"
  rsync -a --exclude .git --exclude .venv --exclude target "$ROOT/" "$WORK/copy/"
  python3 "$ROOT/scripts/plant_leak.py" "$WORK/copy" "$leak" || { echo "could not plant $leak"; missed=1; continue; }
  caught=""
  (cd "$WORK/copy" && cargo test -q --test boundary --test no_storage >/dev/null 2>&1) || caught="core tests"
  if (cd "$WORK/copy" && ./scripts/build.sh >/dev/null 2>&1); then
    out=$(cd "$WORK/copy" && BUILDER_ENGINES=chromium BUILDER_IDLE=1 "$PY" tests/browser/check.py 2>&1)
    grep -q "BROWSER CHECK PASSED" <<<"$out" || caught="${caught:+$caught; }browser check"
  fi
  if [ -n "$caught" ]; then echo "  caught $leak by $caught"; else echo "  MISSED $leak"; missed=1; fi
done
[ "$missed" = 0 ] && echo "PRIVACY PROOF PASSED" || { echo "PRIVACY PROOF FAILED"; exit 1; }
```

It takes minutes, so it stays out of `check.sh`, like mutation testing. Run it after any change to the page's data path or to the checks, and record the result in `HANDOFF.md`.

## What it does not show

A pass shows what the page did on the check's path, in those engines. It does not show what a browser extension, the operating system or the host does with the page. It does not show that a feature the interactions never reach keeps the data in. Say so where the tool makes its promise to readers.

## Credits

Status Trends: the strict profile in its `tests/browser/check.py`, `scripts/plant_leak.py` and `scripts/prove-privacy.sh`, including the guard against an empty list of leaks. The code here was written fresh for the kit.
