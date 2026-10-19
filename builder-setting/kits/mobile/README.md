# Mobile kit

How to make a Builder tool work on a touch screen: one input path for finger, pen and mouse, a CSS baseline for touch, a manifest so the tool can fill the screen from the home screen, and a tablet pass in the browser check. It comes from Slushline's iPad work (its `MOBILE.md`), which proposed four template defaults. This kit keeps them as an option a project takes on purpose.

This kit is a README only. Slushline's files are tied to its game, so each section below says what to write and what to test.

## What it changes

| Default | Change |
| --- | --- |
| D4 (the CSP) | adds `manifest-src 'self'`, or the browser refuses the manifest |
| D7 (the browser check) | adds a tablet engine beside the template's `webkit-iphone` |

Row for the project's change table:

```markdown
| D4, D7 | CSP adds `manifest-src 'self'`; the browser check adds a `webkit-ipad` pass | the tool is meant for tablets (kits/mobile) | SECOND-ORDER.md |
```

Two defaults bear on it without changing. D3 keeps numbers out of page JavaScript, so the smallest touch target (44 CSS pixels, Apple's guidance) goes in `data/`. D17 bans browser storage, so a touch setting the reader chooses lasts for the visit only, unless the brief decides to keep it and the project records that change too.

## M1. Devices become actions; only actions become input

A device produces actions (`fire`, `left`), and actions become the core's input bits in one place. The core never learns which device was used, so its rules, its tests and its replays are the same for touch as for keys. The realtime kit's `web/input.js` already has this shape for keys and the mouse; touch extends it.

- Use Pointer Events, not touch events. One path covers a finger, a pen and a mouse.
- Capture a pressed pointer (`setPointerCapture`), so a finger that slides off its target still releases where it lifts. Release on `pointerup`, `pointercancel` and `lostpointercapture`. Wrap the capture call: a refused capture throws, and an error thrown in a listener is a console error the check fails on.
- The page answers one question: which action is under this point. It answers from the geometry the core already sent for drawing, and never with a target smaller than the size in `data/`.
- Two modes, if the brief wants them: *hold*, where an action is on while the finger is down, and *tap*, where a press latches the action until the next press. The latch belongs to the input layer, like a sticky key; the core still sees only which bits are held each tick. Controls that must never latch, such as a throttle, use a hold-only button.
- Combine every device: `keys.bits | touch.bits`. A keyboard on a tablet still works.

Tests: a held finger turns the action on and lifting turns it off; in tap mode a tap latches it and the next tap clears it; a pointer that leaves its target still releases; a cancelled pointer releases.

## M2. A touch baseline in CSS

- `touch-action: manipulation` on the page: no double-tap zoom and no tap delay. `touch-action: none` on the play area, so pressing it never scrolls.
- No text selection, callout or tap highlight over the play area.
- Under `@media (pointer: coarse)`: targets at least the size in `data/`, keyboard hints hidden ("Press Enter"), and on-screen controls shown in their place.
- On a short screen the play area fits the screen's height, so on-screen controls stay visible under the browser's own toolbars.

Test: the browser check reads `getComputedStyle(...).touchAction` on the page and on the play area.

## M3. Filling the screen

- A web app manifest (`display: fullscreen`) and an opaque `apple-touch-icon`. iOS fills a transparent corner with black. Have `scripts/build.sh` write the manifest from `data/`, so the tool's name and colours have one home.
- `viewport-fit=cover` in the viewport meta element, and padding by the safe-area insets when the page is started from the home screen.
- A fullscreen button only where the Fullscreen API exists, including Safari's prefixed form. iPadOS Safari has it; a phone's Safari and Playwright's mobile WebKit do not. Where it is missing on a touch screen, say how to fill the screen another way, in a string from `data/`: "To play fullscreen, choose Share, then Add to Home Screen."
- No service worker. It would let the tool work offline, but the browser check fails on any worker, and caching a build needs its own decision about updates.

Tests: the manifest and every icon it names load from the origin; the fullscreen button is shown exactly where the API exists.

## M4. A tablet pass in the browser check

The template's check already offers `webkit-iphone`. Add a tablet beside it, using a Playwright device profile such as "iPad Pro 11 landscape": touch, a mobile browser, a tablet's screen. Give `interactions.py` the device, so a project can drive its tool by touch:

- drive menus with `page.tap`;
- hold a finger by dispatching a `PointerEvent` with `pointerType: 'touch'`, which is what a held touch becomes in the browser;
- check both input modes, the on-screen controls, the play area fitting the screen, the fullscreen button or the home-screen line, the manifest, `touch-action`, and the usual: no console error and no request off the origin.

Break each of these once and watch it fail before relying on it.

## What emulation cannot show

Real multitouch timing, Safari's real fullscreen on a tablet, the home-screen install, and how the tool feels under a hand. These are rows for a person with the device, written in `TRIAGE.md`.

## Which tools need more

- Games that aim with a locked mouse need a virtual stick or a drag-to-aim area on a touch screen. That is a design decision for each game.
- Tools built from menus, lists and forms mostly need M2 and M3.
- A tool whose drag code already uses Pointer Events needs little beyond M2.

## Credits

Slushline's `MOBILE.md` and its iPad work: the layers, the hold and tap modes, the home-screen route on iOS, and the tablet pass. The pointer-capture pattern it built on came from CirqueGame's `web/input.js`.
