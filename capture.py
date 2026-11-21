"""Renode capture sequence for Contact Book (not yet run).

Run from xous-dev-toolkit:
    python3 scripts/renode_capture.py --app contacts --script ../precursor-contacts/capture.py

Contract: run(ctl, outdir) -> number of failed steps. The app is already launched.
"""
import os


def run(ctl, outdir) -> int:
    failures = 0

    def shot(name):
        nonlocal failures
        if not ctl.try_screenshot(os.path.join(outdir, name)):
            failures += 1

    shot("01_list_empty.png")
    ctl.inject_key("n")                # new contact
    ctl.sleep(1.0)
    ctl.inject_string("Ada Lovelace")
    ctl.inject_line("")                # Enter: next field (phone)
    ctl.inject_string("555-0100")
    ctl.inject_line("")                # email
    ctl.inject_string("ada@example.org")
    shot("02_form.png")
    ctl.timed_key("F2", after=1.5)     # save -> view
    shot("03_view.png")
    ctl.timed_key("BackSpace", after=1.0)
    shot("04_list.png")
    ctl.inject_key("s")                # search
    ctl.inject_string("0100")
    shot("05_search.png")
    ctl.inject_line("")
    ctl.sleep(1.0)
    ctl.inject_key("d")                # delete -> confirm
    ctl.sleep(1.0)
    shot("06_confirm_delete.png")
    ctl.inject_key("n")                # cancel
    ctl.sleep(1.0)
    ctl.timed_key("BackSpace", after=1.0)   # clear filter
    shot("07_list_unfiltered.png")
    return failures
