#!/usr/bin/env python3
"""Check every relative link and image in the repository's Markdown files.

A link to a file or directory that does not exist fails the run. Links to
other sites are listed, not fetched: whether they answer is a fact about the
day, and a check that depends on the network is not a check this repository
can run in seconds.
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
SKIP = {"target", "dist", ".venv", ".git", "mutants.out"}
LINK = re.compile(r"!?\[[^\]]*\]\(([^)\s]+)\)")

bad, external = [], set()
for md in sorted(ROOT.rglob("*.md")):
    if SKIP & set(md.relative_to(ROOT).parts):
        continue
    text = re.sub(r"```.*?```", "", md.read_text(encoding="utf-8"), flags=re.S)
    for target in LINK.findall(text):
        if target.startswith(("http://", "https://", "mailto:")):
            external.add(target)
            continue
        path = target.split("#")[0]
        if not path:
            continue
        if not (md.parent / path).exists():
            bad.append(f"{md.relative_to(ROOT)}: {target}")

for e in sorted(external):
    print("external (not fetched): " + e)
for b in bad:
    print("BROKEN " + b)
print(f"{'LINKS OK' if not bad else 'LINKS BROKEN'}: {len(bad)} broken, {len(external)} external listed")
sys.exit(1 if bad else 0)
