# Ticket 01: sofui ownership, JSON and gallery adoption

Implementation candidate, 2026-09-23. Host: macOS 26.2 (25C56), arm64; rustc
1.98.1. The observed checkout HEAD during this worker run was `f1a26e3`
(GitButler workspace state); the coordinator owns commit integration.

## Changed behavior and ownership

- `crates/ui` is now the independently versioned `sofui` 0.1.0 package. GPUI
  process startup moved to the application and gallery entry points. sofui
  retains only `init` and `mount` for component/editor setup. `gpui-platform`
  is an app dependency and a gallery dev dependency, rather than a library
  dependency.
- `Button::with_id` and `primary_with_id` bind a stable ID independently of
  the label. Retained `FocusHandle`s preserve keyboard focus across redraws.
  The product-named button action/key contexts became sofui-owned. JSON and
  gallery use explicit IDs, including changing labels and two same-label Copy
  controls in the gallery. Disabled activation is inert.
- `TextField` and `TextEditor` expose `assign_text` (silent, clears undo) and
  `edit_text` (change notification and undoable). `TextEditor::focus` is now an
  owned focus API. The underlying `gpui-component` editing engine remains
  private to these wrappers. JSON uses assignments for derived output and
  restoration, and edits for Paste/Clear. Gallery starts with Unicode input,
  has real clipboard writes for both Copy buttons, and shows feedback only
  after an actual write.
- Root `AGENTS.md`, `rust/README.md`, and `crates/ui/README.md` state current
  ownership, the later separate release intent, setup, IDs, focus, text
  semantics, dependency licenses, and the working gallery command.

The application dependency intentionally remains named `sofdevtool-ui` as an
alias for package `sofui`. `Button::new`/`primary`, `set_text` and `replace_all`
remain compatibility methods for unmigrated consumers. Ticket 19 can remove
them and the alias after migrating `workbench.rs`, `launcher.rs`, `settings.rs`,
`text_diff/mod.rs` and the Utility workspaces in `utilities/` (Base64, YAML,
URL, Hashes, Identifiers, Timestamps, JWT, Regex, Case, Whitespace, Color,
Sample Data, Random String). JSON and app main also use the temporary alias.

## Verification

All commands used `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target` and
`CARGO_NET_OFFLINE=true` from `rust/`.

| Command | Result |
| --- | --- |
| `cargo test -p sofui` | Passed: 5 editor grapheme tests and 2 public interaction tests. Public tests cover repeated labels, redraw and changed labels, focus/Enter/Space, disabled activation, Unicode text, assignment/change events, undo and undo reset. |
| `cargo check -p sofui --all-targets -p sofdevtool-app` | Passed, including gallery and application compilation. |
| `cargo test -p sofdevtool-app` | Passed: 57 application tests; bin/doc tests also passed. |
| `rustfmt --edition 2021` on owned Rust files | Applied; workspace-wide formatting was avoided while other workers edit. |
| `rustfmt --edition 2021 --check` on owned Rust files | Passed. |

Cargo reported the existing transitive future-incompatibility warning for
`block` 0.1.6. No native application or gallery launch was performed by this
worker, per the coordinator's native-launch ownership. The test harness uses
the dependency Root only as the window host, and exercises sofui's exported
controls and event interfaces. Actual JSON window Copy feedback, accessibility
tree and gallery visual behavior still require coordinator-owned native
acceptance; build and tests are not evidence of native presentation.
