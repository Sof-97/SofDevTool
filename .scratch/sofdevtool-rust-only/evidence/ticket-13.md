# Ticket 13 — JSON, YAML/JSON, Base64, URL Encoding and JWT

The five text and inspection workspaces now use the approved compact shell
presentation and public sofui controls while retaining their application-owned
sessions, domain operations and History policy. The Workbench supplies each
Utility's primary title, leaving more room for the editors, results and
trailing History.

## Implementation

- JSON's Format/Minify/Query, YAML/JSON direction, Base64 mode and alphabet,
  and URL direction and component now use the reusable `SegmentedControl` with
  stable option IDs and retained focus. The existing session methods remain
  the callbacks. JWT stays an inspection-only flow.
- The five workspaces use stable Button IDs for explicit Copy, Paste, Clear,
  History and Utility-specific actions. Existing editor/diagnostic/result
  behavior remains attached to the current session.
- History presentation uses generic `SelectableList` and `ConfirmationBar`
  instead of the old `HistoryPanel` adapter. Selection, exact restore,
  unavailable-entry messaging, recording controls and retention wording remain
  application-owned. JWT History is still off by default.
- The workspace root padding and repeated Utility title were removed to fit
  the approved Workbench hierarchy. Domain parsing, encoding and persistence
  implementations were not changed.

## Checks completed

- `rustfmt --edition 2021 --check crates/app/src/json_workspace.rs crates/app/src/utilities/yaml_json.rs crates/app/src/utilities/base64.rs crates/app/src/utilities/url_encoding.rs crates/app/src/utilities/jwt.rs` — passed.
- `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target CARGO_NET_OFFLINE=true cargo test -p sofdevtool-app --lib` — 94 passed, 0 failed. This includes live GPUI interaction tests for keyboard mode changes, Unicode input and Copy, invalid-state stale-Copy protection, exact History selection/restore and JWT recording opt-in. The existing JSON interaction test also exercises keyboard mode selection and restore.
- `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target CARGO_NET_OFFLINE=true cargo clippy -p sofdevtool-app --all-targets -- -D warnings` — passed.
- A scoped search of these five files found no remaining `HistoryPanel`, `HistoryItem` or `Button::new` compatibility calls.

These are build and automated interaction results. No native app was launched
for this ticket; direct visual, focus and macOS Clipboard observations remain
for the coordinator's integrated native acceptance pass.
