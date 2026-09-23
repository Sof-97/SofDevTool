# Ticket 11 — native Workbench preview candidate

Implementation candidate: `rust/crates/app/examples/workbench_preview.rs`.
This is a standalone, auto-discovered GPUI example, not an application entry
point or a second installed app. It imports public sofui controls and uses only
synthetic in-memory catalog, editor, channel, and History data. It does not
construct application preferences, History storage, Utility sessions, or files.

Run from `rust/`:

```sh
CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target CARGO_NET_OFFLINE=true cargo run -p sofdevtool-app --example workbench_preview
```

The candidate opens one 1340 × 820 native GPUI window. Library, Recent and
Favorites filter a grouped 15-Utility catalog; search filters within the
active scope. A central editor/result pair, validation feedback, explicit
copy, a numeric RGB control and swatch, and a collapsible trailing History
show the intended compact hierarchy. The view uses sofui `Button`,
`TextField`, `TextEditor`, `panel`, `NumericStepper`, `SelectableList`,
`HoldButton`, `ConfirmationBar`, and semantic theme tokens. Graphite and
Catppuccin Frappé buttons update the live window and editor palette. Buttons,
list rows and stepper parts have stable IDs and retained focus handles.

Representative actions are bounded to preview state: a malformed JSON sample
shows an error and disables Copy/Format; a valid edit restores the result;
History Restore asks for confirmation; pointer hold clears only preview rows;
keyboard/assistive activation asks for ordinary confirmation. Launcher and
Settings buttons present their shell placement and an explanatory footer
notice; those separate application windows are not started from this example.
The footer counts state-changing preview interactions. The window can be
resized, and the History inspector can be collapsed to assess central density.

## Checks completed

- `rustfmt --edition 2021 rust/crates/app/examples/workbench_preview.rs` — passed.
- `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target CARGO_NET_OFFLINE=true cargo check -p sofdevtool-app --example workbench_preview` — passed.
- `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target CARGO_NET_OFFLINE=true cargo clippy -p sofdevtool-app --example workbench_preview -- -D warnings` — passed.
- `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target CARGO_NET_OFFLINE=true cargo build -p sofdevtool-app --example workbench_preview` — passed.

No native window was launched by this worker. The coordinator owns launch,
screenshots and direct interaction observations. The owner still must review
and explicitly approve the concrete preview before ticket 12's visual rollout.
