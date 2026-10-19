# Online kit

Peer-to-peer play between browsers, with no server of the project's own. Two browsers are introduced by a room code over public Nostr relays, or by pasting codes to each other, and then talk over a WebRTC data channel. The core decides everything about the match; the page's transport only connects and carries text.

Use it when the brief names online play. It is the one off-origin feature Builder has a pattern for. [`docs/online.md`](../../docs/online.md) states the rule it follows, in short:

- the brief names the feature;
- it loads only when a player hosts or joins, so loading and idling stay on the origin;
- the third-party code is vendored and checked against a SHA-256 sum;
- every address it can reach is listed in one config file, and a test matches that list against each page's CSP.

## What it changes

| Default | Change | Where |
| --- | --- | --- |
| D3 (no numbers in page JavaScript) | `web/net/config.js` may hold numbers: transport timings and a channel id. | patch to `boundary.rs`, below |
| D4 (CSP `connect-src 'self'`) | `connect-src` is `'self'` plus exactly the relays in `config.js`. | `web/index.html`; `online_fence.rs` |
| Page scan (no network APIs in page JavaScript) | `web/net/rtc.js` may name `RTCPeerConnection` and the other network APIs. No other file may. | patch to `boundary.rs`, below |

Builder requirement R7 already allows a declared off-origin feature, so no requirement changes. Paste this row into the project's `BUILDER-SPEC.md` change table:

```markdown
| D3, D4, page scan | `web/net/` holds the online transport: `config.js` may hold timings and the relay addresses, `rtc.js` may name the WebRTC and WebSocket APIs, and `connect-src` lists the relays in `config.js` | online play is in the brief (kits/online) | `crates/core/tests/online_fence.rs`; SECOND-ORDER.md |
```

## Files

| Kit file | Copy to | What it is |
| --- | --- | --- |
| `web/net/config.js` | `web/net/config.js` | app id, protocol tag, relays, ICE servers, timings. The only file with a `wss://` address |
| `web/net/vendor/README.md` | `web/net/vendor/README.md` | how to vendor Trystero and pin it |
| `crates/core/src/online.rs` | `crates/core/src/online.rs` | the core side: messages, a lockstep session, the required tests |
| `crates/core/tests/online_fence.rs` | `crates/core/tests/online_fence.rs` | the fence around `web/net/` |
| `tests/browser/online.py` | `tests/browser/online.py` | two browsers meet by pasted codes and play, on the origin |

The project writes two more files: `web/net/rtc.js`, the transport, and `web/online.js`, the one page file that imports it. The kit does not ship a transport. The working ones (Peter's in Mandible and Ape Race, the author's in Ribbit King, Level Twenty and Raiders of the Lost Mart) are each tied to their game, and Peter's has no licence yet. The design below is what to write.

## Edits to existing files

1. `crates/core/src/lib.rs`: add `pub mod online;`.
2. `web/index.html` (and every other page): put the relays from `config.js` into `connect-src`, in any order:

   ```html
   connect-src 'self' wss://relay-one.example wss://relay-two.example;
   ```

3. `crates/core/tests/boundary.rs`, in `page_keeps_no_rule_and_names_no_other_origin`. The template scans every file under `web/` except `vendor/` directories. Two files need room, and the fence takes over the checks it gives up. Replace

   ```rust
           let text = fs::read_to_string(&path).expect("read page file");
           for (n, line) in text.lines().enumerate() {
               if without_allowed_urls(line).contains("://") {
   ```

   with

   ```rust
           let text = fs::read_to_string(&path).expect("read page file");
           // Online kit: online_fence.rs owns the relay addresses. config.js
           // holds them with the transport's timings, each page's CSP line
           // lists them, and the fence matches the two exactly.
           let online_config = path.ends_with("web/net/config.js");
           let transport = path.ends_with("web/net/rtc.js");
           for (n, line) in text.lines().enumerate() {
               let csp_line = line.contains("http-equiv=\"Content-Security-Policy\"");
               if !online_config && !csp_line && without_allowed_urls(line).contains("://") {
   ```

   and replace

   ```rust
               problems.extend(report(&path, &scan_page_js(&text)));
   ```

   with

   ```rust
               let mut found = scan_page_js(&text);
               if online_config {
                   found.retain(|v| v.why != "number in the page");
               }
               if transport {
                   found.retain(|v| v.why != "no network requests beyond the page's own files");
               }
               problems.extend(report(&path, &found));
   ```

   `rtc.js` is still held to everything else: no numbers other than 0 and 1, no `Math`, no navigation, no address. `config.js` is still held to no `Math`, no `eval` and no navigation.
4. `scripts/build.sh`: nothing. The template already refuses to build when a file under a `vendor/` directory differs from its `SHA256SUMS`, and copies `web/` with its subdirectories.
5. `data/strings.json`: add a string for each status key the core uses (`online.waiting`, `online.playing`, `online.reload`, `online.garbled`, `online.left`, `online.full`, `online.desync`) and each error key the transport raises (`online.no_bundle`, `online.no_relay`, `online.no_path`, `online.signal_failed`).
6. `tests/browser/interactions.py`: nothing. The browser check never presses Host or Join, so it still sees no WebSocket and no off-origin request. That is the evidence that load-and-idle stays clean.

## The transport design

This design is Peter's, from the header of Mandible's `online.js`, which in turn gathered lessons from Floodline's `quad_rtc.js` and Vagrancy's `rtc.js`.

**Two introductions reach the same link.**

- *A room code.* The page imports the vendored Trystero bundle, and only then, when a player hosts or joins by code. Trystero meets the other browsers on the public relays in `config.js` and hands back one WebRTC connection per peer.
- *Pasted codes.* The host's offer code and the joiner's answer code carry the whole handshake, compressed, with ICE gathering finished first (no trickle). No relay is contacted. This path works where the relays are blocked, and it is the path the automated check uses.

**From the channel down, both paths are the same.**

- Each side creates one *negotiated* data channel on a fixed stream id (`channelId`). There is no `ondatachannel` event, so there is no event to miss (Floodline).
- A message sent before the other side has created its half of a negotiated channel is lost. So each link repeats a hello every `helloRepeatMs` until the other side acknowledges it, and sends no data until the other side's hello has arrived. Data that arrives before a hello is held and delivered after it (Vagrancy).
- A peer is the other browser, keyed by a stable id: Trystero's `selfId`, or an id sent in the hello on the pasted-code path. Trystero may replace the connection to the same browser; the new link goes under the same peer, and unsent text waits in that peer's queue. Vagrancy's first transport treated the new connection as a stranger.
- Every frame starts with one character saying what it is: hello, acknowledgement, data.
- Errors are keys for strings in `data/`, never sentences: `online.no_bundle` (the bundle failed to load), `online.no_relay` (no relay answered within `relayTimeoutMs`), `online.no_path` (no route between the two browsers, usually two strict NATs and no TURN), `online.signal_failed`.

**The transport decides nothing about the match.** It reports three events to the page (`connected`, `message`, `disconnected`) and the page hands each to the core, through the shim, as a peer id and text. The core leaves replies in an outbox, and the page passes them back to the transport. `online.js` is that wiring and nothing more.

**Address list and CSP.** In Mandible, Peter's `online-config.js` is the only file allowed to hold `wss://` addresses, and his boundary test reads its relay list and requires each page's `connect-src` to list exactly those. `online_fence.rs` is that check, written again. Name each relay exactly. Allowing the whole `wss:` scheme lets the page reach any socket server, while a list says which ones a person agreed to.

**`?ice=none`.** `config.js` drops the STUN servers when the page is opened with `?ice=none`, so two tabs on one machine connect on host candidates alone and nothing leaves the origin. Only the automated check uses it.

## The core side

`crates/core/src/online.rs` is a working skeleton: a message enum, a session for each browser, and a toy two-seat world played in lockstep. Replace the world with the project's own and keep the message handling. The page calls `connected`, `left` and `message` when the transport reports them, calls `local` and `step` once a display frame, and sends what `take_outbox` returns.

Rules it keeps, each from a game that needed it:

- **Versions meet, then part with a reason.** The room name is the app id, the protocol tag and the typed code (`room_name`), never the build hash. Ribbit King first put the build hash in the room name, and after every deploy a stale page waited forever in a room nobody else could find (its PLAN row P35). With the protocol tag, ordinary deploys share rooms, and the hello turns a different build away with `online.reload`. A protocol change still separates rooms; the relay timeout then says `online.no_relay` instead of waiting silently. Pasted codes have no room name at all, so the hello's check is what protects that path. Keep the hello's shape fixed so an old copy can always read it.
- **Garbage changes the status and nothing else.** Text that does not parse, a message of an unknown kind, an input for a tick outside the window, a second input for the same tick, and data from a peer that has not said hello all set `online.garbled` and leave the world, the tick and the queues as they were. A message from a browser that never connected is ignored outright (Level Twenty).
- **No guessing.** In lockstep a browser waits for the other seat's input. A project that cannot afford the wait moves to rollback, as Raiders of the Lost Mart did for a four-player shooter.
- **Checksums.** Every `SUM_EVERY` ticks each browser sends a checksum of its world, and a mismatch is reported as `online.desync`. A desync is a bug in a rule that is not deterministic; the core tests should find it first.

### Required tests

Each is implemented in `online.rs` against the example protocol. Each was seen failing with its behaviour broken.

| Test | What it pins down |
| --- | --- |
| `every_message_round_trips` | encoding and decoding agree for every message |
| `the_room_name_carries_the_protocol_and_not_the_build` | deploys do not split rooms |
| `another_build_or_protocol_is_turned_away_with_a_reason` | both ends show `online.reload`; neither plays |
| `garbage_on_the_wire_changes_only_the_status` | seven kinds of junk, a stranger, and an unknown reason on a bye |
| `a_copy_waits_for_the_other_seat_and_does_not_guess` | no tick runs without both inputs |
| `two_lagged_copies_reach_the_world_of_a_copy_fed_on_time` | lags of 0, 1, 3 and 7 frames over 300 ticks |
| `a_peer_that_leaves_is_reported_and_a_third_is_told_the_room_is_full` | departures and a full room |
| `worlds_that_disagree_are_reported_as_a_desync` | the checksum comparison |

A project that moves to rollback adds two more: a corrected input restores the snapshot before its tick and replays to the world lockstep would reach (Raiders' `two_copies_with_lag_roll_back_into_agreement_with_the_true_run` is the model), and a copy never runs more than its limit past its confirmed ticks.

Rows for `tests/breaks.json`, as a start:

```json
{
  "name": "another build is let in",
  "file": "crates/core/src/online.rs",
  "find": "if protocol != PROTOCOL || build != self.build {",
  "replace": "if protocol != PROTOCOL {",
  "test": "another_build_or_protocol_is_turned_away_with_a_reason"
},
{
  "name": "a tick runs on a guess",
  "file": "crates/core/src/online.rs",
  "find": "world.step([a, b]);",
  "replace": "world.step([a, a]);",
  "test": "two_lagged_copies_reach_the_world_of_a_copy_fed_on_time"
}
```

## The fence

`crates/core/tests/online_fence.rs` checks, on every `cargo test`:

| Test | Fails when |
| --- | --- |
| `web_net_holds_the_transport_and_nothing_else` | a file is added to or missing from `web/net/`, or page JavaScript sits in another subdirectory |
| `only_one_page_file_imports_the_transport` | any page file other than `online.js` names `net/rtc.js` or the vendored bundle |
| `only_the_config_names_a_relay` | an address appears in any file under `web/` other than `config.js`, a page's CSP or the vendored bundle |
| `every_page_connects_only_to_self_and_the_configured_relays` | a page's `connect-src` has a relay `config.js` lacks, lacks one it has, or another directive names an address; also when `config.js` names no relay, so an empty list cannot pass by matching nothing |
| `the_protocol_tag_is_the_core_s` | `protocol` in `config.js` differs from the core's `PROTOCOL` |

Three more tests check the fence's own readers. When the kit was written, each fence test was seen failing on a planted defect: an extra relay in the CSP, a missing one, an address in `img-src`, a relay named in `app.js`, a second importer, an extra file in `web/net/`, a protocol mismatch, and an empty relay list with a plain CSP.

## The online check

`tests/browser/online.py` opens two browsers on the local server with `?ice=none`, has them meet by pasted codes, plays briefly, and compares what both show. It turns off mDNS host-address hiding (Chromium's `WebRtcHideLocalIpsWithMdns`, Firefox's `obfuscate_host_addresses`) because a CI runner may not resolve those names. It fails on any WebSocket, so the room-code path cannot start by accident. Fill in `SELECTORS` for the project's page. It waits on selectors and `data-*` attributes: Firefox refuses `page.wait_for_function` when the CSP forbids `eval`.

It is not part of `check.sh`, because `check.py` must keep failing on any WebSocket. Run it beside the check before a release.

## What stays with a person

The room-code path over real relays, STUN, two players on different networks, and two strict NATs, which need a TURN server this design does not have. Check them by hand before a release, and write the result and the date in `HANDOFF.md`. Relays come and go: in October 2026 one of Trystero's default relays redirected every handshake, which put a console error on every page that hosted. Check that each relay in `config.js` answers before a release.

## Credits

The transport design and the address-list check are Peter's (GitHub [ptonkle](https://github.com/ptonkle)), from Mandible and Ape Race. The negotiated channel comes from Floodline, and the repeated hello and the stable peer from Vagrancy. The `web/net/` fence, the pasted-code browser check and the room-name lesson come from Ribbit King; the garbage and wrong-build tests from Level Twenty; the lagged-copies test from Raiders of the Lost Mart. All code in this kit was written fresh for it.
