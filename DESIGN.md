# CONTACT BOOK
## Design Specification

```
"Your contacts, your device, your encryption.
No cloud. No sync. No tracking.
Just names and numbers in a vault."
```

---

## Problem Statement

Every contact app syncs to a cloud. Google Contacts, iCloud, Microsoft Exchange — your relationships are someone else's data. Who you know, how often you call them, when you added them — all metadata that builds a social graph.

Precursor's Contact Book keeps your contacts in hardware-encrypted flash with plausible deniability. Your address book exists only on your device. No sync. No backup to someone else's server. No social graph for an algorithm to mine.

---

## Precursor Fit

| Constraint | Advantage |
|-----------|-----------|
| Physical keyboard | Fast name/number entry |
| PDDB encrypted | Contacts are plausibly deniable |
| 1-bit display | Clean text list, readable at any angle |
| No network | Cannot be synced, scraped, or exfiltrated |
| Portable | Always in your pocket |

---

## Features

### Contact Record
- **Name** (required, max 40 chars)
- **Phone** (optional, max 20 chars)
- **Email** (optional, max 60 chars)
- **Notes** (optional, max 120 chars)
- **Category** (Personal, Work, Family, Other)

### Operations
- **Add** new contact
- **Edit** existing contact
- **Delete** contact (with confirmation)
- **Search** by name (incremental filter)
- **Sort** alphabetically (default) or by category
- **Browse** with scrollable list
- **View** full contact details
- **Count** total contacts displayed

### Storage
- All contacts in PDDB, encrypted at rest
- Index for fast lookup
- Max ~200 contacts (PDDB key limit per dictionary)

---

## Screen Flows

```
         ┌──────────┐
         │   List   │◄── Default view
         └────┬─────┘
              │
    ┌─────┬──┴──┬──────┐
    ▼     ▼     ▼      ▼
┌──────┐┌────┐┌──────┐┌──────┐
│ View ││ Add││Search││ Sort │
│Detail││New ││Filter││      │
└──┬───┘└──┬─┘└──────┘└──────┘
   │       │
   ▼       ▼
┌──────┐┌──────┐
│ Edit ││Fields│
│      ││Entry │
└──────┘└──────┘
```

---

## Keyboard Mapping

### Contact List
| Key | Action |
|-----|--------|
| Up/Down | Navigate contacts |
| Enter | View selected contact |
| N | New contact |
| S | Search/filter |
| O | Sort (toggle alpha/category) |
| D | Delete selected (with confirm) |
| Q | Quit app |

### Contact View
| Key | Action |
|-----|--------|
| E | Edit this contact |
| D | Delete (with confirm) |
| Q/Backspace | Back to list |

### Add/Edit Contact
| Key | Action |
|-----|--------|
| Up/Down | Navigate fields |
| Enter | Edit selected field |
| Tab | Next field |
| C | Cycle category |
| Enter (on Save) | Save and return |
| Q/Backspace | Cancel |

### Search
| Key | Action |
|-----|--------|
| A-Z | Type search query |
| Backspace | Delete last char |
| Enter | Apply filter |
| Q | Clear and return |

---

## PDDB Schema

### Dictionary: `contacts.data`
| Key | Format | Description |
|-----|--------|-------------|
| `index` | JSON | Array of contact IDs |
| `contact.{id}` | JSON | Individual contact record |
| `config` | JSON | Sort preference, next ID |

---

## Complexity Estimate

| Metric | Estimate |
|--------|----------|
| Total LOC | ~1,500-1,800 |
| Modules | 5 (main, app, contact, ui, storage) |
| Threading | None |
| PDDB | 1 dictionary, N+2 keys |
| Key challenge | Text input for multiple fields; scrollable list |
