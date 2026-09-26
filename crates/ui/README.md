# sofui 0.1.0

`sofui` is an independently versioned GPUI component library. It remains in
this repository until another real application validates a separate release.
It contains reusable presentation and component interaction logic. Consumers
own process startup, Utility or other domain behavior, persistence, Clipboard
policy, and application lifecycle.

The library uses pinned `gpui-pre` 0.3.6 and `gpui-component` 0.6.6. The latter
supplies the complex text-editing engine behind sofui's `TextField` and
`TextEditor` interfaces; applications do not need its input types. These two
dependencies and `gpui-pre-platform` are Apache-2.0. The platform crate is
used only by the gallery example; a consumer supplies its own GPUI entrypoint.

Initialize components with `sofui::init(cx)` after the consumer's GPUI app
starts. Make an application view, then pass it to `sofui::mount(view, window,
cx)` as the window root. This mount supplies the input registry required by
the editor. Call `sofui::apply_theme(ThemeVariant::Graphite, cx)` for the fresh
default, or choose `CatppuccinFrappe`. `apply_custom_theme` accepts
`ThemeTokens::from_palette(ThemePalette { ... })` with `0xRRGGBB` semantic
colors. Both calls update every open window and the wrapped editor's background,
text, caret and focus colors without recreating editor or view entities. A
window opened later inherits the same palette. The consuming application owns
theme-choice persistence.

Use `Button::with_id(id, label)` or `Button::primary_with_id(id, label)` with
a stable ID for each logical control. Labels may repeat or change while IDs
remain stable. Retain `FocusHandle`s in the owning view and pass them through
`focus_handle`; focused enabled buttons activate through Enter, Space, or a
pointer click. Disabled buttons invoke no handler. Text controls retain their
editing state across redraws; `render` takes an accessibility ID, and `focus`
focuses the underlying editor.

`NumericStepper::new(id, label, value, min, max, step)` renders a bounded,
focusable pair of buttons and a numeric value. Pass `None` to disable it. Its
`on_step` callback receives a signed delta, so apply it to the latest value in
the owning view with `NumericStepper::stepped`, then notify the view. The
component has no color or parsing policy; Color Conversion uses it for sRGB
channels. The gallery shows keyboard activation and boundary clamping.

`SegmentedControl::new(id, label, options, selected_id, focus)` presents an
arbitrary set of caller-labelled choices, each with a stable option ID and an
optional disabled state. Retain `SegmentedControlFocus` in the owning view.
Click or Enter/Space activates a focused option; Left/Right wraps over enabled
choices and Home/End selects the first/last enabled choice. The `on_change`
callback fires only when the selected ID changes; the consumer owns that value
and its domain meaning. Rows wrap when the available width is narrow.

`SelectableList::new(id, title, rows, selected, empty_message, focus)` accepts
plain `SelectableRow`s. Supply the count or quota wording with `summary` and
retain the required `SelectableListFocus` in the owning view across redraws.
Click, Enter and Space select a row with a stable ID and retained
focus. Rows can show a caller-defined status or be disabled. The consumer owns
the selected ID and any response to selection.

Retain one `HoldController` per destructive action and pass it to
`HoldButton::new(id, label, controller)` with the caller's `duration`. The
library runs the timer, paints progress and
cancels on early pointer release, pointer exit or Escape. A completed pointer
hold invokes `on_complete` once. Enter or Space invokes `on_keyboard` instead;
the consumer can show `ConfirmationBar` with its own message, labels, focus
handles and confirm/cancel actions. The app decides what is destructive and
performs persistence; sofui only manages the interaction.

`assign_text` silently initializes or restores text: it emits no change event
and clears undo history. `edit_text` performs a user-style whole-text edit:
it emits a change event and is undoable. `on_change_in` observes editing
events. Consumers that silently assign text and need to evaluate it must
schedule that work explicitly.

From the repository root, run `cargo run -p sofui --example gallery` to open the gallery.
The gallery imports only sofui and GPUI, and demonstrates repeated labels,
focus, disabled actions, real Copy feedback, Unicode editors, diagnostics,
selected, unavailable, disabled and failed list rows, an empty list, hold
cancellation, confirmation, both presets, a high-contrast custom palette and
the numeric control. `cargo test -p sofui` covers public component interactions;
`cargo check -p sofui --all-targets` includes the gallery. Native launch
observations require a separate acceptance run.

`crates/ui/tests/consumer` is an independent Cargo package with its own
`[workspace]` boundary. From the repository root, run
`cargo build --offline --locked --manifest-path crates/ui/tests/consumer/Cargo.toml` to compile a
second GPUI application using only sofui's public controls and declared
dependencies. Run it with
`cargo run --manifest-path crates/ui/tests/consumer/Cargo.toml` for a standalone editor, choice control
and stable-ID button. It imports no SofDevTool application or core crate.
`bash scripts/rust/verify-sofui-isolation.sh` copies only the library and
consumer sources to a temporary directory outside this repository, then builds
both the gallery and consumer against their dedicated lockfiles. This is part
of `make verify-full`. The copied library requires neither the repository root
manifest nor any application/core source or asset path.
