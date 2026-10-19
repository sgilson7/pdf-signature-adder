"""The project's own browser interactions, run by check.py in every engine.

check.py has already loaded the page, waited for it to report ready, and let
it idle. This drives the tool the way a person would: open the sample PDF,
draw a signature, place it on the signature line, save, and read the saved
file; then sign page 2 with a picture opened from a file.

Expected values are worked out by hand from the sample's geometry (C4), never
copied from the tool's output: the sample is US Letter (612 x 792 points)
with a signature line at y = 165 points from the bottom, so 627 points from
the top, running from x = 72 to 300.
"""

import json
import pathlib
import struct
import zlib

HERE = pathlib.Path(__file__).resolve().parent
STRINGS = json.loads((HERE.parents[1] / "data" / "strings.json").read_text())
SAMPLE = HERE / "sample.pdf"
LINE_Y = 627 / 792
TAP_X = 186 / 612


def png(width, height, ink_rows):
    """A greyscale PNG: paper (240) with the given rows in ink (30)."""
    raw = b"".join(b"\x00" + bytes([30 if y in ink_rows else 240] * width) for y in range(height))
    chunk = lambda kind, data: struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    head = struct.pack(">IIBBBBB", width, height, 8, 0, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", head) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b"")


def tap(page, selector, fx, fy, touch):
    """A tap or click at a fraction of an element, scrolled into view first."""
    target = page.locator(selector)
    target.scroll_into_view_if_needed()
    box = target.bounding_box()
    at = {"x": box["width"] * fx, "y": box["height"] * fy}
    if touch:
        target.tap(position=at)
    else:
        target.click(position=at)


def run(page, check, engine):
    touch = engine.endswith("ipad") or engine.endswith("iphone")
    check(page.inner_text("#title") == STRINGS["title"], "the title from data/strings.json is drawn")
    check(page.get_attribute("body", "data-screen") == "start", "the start screen is shown")
    check(page.inner_text("#privacy") == STRINGS["privacy_note"], "the start screen says the file stays in the tab")

    # Open the sample.
    page.set_input_files("#pdf-input", str(SAMPLE))
    page.wait_for_selector("body[data-screen='document']", timeout=20000)
    check(page.inner_text("#file-name") == "sample.pdf", "the file's name is shown")
    check(page.inner_text("#file-detail") == "2 pages · no signatures yet", "the page count is read from the file")
    painted = page.wait_for_function(
        """() => {
            const c = document.querySelector('[data-page="0"] canvas');
            if (!c || !c.width) return false;
            const d = c.getContext('2d').getImageData(0, 0, c.width, c.height).data;
            let dark = 0;
            for (let i = 0; i < d.length; i += 4) if (d[i] < 128) dark++;
            return dark > 50;
        }""",
        timeout=20000,
    )
    check(bool(painted), "pdf.js painted page 1 (dark text pixels on the canvas)")
    box = page.locator('[data-page="0"] .paper').bounding_box()
    check(abs(box["height"] / box["width"] - 792 / 612) < 0.01, "page 1 is shown in Letter proportions")

    # Draw a signature on the pad.
    page.click("button[data-action='sheet.open']")
    page.wait_for_selector("#pad")
    pad = page.locator("#pad").bounding_box()
    page.mouse.move(pad["x"] + pad["width"] * 0.1, pad["y"] + pad["height"] * 0.6)
    page.mouse.down()
    for k in range(1, 9):
        page.mouse.move(pad["x"] + pad["width"] * (0.1 + 0.1 * k), pad["y"] + pad["height"] * (0.6 - 0.04 * (k % 2)))
    page.mouse.up()
    check(page.locator("#pad-ink path").count() == 1, "one stroke drawn on the signature pad")
    page.click("button[data-action='signature.use']")
    page.wait_for_selector("body[data-placing='true']")
    check(page.inner_text("#status") == STRINGS["status_placing"], "the status line says where to tap")

    # Tap the signature line on page 1: the signature's bottom edge lands on it.
    tap(page, '[data-page="0"] .paper', TAP_X, LINE_Y, touch)
    page.wait_for_selector('[data-page="0"] .overlay')
    paper = page.locator('[data-page="0"] .paper').bounding_box()
    sig = page.locator('[data-page="0"] .overlay').bounding_box()
    bottom = (sig["y"] + sig["height"] - paper["y"]) / paper["height"]
    centre = (sig["x"] + sig["width"] / 2 - paper["x"]) / paper["width"]
    width_in = sig["width"] / paper["width"] * 612 / 72
    check(abs(bottom - LINE_Y) < 0.004, f"the signature's bottom sits on the line ({bottom:.4f} vs {LINE_Y:.4f})")
    check(abs(centre - TAP_X) < 0.004, f"the signature is centred on the tap ({centre:.4f} vs {TAP_X:.4f})")
    check(abs(width_in - 2.5) < 0.02, f"the signature starts 2.5 inches wide ({width_in:.3f})")
    check(page.inner_text('[data-page="0"] .page-label') == "Page 1 · signed", "page 1 says it is signed, in words")

    # Arrow keys nudge it: Shift+Left moves it 10 points.
    page.keyboard.press("Shift+ArrowLeft")
    moved = page.locator('[data-page="0"] .overlay').bounding_box()
    step = (sig["x"] - moved["x"]) / paper["width"] * 612
    check(abs(step - 10) < 0.2, f"Shift+Left nudges it 10 points ({step:.2f})")
    page.click("button[data-action='selection.done']")

    # A picture from a file, placed on page 2.
    page.click("button[data-action='sheet.open']")
    page.click("button[data-action='sheet.tab'][data-value='open']")
    page.set_input_files("#image-input", files=[{"name": "scan.png", "mimeType": "image/png", "buffer": png(120, 40, {18, 19, 20, 21})}])
    page.wait_for_selector("#preview canvas", timeout=10000)
    # Ink rows 18 to 21 with a 2 px margin (120 / 50) trim to 120 x 8.
    check(page.inner_text("#sheet .source") == "From scan.png, 120 × 40 pixels", "the picture's source is named")
    size = page.evaluate("() => { const c = document.querySelector('#preview canvas'); return [c.width, c.height]; }")
    check(size == [120, 8], f"the picture is trimmed to its ink plus a margin {size}")
    page.click("button[data-action='signature.use']")
    page.click("button[data-action='page.go'][data-value='1']")
    page.wait_for_selector('[data-page="1"] .paper')
    tap(page, '[data-page="1"] .paper', 0.5, 0.5, touch)
    page.wait_for_selector('[data-page="1"] .overlay img')
    check(page.inner_text("#file-detail") == "2 pages · 2 signatures placed", "two signatures are counted")

    # Save, and read the file the browser was handed.
    page.click("button[data-action='file.save']")
    page.wait_for_selector("#saved-backdrop:not([hidden])")
    check(page.inner_text("#saved-title") == STRINGS["saved_title"], "the saved dialog opens")
    with page.expect_download() as info:
        page.click("button[data-action='saved.download']")
    download = info.value
    saved = pathlib.Path(download.path()).read_bytes()
    original = SAMPLE.read_bytes()
    check(download.suggested_filename == "sample-signed.pdf", f"the copy is named sample-signed.pdf ({download.suggested_filename})")
    check(saved.startswith(original), "the signed copy starts with the original, byte for byte (PS-1)")
    check(saved.rstrip().endswith(b"%%EOF") and len(saved) > len(original), "an update is appended after it")
    check(saved.count(b"/PSAsig") >= 2, "both signatures are named in the update")
    page.click("button[data-action='saved.close']")
    page.keyboard.press("Escape")
    check(page.inner_text("#message") == "", "no error message is shown")
