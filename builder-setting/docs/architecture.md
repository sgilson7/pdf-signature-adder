# Architecture

![Builder's three layers: core, shim and page, with the check that watches each](img/architecture.svg)

A Builder tool has three layers, and each one has a check that can see it.

## Core

`crates/core` is a Rust library that holds every application rule and state transition. It performs no browser, filesystem, network, clock, random-device or UI input or output, so `cargo test` reaches every rule without launching a browser. Public functions take ordinary values and return ordinary values or serialisable structures. The template's convention is one module, `api.rs`, whose functions take strings and return `{"ok": ...}` or `{"error": "..."}` as JSON text. An error carries the message the page will show, so the wording of a rule's failure sits beside the rule.

The core describes each screen as a view model. `api::start` and `api::act` return a `Ui`: the screen's name, its panels with their text and actions, and the glossary's definitions. Settings, screens and flags all come back through `act(name, value)` with a dotted name such as `screen.show`, so the core's dispatch is one `match` the suite can test, and a new option needs a new arm and no new shim function. Both conventions come from Peter's Mandible, where every menu and panel is built this way.

The core's dependencies are on an allowlist in `crates/core/tests/boundary.rs`. The reference default is `serde` and `serde_json`, and the reference default also bans floats, hash maps, clocks and randomness so that the same input gives the same output on every machine.

## Shim

`crates/shim` uses `wasm-bindgen` to expose the core to the page. Each exported function converts its arguments, makes one call into the core, and returns the result. The boundary test fails if a branch, a loop, a `match` or the `?` operator appears in the shim's source.

The rule exists because a decision made in the shim is a decision the fast suite cannot reach. One of the seven tools behind Builder grew a shim of several thousand lines, and during play a barrel "showed one thing and sold another" because the shim performed its own lookup. With the rule in the core, there is one rulebook and the tests read it.

Core and shim compile into one WebAssembly module. `wasm-bindgen` pairs it with a JavaScript glue file that loads it. The module reaches the page only through its imports, so listing the imports and following each into the glue shows what the compiled code can reach (`scripts/wasm_imports.py`).

## Page

`web/` is plain HTML, CSS and JavaScript with no framework, bundler or package manager. It loads the module, fetches its content from `data/` on its own origin, passes values to the shim, and draws what comes back. It does not recompute an application rule.

The template's page is a generic renderer. It builds the DOM from the core's `Ui` without knowing what any panel means, marks any label that is a glossary term so a reader can hover it for the definition, and sets `body[data-screen]` from the core's screen name, which gives the browser check a state to wait on. A canvas or WebGL page goes one step further: the core returns a draw list of integers each frame, and the page replays it with op codes, colours, shapes and scales the core sent once (`kits/canvas/`, after Mandible's `render.js` and Ape Race's `gl.js`). Sounds and icons can arrive as bytes the core built. A page built this way holds no number and knows no screen, which is how it meets the reference default that page JavaScript holds no numeric literal other than 0 and 1.

The page scans read every file under `web/` at any depth, except a directory named `vendor`, whose files are pinned by `SHA256SUMS` and checked by `scripts/build.sh`. Shaders live in `data/shaders/`, hold no number other than 0 and 1, and take the values a rule depends on as uniforms from the core. The page does not navigate on its own (`location.href = ...`, `window.open`), and it does not branch on `navigator.webdriver`, because a page that behaves differently under automation sends the browser check down a path no reader takes.

Some state is presentation, and the page may keep it: which panel is open, focus, scroll position, an animation's timing, and the last value the core returned, which the page hands back on the next call. When it is not obvious whether something is presentation or a rule, ask whether a test of the tool's behaviour would need it. If it would, it belongs in the core.

The page sets a Content-Security-Policy in a meta element (reference default). The policy allows WebAssembly with `'wasm-unsafe-eval'`, keeps connections on the origin with `connect-src 'self'`, and blocks form posts with `form-action 'none'`. A meta policy cannot stop a page sending data away by navigating, so the browser check still watches every request.

## Data

Reader-visible text and content live in `data/` (reference default). Content changes without touching a rule, and a copy audit can read every string in one place. `GLOSSARY.md` defines every term and `TONE.md` governs every string; [`glossary.md`](glossary.md) describes both and the tests that hold them. Presentation numbers (sizes, durations, audio) live in `data/*.json`.

## Art

2D art is generated from source: `tools/art/art.py` writes TikZ and renders PNGs, with colours from a style pack. [`art.md`](art.md) describes the pipeline and the default pack.

## What sits outside the default build

A feature that needs a server, a live model, accounts, or shared state between people's devices does not fit the default build, whose finished form is static files with no off-origin request on load. Such a feature needs its own design, checks and approvals. Peer-to-peer play is the one departure Builder has a pattern for: a declared off-origin feature, named in the brief, loaded only when a player hosts or joins, with its relays listed in one config file that a test matches against the CSP ([`online.md`](online.md), `kits/online/`). [`adoption.md`](adoption.md) says more.
