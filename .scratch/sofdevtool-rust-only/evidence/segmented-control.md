# Segmented control foundation

Added a reusable selection component for caller-defined choices. Consumers
supply stable option IDs and labels, disabled
state, selected ID, retained focus storage, and an `on_change` callback. The
control presents an accessible radio group with named options and exposes
selected, focused, and disabled states. Left/Right wrap and skip disabled
options; Home/End select the first or last enabled option; Enter/Space activate
the focused option. Selection meaning and persistence remain with the caller.

The gallery now demonstrates three generic formats and seven generic options,
including wrapping behavior at narrow widths. The public interaction test covers
pointer selection, disabled pointer rejection, arrow navigation, wrapping,
disabled-option skipping, Home/End, and Enter/Space.

Validation on 2026-09-23, offline with the shared terra target:

- `cargo test -p sofui` — passed: 12 tests across library and integration suites.
- `cargo check -p sofui --example gallery` — passed.
- `cargo clippy -p sofui --all-targets -- -D warnings` — passed.
- `cargo fmt -p sofui --check` — passed after formatting with edition 2021.

No native application or gallery launch was performed; the gallery was compiled
only. Cargo reported the existing upstream future-incompatibility notice for
`block v0.1.6`.
