# precursor-contacts: Contact Book

An offline address book for the [Precursor](https://precursor.dev). Contacts live in the device's PDDB (encrypted flash); there is no network code, no sync and no export.

## Features

- **Contacts** with name (required, up to 40 chars), phone (20), email (60), notes (120) and a category (Personal, Work, Family, Other). Limits count characters, so accented names work.
- **Search** as you type across name, phone, email and notes, case-insensitive. The filter stays on the list until you clear it.
- **Sort** alphabetically or by category then name. Both use the same case-insensitive name order.
- **View** shows every field and which PDDB basis the contact lives in.
- **Add / edit / delete**, with a confirmation before deleting and before discarding unsaved edits. Saving without a name keeps the form and says "Name is required".
- **Basis aware.** Locking or unlocking a basis reloads the list, so contacts in a locked basis disappear from memory and screen, and contacts in a newly unlocked basis appear. The new-contact form shows which basis it will be saved to, and the confirmation names the basis it landed in.

## Keys

| Screen | Key | Action |
|---|---|---|
| List | ↑ ↓ | Move |
| | Enter | View contact |
| | N or F1 | New contact |
| | S, / or F2 | Search |
| | O | Toggle sort (A-Z / category) |
| | D or F3 | Delete (asks Y to confirm) |
| | Backspace | Clear the search filter; with no filter, leave the app |
| | F4 | Leave the app |
| Search | type | Filter as you type |
| | Enter | Back to the filtered list |
| | Backspace | Delete a character; on an empty query, cancel |
| | F4 | Cancel (clears the filter) |
| View | E, Enter or F1 | Edit |
| | D or F3 | Delete (asks Y to confirm) |
| | Backspace / F4 | Back |
| Edit | ↑ ↓ | Previous / next field |
| | type | Into the selected text field |
| | Enter | Next field; on Category, cycle; on [Save], save |
| | ← → or Space | Cycle category (on Category) |
| | Backspace | Delete a character; on an empty field, previous field |
| | F2 | Save from any field |
| | F4 | Cancel (asks first if anything changed) |

The menu key `∴` opens the system menu; the app never sees it. Leaving hands focus back to the system; the app keeps running and reopens from the menu.

## Storage

PDDB dictionary `contacts.data`, one key per contact named `c.<16 random hex>`, holding JSON `{"version":1, "name", "phone", "email", "notes", "category"}`.

- There is no index and no id counter. Record names are random, so a lost counter can no longer make a new contact overwrite an old one, and a new contact can't collide with one in a locked basis. If the TRNG fails, the contact isn't saved and the app says so.
- New contacts go to the most recently unlocked basis. Editing a contact rewrites its key in the basis it already lives in (delete and recreate, then sync), so contacts don't migrate between bases.
- Nothing is written except on save and delete; switching apps writes nothing.
- A record that can't be parsed, or that a newer version wrote, is counted as "unreadable" on the list and left untouched.
- Delete removes that contact's PDDB key and syncs. The PDDB doesn't overwrite the old flash pages immediately, so treat deletion as "gone from the app", not secure erasure.

### What the PDDB does and doesn't give you

Contacts are encrypted at rest by the PDDB. Anyone who can unlock a basis can read every contact in it. Contacts saved while a secret basis is open go into that basis and are hidden when it is locked; that is the PDDB's own plausible-deniability behaviour, which the app now respects (reload on basis change, no writes into the wrong basis). Within an unlocked basis, the dictionary name and the number of `c.*` keys reveal that you keep contacts and roughly how many.

## Limitations

- No vCard import/export and no QR export yet.
- Contact text is wiped from the app's own buffers on drop where practical (`zeroize`), and `Debug` output never contains it. Copies made by GAM when drawing, and transient buffers inside `serde_json` and the PDDB, are outside the app's control.
- The 0.1.0 keys (`index`, `config`, `contact.<id>`) are not migrated: that version never compiled, so no device has such data.
- Renode capture not yet run. `capture.py` is written but not executed; verification so far is the hosted build, clippy, lint and 35 host tests.

## Build and test

```bash
cd core && cargo test                                   # host tests
cd xous-dev-toolkit && bin/integrate --xous-root ~/xous-core --grade ../precursor-contacts
cd ~/xous-core && cargo xtask run contacts               # hosted mode
```

Layout: `core/` (`contacts-core`: record format, search, sorting, screen state machine; no Xous deps), `src/main.rs` (message routing), `src/app.rs` (drawing, PDDB, TRNG on `precursor-appkit`).

---

Made by Tyler Colby, [Colby's Data Movers, LLC](https://colbysdatamovers.com). Issues: [GitHub](https://github.com/tbcolby/precursor-contacts/issues).

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).
