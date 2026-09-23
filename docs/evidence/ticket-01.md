# Ticket 01 evidence — JSON in a GPUI Workbench with reusable text controls

- Base (immutable): `5ed34cc460d9c5cabcd0b795aab79ee25d931bf6`
- Branch: `feat/rust-01-json`
- Provider / model: `acp-opencode` / `opencode-go/deepseek-v4.1-flash`
- Build/test host: macOS 26.2 (build 25C56), arm64
- Toolchain: rustc/cargo 1.98.1 (`aarch64-apple-darwin`)

## Automated gate

| Command | Result |
| --- | --- |
| `cargo test -p sofdevtool-core` (first red) | `error[E0583]` module `json` missing — exit 101 |
| `cargo test --workspace` | 23 JSON contract + 5 session + 2 identity tests, 0 failed — exit 0 |
| `scripts/verify` | `gate passed` — exit 0 |

`scripts/verify` runs, in order and without hiding exit statuses:
`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, and a Debug build of all binaries and examples. It ran
with `MACOSX_DEPLOYMENT_TARGET=14.0`. Clippy is clean (warnings denied); the only
build note is a transitive future-incompatibility warning from `block v0.1.6`
(an `objc` dependency), not first-party code.

## Deployment compatibility and identity

```
otool -l target/debug/sofdevtool     -> minos 14.0, sdk 26.5
otool -l target/debug/examples/gallery -> minos 14.0, sdk 26.5
rust/scripts/bundle-debug -> artifacts/SofDevToolRust.app
  CFBundleIdentifier com.gerardo.sofdevtool.rust, LSMinimumSystemVersion 14.0
```

`WindowOptions.app_id` is a no-op on the macOS backend, so the bundle's
`Info.plist` is the real identity. The bundle is produced locally and is not installed. The resumed
orchestrator session launched local Debug bundles for the native checks below. macOS 14 and 15 **runtime** verification is pending; the
host that builds and runs the gate is macOS 26.2.

## Contract coverage

`sofdevtool-core` (23 contract tests + 15 fixtures):

- format with 2- and 4-space indentation; minify; recursive key sorting that
  preserves array order and number spelling.
- empty/whitespace input is neutral; invalid input yields line/column
  diagnostics and never a stale output.
- numbers keep their exact source literal (large integers, `1.0`, `1e3`, `-0.0`).
- Unicode, accented text, multi-scalar emoji, and `\uXXXX` surrogate pairs.
- a user object keyed `$serde_json::private::Number` stays an object; non-numeric
  variants are accepted.
- query via JSON Pointer (with `~0`/`~1` unescaping) and dot/bracket paths;
  queried booleans render as `1`/`0`, and queried containers are always
  recursively sorted with two-space indentation, matching the Swift baseline
  (`SofDevToolContractTests.swift:275-276`).
- `MAX_NESTING_DEPTH = 128`, counting containers only and checked before
  descending, so 128 nested arrays/objects (with scalar, empty or object leaves)
  are accepted and 129 are refused with a diagnostic instead of aborting the
  process; 50,000 nested arrays are refused too.
- `JsonSnapshot` carries the input once, inside `JsonRequest`.

`sofdevtool-core::session` (5 tests): an unchanged request is not rescheduled; a
new request clears the previous evaluation; an obsolete revision never settles;
invalid input settles as diagnostics; the evaluation epoch advances on submit and
settle.

## Findings from both reviews, addressed

First review:

- **P1** `serde_json` `arbitrary_precision` rewrote `$serde_json::private::Number`
  objects — replaced with a literal-preserving parser; regression tests/fixtures.
- **P2** queried booleans render `true -> "1"`, `false -> "0"`.
- **P3** removed the duplicated input from `JsonSnapshot`; the round-trip test
  asserts the serialized key set.
- **Standards** `rustfmt` passes and is part of the gate.

Second review:

- **Button focus** buttons accept a view-owned `FocusHandle` configured for Tab
  traversal (`.tab_stop(true).tab_index(0)` before `track_focus`), carry a focus
  border, and bind Enter/Space in the `SofDevToolButton` key context. Real
  traversal and visible focus are configured in code and still to be verified in
  the running app; the gate does not exercise them.
- **Encapsulation** `init`, `set_dark_theme`, `run` and `Root` now live in
  `sofdevtool-ui`; the application and gallery no longer depend on
  `gpui-component` or `gpui-platform`.
- **Clipboard boundary** an app-owned `Clipboard` trait is injected into the
  workspace; the production `GpuiClipboard` adapter is composed in `main`.
- **Debounce + revision gate** input changes clear the visible result, then a
  250 ms debounced evaluation settles through `JsonSession`, which rejects stale
  revisions; the logic is GPUI-independent and unit-tested.
- **Explicit edits** Paste and Clear use `replace_all`, which records undo
  history and emits a change event, so they recompute and keep undo coherent.
- **Query containers** always recursively sorted with two-space indentation,
  independent of Format options; fixture `query-root-sorted`.
- **README** wording corrected: macOS 26.2 is the build/test host; runtime is
  pending.
- **Identity** a minimal Debug bundle with a distinct `CFBundleIdentifier` is
  produced by a documented command.

Third review:

- **Tab traversal** button focus handles are created with
  `.tab_stop(true).tab_index(0)` before `track_focus`, which is what makes them
  Tab/Shift-Tab traversable; the actual traversal is still to be verified in the
  running app.
- **Visible focus on Primary** the focused border uses `text` on Primary (whose
  resting border is the accent) and `accent` on Secondary, so focus is visible on
  both.
- **Mount API** `sofdevtool-ui` exposes `mount`, which keeps the dependency's
  root as the true window root (so its input registry and focused-input routing
  still work) and returns an opaque `Entity<impl Render>`; the external `Root`
  type is neither re-exported nor named by app/gallery. A GUI-free contract test
  was not added: the `TestAppContext` window-view API takes a view, not a mounted
  `Entity`, so the contract is verified by construction and left to the native
  pass.
- **Bundle path** `scripts/bundle-debug` resolves the built executable from
  Cargo's JSON artifact messages instead of assuming `target/debug`, verified
  with both the default and an isolated `CARGO_TARGET_DIR`.
- **Nesting depth** counts containers only and checks the budget before
  descending; explicit 128/129 regressions cover scalar leaves, empty containers
  and nested objects.

## Native editing verification (requires a desktop session)

### Grapheme deletion follow-up

The native candidate exposed a regression before this follow-up: Backspace
removed the last scalar from both a family ZWJ emoji and `e` plus a combining
acute accent. The owner wrapper now captures only `Backspace` and `Delete` when
the deletion spans an extended grapheme (or expands a partial selection), then
calls the public GPUI input-handler API with an explicit UTF-16 range. The
explicit range keeps the original collapsed selection in the upstream undo
history. Normal scalar deletion, movement, selection, read-only fields, and
platform composition use GPUI's existing behavior.

`cargo test -p sofdevtool-ui editor::tests --lib` passed five range, routing-decision and UTF-16
conversion tests after the implementation. They were added with the fix, not as
a separate test-first red run; the native scalar-splitting observation is the
red evidence. The orchestrator subsequently verified corrected emoji and combining-accent deletion,
Undo followed by insertion at the original caret, partial-selection deletion,
read-only output, Query editing, and the gallery empty-state layout through
Computer Use. IME composition remains pending. Each intercepted deletion is an
individual undo operation; matching GPUI's multi-delete grouping is not claimed.

The earlier worker session had rejected launch/screenshot calls. The owner
explicitly authorized native verification in the resumed Codex task. Current
observations and remaining limitations are recorded in
[the native resume report](previous-migration/2026-09-21-native-resume.md).
This supersedes the prior authorization blocker, not the pending IME criterion.

Runnable artifacts (build/test host macOS 26.2, arm64):

- `rust/target/debug/sofdevtool`
- `rust/target/debug/examples/gallery`
- `rust/artifacts/SofDevToolRust.app` (from `rust/scripts/bundle-debug`)

Manual scenario for the JSON app:

1. Launch the app. Paste JSON with the Paste button; confirm the result formats
   with two-space indentation and that Paste is undoable with Cmd+Z.
2. Multiline selection: drag across lines and use Shift+Arrow; confirm selection
   and that Copy Result copies the whole result.
3. Undo/redo: type, then Cmd+Z and Cmd+Shift+Z; confirm text and cursor state.
4. Unicode/emoji: paste `{"text":"caffè 👩‍👩‍👧‍👦 🏳️‍🌈 𝄞"}`; confirm no mojibake and
   that emoji graphemes edit as one unit.
5. IME composition: with a composing input method enabled, type into the editor
   and confirm inline composition, commit, and cancel.
6. Scrolling: with a large document, scroll with trackpad and scrollbar; confirm
   the caret stays visible and the layout does not clip.
7. Clipboard: Paste inserts clipboard text; Copy Result shows the confirmation
   and the clipboard holds the exact result.
8. Focus: Tab/Shift-Tab move focus across the mode, indent, sort, Paste, Copy and
   Clear buttons; the focused button shows a border and activates on Enter/Space.

Also launch `gallery` and confirm the components render in their normal,
focused, disabled, invalid and empty states, independently of the application.

Any scenario that cannot be exercised is recorded as a limitation, not as a pass.

## Resumed final gate

`CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target scripts/verify --full`
passed on the Terra correction: formatting, Clippy with warnings denied, 35 tests
(23 JSON, 5 session, 5 UI-helper, 2 identity), Debug and Release builds. The only
notice was the existing transitive `block v0.1.6` future-incompatibility warning.
Full log is preserved in the [previous migration evidence](previous-migration/2026-09-21-terra-full-gate.log).

Integration byte comparison matched all 46 source/config/fixture files against
the tested candidate; only this evidence document differs. Running
`CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target rust/scripts/bundle-debug`
in the main checkout rebuilt all three first-party crates successfully and
produced `rust/artifacts/SofDevToolRust.app` with the distinct Rust identity.
IME and macOS 14/15 runtime are not claimed by these gates.
