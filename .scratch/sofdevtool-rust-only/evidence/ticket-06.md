# Ticket 06: synchronized Color interactions

Implementation candidate, 2026-09-23. Coordinator owns integration and native launch.

## Behavior

- `NumericStepper` is a public sofui control with stable button identities,
  descriptive accessibility labels, retained focus handles, keyboard/pointer
  activation, disabled bounds and signed step callbacks. The consumer applies
  each callback to its latest value via `NumericStepper::stepped`. It contains
  no sRGB or Color Conversion policy. The gallery demonstrates it at a bound.
- Color channel actions now evaluate current editor text, not the last settled
  session. Empty text starts from opaque white; invalid text disables the
  channel control. A channel action calls `TextField::edit_text`, which emits a
  user-edit event and schedules the revision-gated session. Rapid adjustments
  therefore accumulate, retain undo semantics and do not rely on silent
  assignment. Core `color.rs` remains the sole parsing/formatting authority.
- Text, numeric values, swatch and HEX/RGB/HSL outputs derive from the same
  current request while debounce is pending. Copy recomputes that request at
  activation, so an obsolete rendered button cannot copy a former valid color.
  Invalid current text shows its diagnostic and no copyable conversion.
  History still records only the final valid settled revision through the
  existing one-shot session snapshot policy.

## Verification

Commands ran from `rust/` with `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target`
and `CARGO_NET_OFFLINE=true`.

| Command | Result |
| --- | --- |
| `cargo test -p sofdevtool-app picker_events_accumulate_latest_color_and_record_only_settled_revision` | Passed. Real keyboard picker and Copy actions: two pre-settlement steps, post-settlement step, current HEX/RGB/HSL and color, History counts/snapshot, clamping, invalid text and disabled Copy. Controlled GPUI clock; synthetic isolated History root removed after test. |
| `cargo test -p sofui` | Passed: 5 editor, 3 public interaction (including numeric keyboard/boundary), 1 public theme tests. |
| `cargo check -p sofdevtool-app --all-targets` | Passed. |
| `cargo clippy -p sofui --all-targets -- -D warnings` | Passed. |
| `cargo clippy -p sofdevtool-app --lib -- -D warnings` | Passed. |
| `rustfmt --edition 2021 --check` on owned Rust files | Passed. |

During parallel compilation, the shared Cargo target held one stale sofui
artifact lacking the new export. A direct rustc import probe isolated it;
`cargo clean -p sofui` removed that package's target artifacts, and the fresh
build and focused test passed. Another worker's temporary Identifiers test
import error was corrected before the final Color test run.

The GPUI interaction test uses a synthetic window and controlled time. It
does not claim native Workbench appearance or macOS 14/15 runtime validation;
the coordinator owns that acceptance.

## Coordinator native acceptance

On macOS26.2 arm64, the frozen03+06+07 candidate was built as a Debug bundle
and launched from `/private/tmp` with an isolated temporary support root.
Entering `#102030`, then pressing Increase R twice, produced `#1a2030`,
R=26/G=32/B=48 and matching RGB/HSL/swatches. Copy RGB pasted back as
`rgb(26 32 48)`. An invalid current input removed all three Copy controls and
left the four retained valid operations unchanged. Pointer steps settled
individually in this native test; rapid pre-debounce accumulation is evidenced
by the controlled-clock GPUI test above. The test app quit successfully.

The combined default gate passed304 tests, formatting, Clippy and Debug
builds; logs and temporary profile metadata are under
`.artifacts/parallel-rust-only/wave3/`. This is macOS26.2 runtime evidence only.
