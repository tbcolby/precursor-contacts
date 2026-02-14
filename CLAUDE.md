# Contact Book — Agent Instructions

## App Identity
- **Package**: `contacts`
- **SERVER_NAME**: `_Contact Book_`
- **APP_NAME**: `Contacts`
- **manifest context_name**: `Contacts`
- **TCP Port**: 7882 (reserved, not implemented)

## Architecture
- **UX Type**: `UxType::Chat` with raw keyboard input
- **State Machine**: 6 states (List, View, Add, Edit, Search, ConfirmDelete)
- **Threading**: None
- **PDDB**: 1 dictionary (`contacts.data`), N+2 keys

## Key Design Decisions
1. **Index + individual keys** — contact index stored as JSON array of IDs; each contact is a separate PDDB key. This allows individual contact updates without rewriting the entire collection.
2. **Case-insensitive search** — manual byte-level lowercasing (no `to_lowercase()` which needs Unicode tables). Simple substring matching.
3. **Field editing sub-mode** — `editing_field: bool` toggles between navigating fields and typing into them. Same EditField enum for both Add and Edit states.
4. **Confirmation dialog** — Delete requires explicit Y key press. Any other key cancels.

## Build
```bash
cargo build -p contacts --target riscv32imac-unknown-xous-elf
```
