#!/usr/bin/env python3
"""Smoke run on real PDFs (PS-9), outside check.sh.

Signs every PDF in IN_DIR through the core (crates/core/tests/smoke.rs),
then has Poppler, a second reader, render page 1 of each original and each
signed copy. A copy passes when Poppler reads it without an error, the page
count is unchanged, and the pixels that changed sit where the core placed
the signature: centred left to right, its bottom edge 80% of the way down.

    python3 scripts/smoke.py IN_DIR OUT_DIR

The person's files never enter the repository; OUT_DIR should be outside it.
"""
import os
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
DPI = "40"


def ppm(path, out_stem):
    """Page 1 of path, rendered to a PPM; returns (width, height, pixels)."""
    r = subprocess.run(["pdftoppm", "-cropbox", "-r", DPI, "-f", "1", "-l", "1", str(path), str(out_stem)],
                       capture_output=True, text=True)
    errors = [l for l in r.stderr.splitlines() if "Error" in l]
    files = sorted(out_stem.parent.glob(out_stem.name + "*.ppm"))
    if r.returncode != 0 or not files:
        return None, errors or [r.stderr.strip()]
    data = files[-1].read_bytes()
    for f in files:
        f.unlink()
    parts = data.split(maxsplit=4)
    w, h = int(parts[1]), int(parts[2])
    return (w, h, parts[4]), errors


def pages(path):
    r = subprocess.run(["pdfinfo", str(path)], capture_output=True, text=True)
    for line in r.stdout.splitlines():
        if line.startswith("Pages:"):
            return int(line.split()[1]), [l for l in r.stderr.splitlines() if "Error" in l]
    return None, [r.stderr.strip()]


def changed_box(a, b):
    w, h, pa = a
    _, _, pb = b
    xs, ys = [], []
    for y in range(h):
        row = y * w * 3
        for x in range(w):
            i = row + x * 3
            if abs(pa[i] - pb[i]) + abs(pa[i + 1] - pb[i + 1]) + abs(pa[i + 2] - pb[i + 2]) > 60:
                xs.append(x)
                ys.append(y)
    if not xs:
        return None
    return min(xs) / w, min(ys) / h, max(xs) / w, max(ys) / h


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    src, out = pathlib.Path(sys.argv[1]).resolve(), pathlib.Path(sys.argv[2]).resolve()
    out.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, SMOKE_IN=str(src), SMOKE_OUT=str(out))
    run = subprocess.run(["cargo", "test", "-q", "--test", "smoke", "--", "--ignored", "--nocapture"],
                         cwd=ROOT, env=env, capture_output=True, text=True)
    lines = [l for l in run.stdout.splitlines() if l.startswith("SMOKE|")]
    if run.returncode != 0 or not lines:
        print(run.stdout, run.stderr)
        sys.exit("the core's smoke run failed")
    failed = 0
    tmp = out / "_render"
    tmp.mkdir(exist_ok=True)
    for line in lines:
        _, name, status, *rest = line.split("|")
        if status != "signed":
            print(f"  REFUSED {name}: {' '.join(rest)}")
            continue
        n_before, _ = pages(src / name)
        n_after, info_errors = pages(out / name)
        before, _ = ppm(src / name, tmp / "a")
        after, render_errors = ppm(out / name, tmp / "b")
        problems = []
        if n_after != n_before:
            problems.append(f"page count {n_before} -> {n_after}")
        if info_errors or render_errors:
            problems.append(f"Poppler errors {info_errors + render_errors}")
        box = changed_box(before, after) if before and after else None
        if box is None:
            problems.append("no visible change on page 1")
        else:
            cx = (box[0] + box[2]) / 2
            if abs(cx - 0.5) > 0.05 or abs(box[3] - 0.8) > 0.05:
                problems.append(f"signature at x {cx:.2f}, bottom {box[3]:.2f}; expected 0.50, 0.80")
        verdict = "ok  " if not problems else "FAIL"
        failed += bool(problems)
        where = f"x {cx:.2f} bottom {box[3]:.2f}" if box else ""
        print(f"  {verdict} {name}  {' '.join(rest)}  {where}  {'; '.join(problems)}")
    tmp.rmdir()
    print("SMOKE PASSED" if not failed else f"SMOKE FAILED: {failed} file(s)")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
