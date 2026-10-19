"""The minimal example's browser interactions, run by check.py in every engine.

Expected values are worked out by hand from data/counter.json (max 10, step 3):
add four times gives 3, 6, 9, then 10, where the counter stops.
"""


def run(page, check, engine):
    count = lambda: page.inner_text("#count")
    check(count() == "0", "the counter starts at 0")
    check(page.is_disabled("#remove"), "Remove is disabled at 0")
    seen = []
    for _ in range(4):
        page.click("#add")
        seen.append(count())
    check(seen == ["3", "6", "9", "10"], f"Add steps by 3 and stops at 10 {seen}")
    check(page.is_disabled("#add"), "Add is disabled at the maximum")
    check(page.inner_text("#note") == "The counter stops at 10.", "the core's note is drawn")
    page.click("#remove")
    check(count() == "7", "Remove steps back by 3")
    page.click("#reset")
    check(count() == "0" and page.is_disabled("#remove"), "Reset returns to 0")
