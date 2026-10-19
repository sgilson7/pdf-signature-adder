# Canvas kit

A page that draws on a 2D canvas without holding a number. The core builds a draw list each frame, a flat list of integers, and the page replays it. Op codes, colours, shapes, scales and the canvas size reach the page once, as the core's assets. The kit also has a script that saves screenshots of still scenes the core sets up.

Use it for a game board, a chart drawn by rule, or any picture whose positions come from the core. A page built of HTML elements does not need it.

## What it changes

No reference default. It is how a canvas page meets D3 (no numeric literal other than 0 and 1 in page JavaScript, no `Math`). `render2d.js` passes the template's page scan as written.

## Files

| Kit file | Copy to | What it is |
| --- | --- | --- |
| `crates/core/src/draw.rs` | `crates/core/src/draw.rs` | `DrawList`, the op codes, the palette roles, shapes, scales, `assets()`, `walk()`, and tests |
| `web/render2d.js` | `web/render2d.js` | replays a list onto a canvas |
| `scripts/tiles.py` | `scripts/tiles.py` | saves a screenshot for each `?tile=<spec>` |

Edits:

1. `crates/core/src/lib.rs`: add `pub mod draw;`.
2. The shim: one function returning the assets as text, and one returning the frame's list. `wasm-bindgen` turns a `Vec<i32>` into an `Int32Array`:

   ```rust
   #[wasm_bindgen]
   pub fn draw_assets() -> String {
       tool_core::draw::assets()
   }
   ```

   The frame's list comes from the project's own game object, for example `pub fn draw_list(&self) -> Vec<i32> { self.inner.draw_list() }`.
3. `web/style.css`: one custom property per palette role. The template already defines `--paper`, `--ink`, `--muted` and `--accent`, with dark-theme values. A new role in `PALETTE` needs a new property.
4. `web/app.js`: create the renderer once and draw each frame.

   ```js
   import { createRenderer } from "./render2d.js";
   const renderer = createRenderer(canvas, JSON.parse(draw_assets()));
   function frame() {
     renderer.draw(game.draw_list());
     requestAnimationFrame(frame);
   }
   ```

## How the list works

Each op is a code followed by its arguments:

| Op | Arguments |
| --- | --- |
| `clear` | colour |
| `rect` | x, y, w, h, fill |
| `shape` | shape, x, y, scale, fill, stroke, line width |
| `line` | stroke, line width, n, then n pairs of x and y |
| `text` | x, y, size, fill, align, n, then n Unicode code points |

The core never sends a colour. It sends a palette index, and `assets().palette` names the role at that index (`paper`, `ink`, ...). The renderer reads `--<role>` from the stylesheet and reads it again when the colour scheme changes. The stylesheet keeps the colours; the core keeps which role each thing is drawn in.

There are no floats. A shape's points are in thousandths of its size (`UNIT`), and a scale is the shape's size in thousandths of a pixel. The renderer divides once when it reads the assets. Text is sent as code points, so the list stays integers, and the words come from `data/` like every other string.

`walk()` reads a list the way the renderer does and returns the number of ops, or `None` when a code is unknown or the list ends in the middle of an op. A builder method that forgets an argument fails in `walk` before it shows up as a garbled frame.

## Tests

| Test | What it pins down |
| --- | --- |
| `each_op_writes_its_code_then_its_arguments` | the exact integers for one of each op, worked out by hand |
| `a_built_list_walks_to_its_end_and_a_cut_one_does_not` | the arity of every op |
| `the_assets_name_every_op_role_shape_and_scale_in_order` | the names `render2d.js` and the stylesheet use |
| `op_codes_and_role_indices_are_distinct_and_dense` | the renderer's handler table has no gaps |
| `every_shape_is_closed_polygons_of_three_points_or_more` | no degenerate shapes |

Each was seen failing with its behaviour broken. When the kit was written, the replayer was also run in Chromium against a list holding one of each op, through `scripts/tiles.py`, and the picture checked by eye.

## Screenshots of still scenes

`scripts/tiles.py OUTDIR SPEC [SPEC ...]` opens `index.html?tile=<spec>` for each spec, waits for `<body data-tile="drawn">`, and saves the canvas as `OUTDIR/<spec>.png`. The page passes the spec to the core (for example `act("screen.tile", spec)`), the core builds the scene and holds it still, and the page sets `data-tile` after the first frame it draws. The spec's grammar is the project's: a level and a seed, a screen name, a unit and a pose.

The server binds port 0, so the system picks a free port and two runs never collide. The script fails on a console error, a page error, a request off the origin, or a tile that never reports drawn. It is a tool for looking at the game while building, and for a project's home-page pictures. `check.sh` does not run it.

## Going further: WebGL

Peter's Ape Race takes the same approach to WebGL 2. Its `gl.js` uploads what the core returns and draws it, and takes every size, stride and offset from the core's `numbers()`, so the file holds no layout of its own. Shaders live in `data/shaders/` and take the values a rule depends on as uniforms (D3). The kit does not ship a WebGL renderer; that file is a model for one.

## Credits

The draw-list design is Peter's (GitHub [ptonkle](https://github.com/ptonkle)): Mandible's `render.js` replays an `Int32Array` from the core with op codes, palette, shapes and scales sent once, and Ape Race's `?tile=<spec>` screenshots gave `tiles.py` its shape. Reading colours from CSS custom properties by role is this kit's addition. The code was written fresh for the kit.

## Pictures for the design journal

A game's journal shows the running build at the size the person views it, usually 1920×1080. `scripts/tiles.py` already saves a still for each `?tile=<spec>`; extend the spec with a camera (position, yaw, pitch, distance) so each shot is framed on what changed, and add a flyover parameter (`?map=<key>&at=<metres>`) for a course. Capture headless, look at every shot before it goes in the journal, and retake one that is too dark, cluttered or cropped. A sheet of every model in every colourway, shot the same way, is the journal's *models* entry. `tools/journal/ENTRIES.md` lists the entry kinds.
