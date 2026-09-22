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
the editor. `sofui::set_dark_theme` and `sofui::set_active_theme` configure the
shared theme; a consumer redraws its views after a theme change.

Use `Button::with_id(id, label)` or `Button::primary_with_id(id, label)` with
a stable ID for each logical control. Labels may repeat or change while IDs
remain stable. Retain `FocusHandle`s in the owning view and pass them through
`focus_handle`; focused enabled buttons activate through Enter, Space, or a
pointer click. Disabled buttons invoke no handler. Text controls retain their
editing state across redraws; `render` takes an accessibility ID, and `focus`
focuses the underlying editor.

`assign_text` silently initializes or restores text: it emits no change event
and clears undo history. `edit_text` performs a user-style whole-text edit:
it emits a change event and is undoable. `on_change_in` observes editing
events. Consumers that silently assign text and need to evaluate it must
schedule that work explicitly. The legacy `set_text` and `replace_all` aliases
remain for workspaces still being migrated.

From `rust/`, run `cargo run -p sofui --example gallery` to open the gallery.
The gallery imports only sofui and GPUI, and demonstrates repeated labels,
focus, disabled actions, real Copy feedback, Unicode editors, diagnostics,
selection, confirmation and theme switching. `cargo test -p sofui` covers
public component interactions; `cargo check -p sofui --all-targets` includes
the gallery. Native launch observations require a separate acceptance run.

The application currently imports this package under the compatibility alias
`sofdevtool-ui`. Remaining workspaces and shell views will migrate to the
`sofui` crate name and explicit IDs/text method names in ticket 19; that
cleanup can then remove the alias and compatibility methods.
