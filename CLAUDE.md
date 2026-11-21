# Contact Book — Build Notes

Package `contacts`, server name `_Contact Book_`, GAM app name
`gam::APP_NAME_CONTACTS` (manifest key `contacts`, context name `Contacts`).

## Architecture
- `core/` (`contacts-core`, no Xous deps, host-tested): contact record
  format, search across all fields, sorting with one shared name comparator,
  and the screen state machine (`Ui`, which returns `Effect`s for storage and
  leaving).
- `src/main.rs` routes GAM messages (template pattern, `UxType::Framebuffer`);
  `src/app.rs` maps keys to `Ui`, performs effects via `appkit::store::Store`,
  draws via `appkit::screen::Screen`. No pump.
- One PDDB key per contact (`contacts.data` / `c.<16 hex>`, random names from
  the TRNG, fail closed). No index, no id counter. Edits rewrite the key in
  its own basis; contacts are reloaded on every basis change.

## Test / gate
```bash
cd core && cargo test
cd ../xous-dev-toolkit && bin/integrate --xous-root <xous-core> --grade -v ../precursor-contacts
```
