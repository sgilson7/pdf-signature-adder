# Online play: a declared off-origin feature

Builder's default build makes no request off its own origin while it loads and idles (requirement R7). Online play between browsers needs to reach other machines, so it cannot fit the default build as it stands. R7 allows it as a *declared off-origin feature*, and [`kits/online/`](../kits/online/) is the pattern for one.

## The rule

A declared off-origin feature is allowed when all five of these hold:

1. **The brief names it.** The person decides the tool reaches other machines, and says which feature does it. An agent does not add one on its own judgement.
2. **It loads only on a player's action.** Nothing off the origin is imported, opened or contacted until a player presses Host or Join. The load-and-idle path stays clean, and the template's browser check, which never presses either, still fails on any WebSocket or off-origin request. That failure staying armed is the evidence.
3. **Its code is vendored and checksummed.** The third-party library sits in a `vendor/` directory with a `SHA256SUMS` file, and `scripts/build.sh` refuses to build when the file differs. Nothing is fetched from a CDN, at build time or at run time.
4. **Its origins are listed in one config file.** `web/net/config.js` is the only file that names a relay. No other page file may hold an address.
5. **A test matches that list against the CSP.** Each page's `connect-src` is `'self'` plus exactly the relays in `config.js`. A relay added to one and not the other fails `cargo test`, and so does an address in any other CSP directive.

The rule says where the feature may reach and when. It does not say the feature is private: relays see that two browsers met, and STUN servers see each browser's public address. The brief says so to the people who will use the tool.

## How the online kit meets it

| Part | Where | What it does |
| --- | --- | --- |
| Transport | `web/net/rtc.js` (written per project) | connects browsers by room code over Trystero and Nostr relays, or by pasted codes with no relay; carries text; decides nothing |
| Config | `web/net/config.js` | app id, protocol tag, relays, ICE servers, timings |
| Door | `web/online.js` | the one page file that imports the transport; passes events to the core and the core's outbox to the transport |
| Rules | `crates/core/src/online.rs` | messages, versions, garbage handling, lockstep, checksums; tested by simulating two browsers with a lagged wire |
| Fence | `crates/core/tests/online_fence.rs` | the file list of `web/net/`, the single importer, the single config, the CSP match, the protocol tag |
| Online check | `tests/browser/online.py` | two browsers meet by pasted codes with `?ice=none` and play, without leaving the origin |

The core never sees a connection. The page hands it a peer id and text, and it answers with text to send. So every rule of online play, including what happens on a garbled message, a different build or a slow link, is reached by `cargo test` in milliseconds.

## What the checks show, and what they do not

The core tests show that the rules hold on the cases they state, including two copies that converge across lags of up to seven frames. The fence shows that the addresses in the code and in the CSP agree. The browser check shows that loading and idling made no off-origin request. The online check shows that two browsers on one machine met and agreed.

None of them shows that the room-code path works over real relays today, that two players on different networks can connect, or that two players behind strict NATs can connect at all (they cannot without a TURN server, which this design does not have). Those are checked by hand before a release, and the result and the date go in `HANDOFF.md`.

## Credits

The transport design (two introductions, a negotiated channel on a fixed id, a hello repeated until acknowledged, peers kept by a stable id, errors as keys for strings in `data/`) and the single config file matched against the CSP are Peter's, from Mandible and Ape Race. They gathered lessons from Floodline and Vagrancy. The fence, the pasted-code browser check and the core tests come from Ribbit King, Level Twenty and Raiders of the Lost Mart. [`kits/online/README.md`](../kits/online/README.md) has the detail.
