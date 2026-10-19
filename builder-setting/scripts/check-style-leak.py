#!/usr/bin/env python3
"""Keep the default style pack inside template/styles/.

The default pack is Builder's fallback, so its colour values and the name of
the picture it was drawn from must not appear in the template, the docs or
anything else people copy. This reads every colour in
template/styles/default/palette.json (matched as a whole token, with or without '#',
in any case) and the reference names, and searches the shared files.

    python3 scripts/check-style-leak.py [root]

root defaults to the repository; prove-checks.sh passes a planted copy.
Prints each hit as path:line: token and exits 1, or prints a success line.
"""
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve()
ROOT = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else HERE.parents[1]
FILES = ["README.md", "BUILDER-SPEC.md", "AGENTS.md", "CLAUDE.md"]
DIRS = ["docs", "template", "examples", "prompts", "kits"]
SKIP = {".git", "target", "dist", ".venv", "node_modules", "__pycache__"}
NAMES = ["Sam_Comm", "Sam Comm"]


def tokens():
    pal = json.loads((ROOT / "template" / "styles" / "default" / "palette.json").read_text())
    hexes = sorted({v.lstrip("#").upper() for k, v in pal.items() if not k.startswith("_")})
    hex_re = re.compile(r"(?<![0-9A-Za-z])#?(" + "|".join(hexes) + r")(?![0-9A-Za-z])", re.I)
    name_re = re.compile("|".join(re.escape(n) for n in NAMES), re.I)
    return hex_re, name_re


def files():
    for name in FILES:
        p = ROOT / name
        if p.is_file():
            yield p
    for d in DIRS:
        base = ROOT / d
        if not base.is_dir():
            continue
        for p in sorted(base.rglob("*")):
            rel = p.relative_to(ROOT)
            if any(part in SKIP for part in rel.parts) or not p.is_file():
                continue
            yield p


def text_of(p):
    data = p.read_bytes()
    if b"\0" in data[:8192]:
        return None
    try:
        return data.decode("utf-8")
    except UnicodeDecodeError:
        return None


def main():
    hex_re, name_re = tokens()
    exempt = {(ROOT / "template" / "styles").resolve(), HERE}
    hits = []
    for p in files():
        rp = p.resolve()
        if rp == HERE or any(e in rp.parents for e in exempt if e.is_dir()):
            continue
        text = text_of(p)
        if text is None:
            continue
        for n, line in enumerate(text.splitlines(), 1):
            for m in list(hex_re.finditer(line)) + list(name_re.finditer(line)):
                hits.append(f"{p.relative_to(ROOT)}:{n}: {m.group(0)}")
    for h in hits:
        print(h)
    if hits:
        print(f"style leak check: {len(hits)} default-style token(s) outside template/styles/", file=sys.stderr)
        return 1
    print("style leak check: no default-style token outside template/styles/")
    return 0


if __name__ == "__main__":
    sys.exit(main())
