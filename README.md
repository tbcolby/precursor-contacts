# Precursor Contact Book

**Your address book in a hardware vault.**

```
"Your relationships are not someone else's data.
Who you know, how often you call them,
when you met — all of this is yours.
Keep it that way."
```

---

## What This Is

An encrypted contact manager for Precursor. Add, search, edit, and organize contacts — all stored in PDDB hardware-encrypted flash with plausible deniability. No cloud sync. No backup to someone else's server. No social graph for an algorithm to mine.

---

## Why Precursor

**PDDB encryption.** Every contact record is encrypted at rest in hardware flash. The plausible deniability feature means contacts can exist in basis sets that are invisible without the correct password. Your address book is private not just from networks, but from anyone holding the device.

**Physical keyboard.** Names, phone numbers, emails — typed directly on hardware keys. No on-screen keyboard, no autocorrect leaking input to a cloud model.

**No network.** Contacts cannot be synced, scraped, or exfiltrated. There is no "sync to cloud" option because there is no cloud.

---

## Features

- **Contact records**: Name, phone, email, notes, category (Personal/Work/Family/Other)
- **Incremental search**: Type to filter contacts by name, live match count
- **Sort modes**: Alphabetical or by category
- **Category tags**: Visual [PER] [WRK] [FAM] [OTH] tags in list view
- **Full CRUD**: Add, view, edit, delete with confirmation
- **Scrollable list**: Browse 200+ contacts with scroll
- **PDDB persistence**: All data encrypted at rest, survives reboot
- **Field editing**: Navigate fields with arrows, type with cursor

---

## Keyboard Reference

### Contact List
| Key | Action |
|-----|--------|
| Up/Down | Navigate contacts |
| Enter | View selected |
| N | New contact |
| S | Search by name |
| O | Toggle sort (alpha / category) |
| D | Delete selected |
| Q | Quit app |

### Editing
| Key | Action |
|-----|--------|
| Up/Down | Navigate fields |
| Enter | Edit field / Save |
| C | Cycle category |
| Tab | Next field |
| Q | Cancel (don't save) |

---

## Architecture

```
src/
  main.rs      — Xous event loop, GAM registration
  app.rs       — State machine (List, View, Add, Edit, Search, ConfirmDelete)
  contact.rs   — Contact data model, search, sorting
  ui.rs        — All rendering
  storage.rs   — PDDB persistence (index + individual records)
```

---

## Build

```bash
cargo build -p contacts --target riscv32imac-unknown-xous-elf
```

---

## License

Apache 2.0 — see LICENSE file.
