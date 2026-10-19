#!/usr/bin/env python3
"""Online check: two browsers meet by pasted codes and play on one origin.

Part of the online kit (kits/online/README.md). Copy it to
tests/browser/online.py, fill in SELECTORS for the project's page, and run it
after scripts/build.sh. It sits beside the browser check, not inside it,
because check.py fails on any WebSocket or off-origin request and must keep
doing so.

How it stays on the origin:

- `?ice=none` makes web/net/config.js drop the STUN servers, so the two
  browsers connect on host candidates alone;
- the pasted-code path carries the whole handshake in the codes, so no relay
  is contacted;
- host addresses are not hidden behind mDNS names, which a CI runner may not
  resolve.

It fails on: a page or console error, any request off the origin, any
WebSocket (the room-code path must not start), a browser that never reaches
the playing state, the two browsers disagreeing at the end, or a browser
reporting a desync.

What it does not cover: the room-code path over public relays, STUN, TURN,
and two real networks. Those are checked by hand before a release and the
result written in HANDOFF.md.

    python3 tests/browser/online.py [chromium firefox ...]   (default: chromium)
"""
import functools
import http.server
import json
import pathlib
import sys
import threading

from playwright.sync_api import sync_playwright

ROOT = pathlib.Path(__file__).resolve().parents[2]
DIST = ROOT / "dist"
STRINGS = json.loads((ROOT / "data" / "strings.json").read_text())

# The project's own element ids. Each one is an element the page draws from
# the core's view; the check reads text and never computes a rule.
SELECTORS = {
    "host": "#online-host",          # starts hosting by pasted code
    "offer_out": "#offer-out",       # the host's code, to copy
    "join": "#online-join",          # opens the joiner's form
    "offer_in": "#offer-in",         # where the joiner pastes the host's code
    "make_answer": "#make-answer",
    "answer_out": "#answer-out",     # the joiner's reply code
    "answer_in": "#answer-in",       # where the host pastes the reply
    "accept": "#accept-answer",
    "status": "#online-status",      # shows the core's status string
    "state": "#state",               # shows a summary of the world to compare
}
# The page sets these on <body> when a code is ready to copy. Waiting on a
# selector works in every engine; page.wait_for_function does not run under
# Firefox when the CSP forbids eval.
OFFER_READY = "body[data-offer='ready']"
ANSWER_READY = "body[data-answer='ready']"
TIMEOUT_MS = 15000

failures = []


def check(ok, what):
    print(("  PASS " if ok else "  FAIL ") + what)
    if not ok:
        failures.append(what)


class Quiet(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
        ".js": "text/javascript",
    }

    def log_message(self, *args):
        pass


def serve():
    handler = functools.partial(Quiet, directory=str(DIST))
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


# Both browsers share this machine, so plain local addresses are what lets
# them meet without STUN.
LAUNCH = {
    "chromium": {"args": ["--disable-features=WebRtcHideLocalIpsWithMdns"]},
    "firefox": {"firefox_user_prefs": {"media.peerconnection.ice.obfuscate_host_addresses": False}},
}


def open_page(browser, origin, who, seen):
    context = browser.new_context()
    page = context.new_page()
    page.on("console", lambda m: seen["errors"].append(f"{who}: {m.text}") if m.type == "error" else None)
    page.on("pageerror", lambda e: seen["errors"].append(f"{who}: {e}"))
    page.on("websocket", lambda ws: seen["sockets"].append(f"{who}: {ws.url}"))
    page.on(
        "request",
        lambda r: None if r.url.startswith((origin + "/", "blob:", "data:")) else seen["offsite"].append(r.url),
    )
    page.goto(origin + "/index.html?ice=none")
    page.wait_for_selector("body[data-ready='true']", state="attached", timeout=TIMEOUT_MS)
    return page


def run(p, engine, origin):
    print(f"\n[{engine}] online, two browsers, pasted codes")
    browser = getattr(p, engine).launch(**LAUNCH.get(engine, {}))
    seen = {"errors": [], "sockets": [], "offsite": []}
    host = open_page(browser, origin, "host", seen)
    joiner = open_page(browser, origin, "joiner", seen)
    s = SELECTORS

    host.click(s["host"])
    host.wait_for_selector(OFFER_READY, state="attached", timeout=TIMEOUT_MS)
    offer = host.input_value(s["offer_out"])
    check(bool(offer), "the host shows a code to send")

    joiner.click(s["join"])
    joiner.fill(s["offer_in"], offer)
    joiner.click(s["make_answer"])
    joiner.wait_for_selector(ANSWER_READY, state="attached", timeout=TIMEOUT_MS)
    answer = joiner.input_value(s["answer_out"])
    check(bool(answer), "the joiner shows a reply code")

    host.fill(s["answer_in"], answer)
    host.click(s["accept"])
    playing = STRINGS["online.playing"]
    for who, page in (("host", host), ("joiner", joiner)):
        try:
            page.locator(s["status"], has_text=playing).wait_for(timeout=TIMEOUT_MS)
            ok = True
        except Exception:
            ok = False
        check(ok, f"the {who} reaches the playing state")

    # Play: the project's own inputs go here, the same on both pages.
    # Keep it short; the core suite already plays long lagged matches.
    for _ in range(20):
        host.keyboard.press("ArrowRight")
        joiner.keyboard.press("ArrowLeft")
    host.wait_for_timeout(2000)

    states = [host.inner_text(s["state"]), joiner.inner_text(s["state"])]
    check(states[0] == states[1], f"both browsers show the same world {states}")
    statuses = [host.inner_text(s["status"]), joiner.inner_text(s["status"])]
    check(STRINGS["online.desync"] not in statuses, f"no browser reports a desync {statuses}")
    check(not seen["errors"], f"no page or console error {seen['errors'][:3]}")
    check(not seen["offsite"], f"no request off the origin {seen['offsite'][:3]}")
    check(not seen["sockets"], f"no WebSocket, so no relay was contacted {seen['sockets'][:3]}")
    browser.close()


def main():
    if not (DIST / "index.html").exists():
        sys.exit("dist/ is missing: run scripts/build.sh first")
    engines = sys.argv[1:] or ["chromium"]
    server = serve()
    origin = "http://127.0.0.1:%d" % server.server_address[1]
    with sync_playwright() as p:
        for engine in engines:
            run(p, engine, origin)
    server.shutdown()
    if failures:
        print(f"\nONLINE CHECK FAILED: {len(failures)} problem(s)")
        for f in failures:
            print("  - " + f)
        sys.exit(1)
    print(f"\nONLINE CHECK PASSED in {', '.join(engines)}")


if __name__ == "__main__":
    main()
