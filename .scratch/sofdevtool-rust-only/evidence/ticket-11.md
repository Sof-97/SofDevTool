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

## Coordinator native preview checkpoint

Candidate `b7e28f0` ran natively on macOS26.2 arm64 from the temporary bundle
`/private/tmp/sofdevtool-wave6-dqidtkho/Workbench Preview.app`. The source is
now relocated to `crates/app/examples/workbench_preview.rs`. Real screenshots
of Graphite, Frappé with restore confirmation, invalid/disabled state and
zoomed layout are in the coordinator conversation. Observed interactions:
Graphite/Frappé update all surfaces and editors; Increase R updates the
swatch/value; selecting a History row changes selection; Restore selected
opens confirmation and Cancel dismisses it; invalid sample empties result and
disables Format/Copy; Hide/Show History expands/restores the central workspace;
Tab shows a visible focus ring; native window zoom and return preserve state.

The preview has only synthetic in-memory data. Launcher/Settings are explicitly
layout affordances here, not a native-lifecycle acceptance claim. No deletion
was performed. The candidate remains open for the owner. Explicit design
approval was requested asynchronously; it is still pending at this entry.

## Owner decision — 2026-09-23

The owner explicitly approved candidate `b7e28f0`: “Sì, approva e applica questo design”. Ticket 12 and the remaining Utility presentation rollout may now apply this design. Approval covers the visual direction; subsequent behavior, accessibility, native lifecycle and final Release acceptance remain subject to their own checks.
