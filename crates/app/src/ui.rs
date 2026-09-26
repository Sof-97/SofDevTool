//! App-owned presentation compositions over GPUI Kit controls.
//!
//! This module is **not** a component library. It contains no reusable
//! interaction logic: every interactive control here is a GPUI Kit control, and
//! every visual token is read from the kit's active [`ActiveTheme`]. The
//! functions only compose kit primitives and layout for the Developer Toolbox's
//! own surfaces, which is the "compose supported kit controls within the owning
//! application view" boundary the migration allows. No palette, focus or
//! pointer behavior is reimplemented here.
//!
//! The specialised Text Diff renderer is the only other retained exception and
//! lives in [`crate::text_diff`].

use std::rc::Rc;

use gpui::{
    div, App, AppContext as _, Context, Entity, EntityInputHandler as _, FontWeight,
    InteractiveElement as _, IntoElement, ParentElement as _, SharedString, Styled as _, Window,
};
use gpui_kit::component::{
    dialog::DialogButtonProps,
    list::{List, ListDelegate, ListItem, ListState},
    ActiveTheme as _, IndexPath, Sizable as _, Size, WindowExt as _,
};

/// A non-color-only diagnostic severity. The banner always carries a text
/// prefix, so the meaning never depends on color alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

/// A labelled region with a muted caption, used to frame editors and results.
pub fn panel(
    cx: &App,
    title: impl Into<SharedString>,
    caption: impl Into<SharedString>,
    content: impl IntoElement,
) -> impl IntoElement {
    let theme = cx.theme();
    let title = title.into();
    let caption = caption.into();
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .rounded(theme.radius)
        .border_1()
        .border_color(theme.border)
        .bg(theme.popover)
        .overflow_hidden()
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .px_3()
                .py_2()
                .border_b_1()
                .border_color(theme.border)
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .child(title),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(caption),
                ),
        )
        // This wrapper establishes the vertical flex context that gives editors
        // and the native WebView their remaining panel height.
        .child(div().flex().flex_col().flex_1().min_h_0().child(content))
}

/// A label stacked above a control, with an optional right-aligned hint.
pub fn labeled_field(
    cx: &App,
    label: impl Into<SharedString>,
    hint: Option<impl Into<SharedString>>,
    control: impl IntoElement,
) -> impl IntoElement {
    let theme = cx.theme();
    let mut header = div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .w_full()
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(label.into()),
        );
    if let Some(hint) = hint {
        header = header.child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(hint.into()),
        );
    }
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .gap_1()
        .child(header)
        .child(div().flex().flex_1().min_h_0().child(control))
}

/// A text diagnostic banner with a severity prefix and optional source location.
pub fn diagnostic_banner(
    cx: &App,
    severity: DiagnosticSeverity,
    message: &str,
    location: Option<(u32, u32)>,
) -> impl IntoElement {
    let theme = cx.theme();
    let (accent, prefix) = match severity {
        DiagnosticSeverity::Error => (theme.danger, "Error"),
        DiagnosticSeverity::Warning => (theme.warning, "Warning"),
    };
    let location = location
        .map(|(line, column)| format!("line {line}, column {column}"))
        .unwrap_or_default();

    div()
        .flex()
        .flex_row()
        .items_start()
        .gap_2()
        .w_full()
        .px_3()
        .py_2()
        .rounded(theme.radius)
        .border_1()
        .border_color(accent)
        .bg(theme.secondary)
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(accent)
                .child(format!("{prefix}:")),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_xs()
                .text_color(theme.foreground)
                .child(message.to_string()),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(location),
        )
}

/// A neutral placeholder shown when there is nothing to display.
pub fn empty_state(cx: &App, message: &str) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .items_center()
        .justify_center()
        .size_full()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(message.to_string())
}

/// An unobtrusive confirmation shown after a successful copy.
pub fn copy_feedback(cx: &App, visible: bool, message: &str) -> impl IntoElement {
    let theme = cx.theme();
    div().text_xs().text_color(theme.primary).child(if visible {
        message.to_string()
    } else {
        String::new()
    })
}

/// One row of a History list. `status` and `selectable` are presentation only;
/// the owning workspace assigns all product meaning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryRow {
    pub id: String,
    pub label: String,
    pub preview: String,
    pub status: Option<String>,
    pub selectable: bool,
}

type RowAction = Rc<dyn Fn(&str, &mut Window, &mut App)>;
type DialogAction = Rc<dyn Fn(&mut Window, &mut App)>;

/// The kit `List` delegate backing a History selection list. The delegate owns
/// only presentation and selection bookkeeping; the owning workspace owns the
/// selected id, its meaning and any response.
pub struct HistoryListDelegate {
    rows: Vec<HistoryRow>,
    selected: Option<String>,
    empty_message: SharedString,
    on_select: Option<RowAction>,
}

impl HistoryListDelegate {
    pub fn new(empty_message: impl Into<SharedString>) -> Self {
        Self {
            rows: Vec::new(),
            selected: None,
            empty_message: empty_message.into(),
            on_select: None,
        }
    }

    pub fn on_select(mut self, handler: RowAction) -> Self {
        self.on_select = Some(handler);
        self
    }

    pub fn rows(&self) -> &[HistoryRow] {
        &self.rows
    }

    pub fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }
}

impl ListDelegate for HistoryListDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.rows.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let theme = cx.theme().clone();
        let row = self.rows.get(ix.row)?.clone();
        let selected = self.selected.as_deref() == Some(row.id.as_str());
        let status = row.status.clone().unwrap_or_default();
        Some(
            ListItem::new(SharedString::from(format!("history.row.{}", row.id)))
                .selected(selected)
                .disabled(!row.selectable)
                .child(
                    div()
                        .debug_selector(move || format!("history.entry.{}", row.id))
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(row.label.clone()),
                                )
                                .child(div().text_xs().text_color(theme.warning).child(status)),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.foreground)
                                .child(row.preview.clone()),
                        ),
                ),
        )
    }

    fn render_empty(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> impl IntoElement {
        empty_state(cx, self.empty_message.as_ref())
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        let id = ix
            .and_then(|ix| self.rows.get(ix.row))
            .map(|row| row.id.clone());
        if self.selected != id {
            self.selected = id.clone();
            if let (Some(handler), Some(id)) = (self.on_select.clone(), id) {
                // The delegate already owns the list selection. The owner
                // updates its HistoryViewState only; reflecting it back into
                // this ListState here would recursively update the entity.
                handler(&id, _window, cx);
            }
            cx.notify();
        }
    }

    fn confirm(
        &mut self,
        _secondary: bool,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) {
        // Selection is reported through `set_selected_index`; confirmation is
        // deliberately inert so clicking a row only selects it. The owning view
        // restores through its explicit "Restore selected" action.
    }
}

/// Creates the retained `ListState` for a History panel.
pub fn history_state(
    window: &mut Window,
    cx: &mut App,
    empty_message: impl Into<SharedString>,
    on_select: RowAction,
) -> Entity<ListState<HistoryListDelegate>> {
    cx.new(|cx| {
        ListState::new(
            HistoryListDelegate::new(empty_message).on_select(on_select),
            window,
            cx,
        )
    })
}

/// Replaces the rendered rows without changing selection.
pub fn history_set_rows(
    state: &Entity<ListState<HistoryListDelegate>>,
    rows: Vec<HistoryRow>,
    cx: &mut App,
) {
    state.update(cx, |state, cx| {
        state.delegate_mut().rows = rows;
        cx.notify();
    });
}

/// Reflects the owning view's selected id into the delegate.
pub fn history_set_selected(
    state: &Entity<ListState<HistoryListDelegate>>,
    selected: Option<String>,
    cx: &mut App,
) {
    state.update(cx, |state, cx| {
        state.delegate_mut().selected = selected;
        cx.notify();
    });
}

/// A History panel: title, count summary, the kit `List` and view-owned actions.
pub fn history_panel(
    cx: &App,
    state: &Entity<ListState<HistoryListDelegate>>,
    title: impl Into<SharedString>,
    summary: impl Into<SharedString>,
    actions: impl IntoElement,
) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .gap_2()
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .child(title.into()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(summary.into()),
                ),
        )
        .child(
            div()
                .flex()
                .flex_1()
                .min_h_0()
                .child(List::new(state).with_size(Size::Small)),
        )
        .child(actions)
}

/// The focus handle GPUI Kit's [`gpui_kit::component::Button`] derives for the
/// given element id.
///
/// Kit buttons own their focus handle internally (keyed by their element id);
/// this returns that same handle so an owning view can move focus
/// programmatically, such as arrow-key navigation over a catalog of buttons.
/// It introduces no new focus behavior of its own.
pub fn button_focus(
    window: &mut Window,
    id: impl Into<gpui::ElementId>,
    cx: &mut App,
) -> gpui::FocusHandle {
    window
        .use_keyed_state(id, cx, |_, cx| cx.focus_handle())
        .read(cx)
        .clone()
}

/// Opens a standard kit confirmation dialog for a destructive or
/// state-replacing action.
///
/// The kit dialog owns focus, keyboard handling, backdrop and dismissal; the
/// caller owns the meaning of confirm and cancel. `on_ok` and `on_cancel` run
/// with the dialog's own window/context, so callers typically capture a
/// `WeakEntity` of the owning view and update it there.
#[allow(clippy::too_many_arguments)]
pub fn confirm_dialog(
    window: &mut Window,
    cx: &mut App,
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    ok_label: impl Into<SharedString>,
    cancel_label: impl Into<SharedString>,
    on_ok: impl Fn(&mut Window, &mut App) + 'static,
    on_cancel: impl Fn(&mut Window, &mut App) + 'static,
) {
    let title = title.into();
    let description = description.into();
    let ok_label = ok_label.into();
    let cancel_label = cancel_label.into();
    let on_ok: DialogAction = Rc::new(on_ok);
    let on_cancel: DialogAction = Rc::new(on_cancel);
    window.open_alert_dialog(cx, move |dialog, _window, _cx| {
        dialog
            .title(title.clone())
            .description(description.clone())
            .button_props(
                DialogButtonProps::default()
                    .ok_text(ok_label.clone())
                    .cancel_text(cancel_label.clone()),
            )
            .show_cancel(true)
            .on_ok({
                let on_ok = on_ok.clone();
                move |_event, window, cx| {
                    on_ok(window, cx);
                    true
                }
            })
            .on_cancel({
                let on_cancel = on_cancel.clone();
                move |_event, window, cx| {
                    on_cancel(window, cx);
                    true
                }
            })
    });
}

/// Renders a multiline GPUI Kit [`Textarea`] with extended-grapheme-safe
/// Backspace and Delete.
///
/// GPUI Kit's editing engine removes one Unicode scalar per Backspace/Delete,
/// so a ZWJ emoji family or a base-and-combining sequence would be split. The
/// Developer Toolbox retained whole-grapheme deletion before this migration;
/// this app-owned composition keeps that externally observable behavior by
/// intercepting the two actions around the kit control. It adds no new control:
/// rendering, selection, IME, undo/redo and scrolling all remain the kit's.
pub fn multiline_editor(
    state: &Entity<gpui_kit::component::input::TextareaState>,
    readonly: bool,
    accessibility_id: &'static str,
) -> impl IntoElement {
    use gpui::DefiniteLength;
    use gpui_kit::component::input::{
        Backspace as BackspaceAction, Delete as DeleteAction, Textarea,
    };

    let state_for_backspace = state.clone();
    let state_for_delete = state.clone();
    div()
        .w_full()
        .h(DefiniteLength::Fraction(1.0))
        .capture_action(move |_: &BackspaceAction, window, cx| {
            if state_for_backspace.update(cx, |state, cx| delete_grapheme_before(state, window, cx))
            {
                cx.stop_propagation();
            }
        })
        .capture_action(move |_: &DeleteAction, window, cx| {
            if state_for_delete.update(cx, |state, cx| delete_grapheme_after(state, window, cx)) {
                cx.stop_propagation();
            }
        })
        .child(
            Textarea::new(state)
                .readonly(readonly)
                .accessibility_id(accessibility_id)
                .h(DefiniteLength::Fraction(1.0))
                .w_full(),
        )
}

fn delete_grapheme_before(
    state: &mut gpui_kit::component::input::TextareaState,
    window: &mut Window,
    cx: &mut Context<gpui_kit::component::input::TextareaState>,
) -> bool {
    if !state.is_editable() {
        return false;
    }
    let selection = state.selected_range();
    let text = state.value().to_string();
    let range = if selection.is_empty() {
        previous_grapheme_range(&text, state.cursor())
    } else {
        expand_to_graphemes(&text, selection.clone())
    };
    replace_extended_grapheme(state, &text, selection, range, window, cx)
}

fn delete_grapheme_after(
    state: &mut gpui_kit::component::input::TextareaState,
    window: &mut Window,
    cx: &mut Context<gpui_kit::component::input::TextareaState>,
) -> bool {
    if !state.is_editable() {
        return false;
    }
    let selection = state.selected_range();
    let text = state.value().to_string();
    let range = if selection.is_empty() {
        next_grapheme_range(&text, state.cursor())
    } else {
        expand_to_graphemes(&text, selection.clone())
    };
    replace_extended_grapheme(state, &text, selection, range, window, cx)
}

fn replace_extended_grapheme(
    state: &mut gpui_kit::component::input::TextareaState,
    text: &str,
    selection: std::ops::Range<usize>,
    range: Option<std::ops::Range<usize>>,
    window: &mut Window,
    cx: &mut Context<gpui_kit::component::input::TextareaState>,
) -> bool {
    let Some(range) = range else {
        return false;
    };
    if !requires_grapheme_replacement(text, &selection, &range) {
        return false;
    }
    state.replace_text_in_range(Some(utf16_range(text, range)), "", window, cx);
    true
}

fn requires_grapheme_replacement(
    text: &str,
    selection: &std::ops::Range<usize>,
    range: &std::ops::Range<usize>,
) -> bool {
    if selection.is_empty() {
        text[range.clone()].chars().count() > 1
    } else {
        range != selection
    }
}

fn utf16_range(text: &str, range: std::ops::Range<usize>) -> std::ops::Range<usize> {
    text[..range.start].encode_utf16().count()..text[..range.end].encode_utf16().count()
}

fn previous_grapheme_range(text: &str, cursor: usize) -> Option<std::ops::Range<usize>> {
    let boundaries = grapheme_ranges(text);
    boundaries
        .iter()
        .find(|range| range.start < cursor && cursor <= range.end)
        .cloned()
        .or_else(|| {
            boundaries
                .into_iter()
                .rev()
                .find(|range| range.end <= cursor)
        })
}

fn next_grapheme_range(text: &str, cursor: usize) -> Option<std::ops::Range<usize>> {
    let boundaries = grapheme_ranges(text);
    boundaries
        .iter()
        .find(|range| range.start <= cursor && cursor < range.end)
        .cloned()
        .or_else(|| boundaries.into_iter().find(|range| range.start >= cursor))
}

fn expand_to_graphemes(
    text: &str,
    selection: std::ops::Range<usize>,
) -> Option<std::ops::Range<usize>> {
    let ranges = grapheme_ranges(text);
    let start = ranges
        .iter()
        .find(|range| range.end > selection.start)?
        .start;
    let end = ranges
        .iter()
        .rev()
        .find(|range| range.start < selection.end)?
        .end;
    Some(start..end)
}

fn grapheme_ranges(text: &str) -> Vec<std::ops::Range<usize>> {
    use unicode_segmentation::UnicodeSegmentation as _;
    text.grapheme_indices(true)
        .map(|(start, grapheme)| start..start + grapheme.len())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAMILY: &str = "👨‍👩‍👧‍👦";

    #[test]
    fn deleting_before_a_cursor_removes_the_whole_zwj_grapheme() {
        let text = format!("{{\"text\":\"{FAMILY}\"}}");
        let cursor = text.len() - 2;

        let range = previous_grapheme_range(&text, cursor).expect("family is deletable");

        assert_eq!(&text[range], FAMILY);
    }

    #[test]
    fn deleting_after_a_cursor_removes_the_whole_combining_grapheme() {
        let text = "Cafe\u{301}!";
        let cursor = "Caf".len();

        let range = next_grapheme_range(text, cursor).expect("accented e is deletable");

        assert_eq!(&text[range], "e\u{301}");
    }

    #[test]
    fn partial_selection_expands_to_the_whole_emoji_before_deletion() {
        let text = format!("a{FAMILY}z");
        let after_family = 1 + FAMILY.len();

        assert_eq!(
            expand_to_graphemes(&text, 2..after_family - 1),
            Some(1..after_family)
        );
    }

    #[test]
    fn explicit_range_uses_utf16_offsets_for_a_family_emoji() {
        let text = format!("{{\"text\":\"{FAMILY}\"}}");
        let start = "{\"text\":\"".len();

        assert_eq!(utf16_range(&text, start..start + FAMILY.len()), 9..20);
    }

    #[test]
    fn only_extended_or_partial_graphemes_intercept_native_deletion() {
        assert!(!requires_grapheme_replacement("ab", &(1..1), &(0..1)));
        assert!(requires_grapheme_replacement("e\u{301}", &(3..3), &(0..3)));
        assert!(!requires_grapheme_replacement("ab", &(0..2), &(0..2)));
        assert!(requires_grapheme_replacement("e\u{301}", &(1..2), &(0..3)));
    }
}
