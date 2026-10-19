// Online settings: the only file in the project that names an address on
// another origin. Copy it to web/net/config.js and fill it in.
//
// Read only by web/net/rtc.js, and rtc.js runs only after a player presses
// Host or Join. Loading and idling the page never reads it.
//
// crates/core/tests/online_fence.rs holds this file to three things:
//   - it is the only file outside web/net/vendor/ that names a wss:// address;
//   - every page's CSP connect-src is 'self' plus exactly `relays` below;
//   - `protocol` equals PROTOCOL in crates/core/src/online.rs.
// So a relay added here and not to a page's CSP fails the test, and so does
// a relay in a CSP that is not here.
//
// The page scan in boundary.rs does not read web/net/, so numbers are
// allowed here. None of them is a rule: each is a transport timing or id.

export const CONFIG = {
  // Keeps this project's rooms apart from every other app on the same
  // public relays. Change it per project.
  appId: "replace-me-p2p",

  // The core's PROTOCOL. The room name is appId, this tag and the typed
  // code, and never the build hash: a stale tab and a fresh one must meet so
  // the hello can tell the stale one to reload.
  protocol: 1,

  // The vendored Trystero bundle, pinned in vendor/SHA256SUMS and checked
  // by scripts/build.sh. Imported only when a player uses a room code.
  bundle: "./vendor/trystero-nostr-0.25.4.js",

  // Encrypts session descriptions while they cross relays this project
  // does not run. It ships in this file, so it is not a secret. Game
  // traffic is DTLS-encrypted end to end regardless.
  password: "replace-me",

  // The Nostr relays that introduce players by room code. Every player
  // must use the same list, so it lives here. Check each one answers a
  // WebSocket upgrade before a release; relays come and go.
  relays: [
    "wss://relay-one.example",
    "wss://relay-two.example",
  ],

  // STUN tells a browser its public address. Without TURN, two players who
  // are both behind strict NATs cannot connect; say so in data/, under
  // online.no_path. A CSP connect-src does not govern these addresses.
  iceServers: [
    { urls: "stun:stun.example:3478" },
  ],

  // The negotiated data channel's stream id. Both ends create it with this
  // id, so there is no ondatachannel event to miss. Not 0 or 1: Trystero
  // opens its own in-band channel from 0 upwards.
  channelId: 40,

  // How often a link repeats its hello until the other end acknowledges it.
  helloRepeatMs: 250,

  // How long to gather ICE candidates before a pasted code is shown anyway.
  gatherTimeoutMs: 6000,

  // How long a room code waits for any relay before reporting online.no_relay.
  relayTimeoutMs: 15000,
};

// `?ice=none` drops the STUN servers, so tabs on one machine connect on host
// candidates alone and nothing leaves the origin. tests/browser/online.py
// uses it; a player never needs it.
export function iceServers() {
  const ice = new URLSearchParams(location.search).get("ice");
  return ice === "none" ? [] : CONFIG.iceServers;
}
