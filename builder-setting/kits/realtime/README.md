# Realtime kit

Pieces for games that run on a fixed tick: input that survives between ticks, pointer lock with a fallback, integer maths for movement, chance from a seed, a way to draw less on a machine with no graphics card, and two kinds of test, probes and checks.

## What it changes

No reference default. `fx.rs` and `rng.rs` are how a game meets D2 (no floats, clocks or randomness in the core), and `input.js` passes the page scan as written (D3). `BUILDER-SPEC.md` points to this kit from D2.

## Files

| Kit file | Copy to | What it is |
| --- | --- | --- |
| `crates/core/src/ticks.rs` | `crates/core/src/ticks.rs` | fixed ticks from display frames, with taps latched; tests |
| `crates/core/src/fx.rs` | `crates/core/src/fx.rs` | Q16.16 fixed point: `mul`, `div`, `sqrt`, `length`, `sin`, `cos`; tests |
| `crates/core/src/rng.rs` | `crates/core/src/rng.rs` | SplitMix64 from a seed the page passes in; tests |
| `web/input.js` | `web/input.js` | keys and mouse to action bits; pointer lock with a fallback |

Add `pub mod ticks;`, `pub mod fx;` and `pub mod rng;` to `crates/core/src/lib.rs` for the ones the project keeps. Replace the example `World` in `ticks.rs` with the game's own.

## Input between ticks

The page reads input once per display frame and hands it to the core with the frame's timestamp. The core runs as many fixed ticks as the time allows. Two things go wrong at the edges, and the kit handles both.

- **A press released before the frame reads it.** `input.js` keeps a `tapped` set beside the `held` set. `read()` returns both together and then clears `tapped`, so a key that went down and up between two frames still reaches the core once.
- **A frame that runs no tick.** At 144 frames a second and 60 ticks a second, most frames run no tick. A tap that arrives in such a frame would be gone by the next one. `Clock::frame` keeps the bits of a frame that ran nothing and adds them to the next tick that runs.

The test that pins the second case, from `ticks.rs`:

```rust
#[test]
fn a_tap_released_before_any_tick_ran_still_counts() {
    let (mut c, mut w) = (Clock::default(), World::default());
    assert_eq!(advance(&mut c, &mut w, 0, 0), 1);
    // pressed and let go inside a frame too short for a tick
    assert_eq!(advance(&mut c, &mut w, TICK_US / 4, FIRE), 0);
    // the next frame's bits no longer hold it
    assert_eq!(advance(&mut c, &mut w, TICK_US + 1, 0), 1);
    assert_eq!(w.presses, 1, "the tap was lost");
}
```

The other tests in `ticks.rs` pin that a latched tap is used once, held bits reach every tick of a slow frame, a tap counts once when a frame runs several ticks, a stall runs at most `MAX_TICKS_PER_FRAME` and drops the rest, a clock that goes backwards runs nothing and owes nothing, and 30, 60 and 144 frames a second give the same world over one second.

The page's timestamp is milliseconds with a fraction. The shim converts it to whole microseconds with one multiplication and a cast. For a game object whose `advance` takes microseconds, the input bits and the mouse movement:

```rust
#[wasm_bindgen]
pub fn advance(&mut self, now_ms: f64, bits: u32, dx: i32, dy: i32) {
    self.inner.advance((now_ms * 1000.0) as i64, bits, dx, dy)
}
```

That is a conversion, not a decision, and the shim scan allows it. The core sees integers only.

## Pointer lock and its fallback

`engage()` asks for pointer lock with raw movement (`unadjustedMovement`), then for plain pointer lock if that is refused. Where neither is granted, as in an embedded frame, the fallback takes over: keys still work, and unlocked mouse movement still turns, through `movementX` and `movementY`. Escape always lets go, of a real lock and of the fallback.

Pursuer and Peter's Ape Race skipped the lock when `navigator.webdriver` was set, because a driven browser could grant the lock and drop it at once, which left Pursuer's run frozen mid-play (its SECOND-ORDER row G9). The template now bans `webdriver` in page JavaScript, since a page that tests for automation behaves differently under the check from how it behaves for a reader. So `input.js` reads `?pointer=free` instead: a mode any reader can open, which the browser check uses. In `tests/browser/interactions.py`, open the page with `?pointer=free` for any interaction that engages the game.

Headless Chromium gives synthetic mouse moves no `movementX`. To turn the view in a test, dispatch the event yourself:

```python
page.evaluate("() => window.dispatchEvent(new MouseEvent('mousemove', {movementX: 40, movementY: 0}))")
```

## Integer maths and chance

`fx.rs` holds Q16.16 values in an `i32`. Add and subtract with `+` and `-`; multiply and divide through `fx::mul` and `fx::div`, which widen to `i64` and saturate, so an overflow pins to the limit and never wraps silently. Angles are in turns (`TURN` is a full circle), and `sin` and `cos` use Bhaskara's rational approximation, within 0.002 of the true value. The same inputs give the same bits on every machine, which replays, ghosts and lockstep online play depend on.

`rng.rs` is SplitMix64. The page draws a seed once with `crypto.getRandomValues` and passes it to the core's constructor; the core never reads a random device. A replay stores the seed. Peter's games pass entropy in the same way.

## Drawing less on a software renderer

Classroom machines and CI runners often have no graphics card, and WebGL then runs on a software renderer that draws a few frames a second at full size. Pursuer and Long Time Comin' detect it by name and draw at a lower resolution:

```js
// web/gl.js
const info = gl.getExtension("WEBGL_debug_renderer_info");
const name = info ? String(gl.getParameter(info.UNMASKED_RENDERER_WEBGL)) : "";
const software = display.software.some((s) => name.includes(s));
```

```json
{
  "software": ["SwiftShader", "llvmpipe", "Software", "Microsoft Basic Render"],
  "software_scale": 0.5
}
```

The list and the scale live in `data/display.json`: they are presentation, and D3 keeps numbers out of page JavaScript. The page tells the core (`game.set_low_detail(software)`) so the core can send fewer objects, and multiplies the canvas size by `software_scale`. Browsers may withhold the extension; then `name` is empty and the page draws at full size.

## Probes and checks

A **check** asserts, runs on every `cargo test`, and fails the build. A **probe** measures and prints: a bot that plays whole runs, the instance count of every scene, the time a tick takes in a release build. It is an ignored test, run on demand:

```bash
cargo test --release -p tool_core --test run_probe -- --ignored --nocapture
```

A probe's numbers go in `MEASUREMENTS.md`, with the date and the machine. When a probe finds a bug, the fix gets a check. Long Time Comin's run probe found a dry canal too deep to climb out of (its SECOND-ORDER row G5), which no check had looked for. Keep probes out of `check.sh`: they take minutes, and their numbers depend on the machine.

## Counting frames

`tools/shots.py` in Pursuer and Long Time Comin' serves `dist/`, drives the page through a script of key presses, saves a picture at each step, and counts frames a second by asking the page to count `requestAnimationFrame` callbacks over a fixed window:

```python
fps = page.evaluate("""() => new Promise((done) => {
    let frames = 0;
    const start = performance.now();
    const tick = (now) => {
        frames += 1;
        if (now - start < 2000) requestAnimationFrame(tick);
        else done(frames * 1000 / (now - start));
    };
    requestAnimationFrame(tick);
})""")
```

The function runs in the page through Playwright, so it is not page JavaScript and the page scan does not read it. Use `page.evaluate` with a function: Firefox refuses `page.wait_for_function` with a string when the CSP forbids `eval`. Launch Chromium with `--enable-gpu --ignore-gpu-blocklist` (and `--use-angle=metal` on a Mac) to measure a real graphics card; without them the number is the software renderer's. Write each reading in `MEASUREMENTS.md`.

## Credits

`input.js` follows the input layer shared by Pursuer and Long Time Comin', which has the same shape as Slushline's `keys.js`. The latched tick and its test come from Pursuer's `advance`. The pointer-lock handling, the entropy passed into the core and the WebGL renderer that takes every size from the core are Peter's (GitHub [ptonkle](https://github.com/ptonkle)), from Ape Race and Mandible. The software-renderer list and `tools/shots.py` come from Pursuer and Long Time Comin'. The code was written fresh for the kit.
