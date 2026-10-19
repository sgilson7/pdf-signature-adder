# 2D art

A Builder tool that needs pictures draws them as text. A Python script writes standalone TikZ, `pdflatex` compiles it, `pdftocairo` renders the PNG, and the PNG is committed. No picture is generated as pixels. The pipeline lives in the template under `tools/art/`; the look lives in a style pack outside it.

Rendering is a step a person runs on a machine with TeX. `./scripts/check.sh` runs only the checks that need no TeX.

## Draw a picture

Register each picture in `tools/art/pictures.py` (or in a `tools/art/art_<group>.py` module) as a function that takes a seeded `random.Random` and returns an `artlib.Fig`:

```python
from artlib import Fig

def badge(rng):
    fig = Fig("icon")                     # size from data/art.json
    fig.circle(1.0, 1.0, 0.7, "gold")     # outlined in ink by Fig
    with fig.frame(1.0, 1.0, s=0.4, rot=rng.uniform(-10, 10)):
        fig.poly([(-1, -1), (1, -1), (0, 1)], "teal")
    return fig

PICTURES["icons/badge"] = badge
```

```bash
python3 tools/art/art.py                 # every picture
python3 tools/art/art.py icons           # one folder, or one name
python3 tools/art/art.py --check-only    # check the committed PNGs; no TeX
```

`icons/badge` is written to `web/art/icons/badge.png`, and its TeX is kept at `tools/art/src/icons/badge.tex`. Each picture's generator is seeded from its name, so a run redraws the same pictures.

What the pipeline enforces:

| Rule | Where |
| --- | --- |
| every filled shape is outlined in the role `ink`, with round joins and caps | `Fig` has no unoutlined fill except `band` (sky strips) and `union` (one outline around overlapping shapes) |
| no words, letters or digits in a picture | `artlib.document` refuses `\node`, `\text`, `\put`, `\char`, `\pgftext`, `label=` and similar before compiling |
| colours are role names the style pack defines | `artlib.document` refuses an unknown role; `tests/art_check.py` fails if `artlib.py`, `art.py` or `style.py` holds a colour literal |
| exact pixel size, alpha and clear edges for cutouts, no transparency in opaque pictures, a file size cap | `artlib.check`, with the values from `data/art.json` `sizes` |
| a calm region stays plain enough for page text | `Fig.keep_calm(box, roles)`, measured on the PNG by `artlib.check` |

`art.py` exits with code 3 and says so when `pdflatex` or `pdftocairo` is missing. Whether a missing TeX skips or fails a check is the check script's decision.

## Supply your own style

`data/art.json` names the style pack:

```json
{"style": "default"}
{"style": "art/my-style"}
```

`"default"` means `styles/default` in the project (the template carries it), or the nearest `styles/default` above it. Any other value is a directory, relative to the repository root unless absolute, holding:

| File | Holds |
| --- | --- |
| `palette.json` | role name to colour; the role `ink` is required; keys starting with `_` are notes |
| `ink.json` | ink width in pt per picture kind, plus `default` |
| `STYLE.md` | the look, as rules for whoever draws |
| `motifs.py` | optional helpers that draw with role names; load them with `style.load().motifs()` |
| `page.css` | optional `:root` tokens for a page that wants its chrome to match |

Pictures name roles and never colour values, so a pack that defines the same roles can replace another without touching picture code. `python3 tools/art/style.py` prints the `\definecolor` lines and ink widths of the current pack.

## The default pack

`styles/default` is Builder's fallback style. It is used only when a brief names no style. It is a reference default, not a Builder requirement: a project that names a style in its brief, or supplies a pack, does not use it. Its `STYLE.md` describes the look as rules an agent can follow.

The default pack's values stay in `template/styles/`, so a copied project has its fallback. `scripts/check-style-leak.py` reads every colour in `template/styles/default/palette.json` and the name of the picture the pack was drawn from, and searches the root documents, `docs/`, `template/` (outside `template/styles/`), `examples/`, `prompts/` and `kits/`. It prints each hit as `path:line: token` and exits 1. The template's page, its CSS and the art code carry none of the pack's values. A project that does not want the pack may delete `styles/default/` once its brief names another style.

## Checks

`tests/art_check.py` runs with plain `python3` and no TeX. It writes small PNGs and confirms that `artlib.check` accepts a good one and refuses each kind of bad one, that `Fig` outlines every fill, that text-setting TeX and unknown roles are refused, that `art.py --check-only` fails on a missing PNG, and that the pipeline's code holds no colour literal. `art.py --check-only` checks the committed PNGs against `data/art.json`. Neither renders anything, so neither shows that the TeX still compiles; that is shown when a person runs `art.py`.
