# Contact Book — agent notes

Rebuilt 2026-11-20 on `precursor-appkit` (strategy R from the portfolio
remediation plan). Read `CLAUDE.md` for layout. Rules that matter here:

- One PDDB key per contact with a random name; never reintroduce an index or
  a stored id counter (a lost counter used to overwrite existing records).
- Reload on basis change; never write except on explicit save/delete.
- A record that fails to parse is counted and left alone, never overwritten.
- No contact data in logs or `Debug`.
- Every screen exits on Backspace or F4; `∴` is never delivered to apps.
- Don't claim deniability beyond what the PDDB itself provides.
