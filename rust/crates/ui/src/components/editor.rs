use std::ops::Range;

use gpui::prelude::*;
use gpui::{
    div, App, Context, DefiniteLength, Entity, EntityInputHandler, Focusable, IntoElement,
    SharedString, Subscription, Window,
};
use gpui_component::input::{Backspace, Delete, InputEvent, Textarea, TextareaState};
use unicode_segmentation::UnicodeSegmentation as _;

/// A multiline text editor owned by this library.
///
/// The underlying editing engine (selection, undo/redo, IME composition,
/// scrolling) comes from `gpui-component`; callers depend only on this surface.
pub struct TextEditor {
    state: Entity<TextareaState>,
}

impl TextEditor {
    pub fn new(window: &mut Window, cx: &mut App) -> Self {
        Self {
            state: cx.new(|cx| TextareaState::new(window, cx)),
        }
    }

    /// Silently assigns text for initialization, restoration or a derived
    /// result. This emits no change notification and clears undo history.
    pub fn assign_text(&self, text: impl Into<SharedString>, window: &mut Window, cx: &mut App) {
        self.state
            .update(cx, |state, cx| state.set_value(text, window, cx));
    }

    /// Replaces the whole text as a user-style edit: recorded in the undo
    /// history and emitting a change event. Use this for explicit Paste/Clear.
    pub fn edit_text(&self, text: impl Into<SharedString>, window: &mut Window, cx: &mut App) {
        self.state
            .update(cx, |state, cx| state.replace_all(text, window, cx));
    }

    /// Compatibility alias for silent assignment.
    pub fn set_text(&self, text: impl Into<SharedString>, window: &mut Window, cx: &mut App) {
        self.assign_text(text, window, cx);
    }

    /// Compatibility alias for a user-style edit.
    pub fn replace_all(&self, text: impl Into<SharedString>, window: &mut Window, cx: &mut App) {
        self.edit_text(text, window, cx);
    }

    pub fn text(&self, cx: &App) -> String {
        self.state.read(cx).value().to_string()
    }

    /// Focuses the retained editor state; redraws do not replace that state.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.state.read(cx).focus_handle(cx).focus(window, cx);
    }

    /// Reports whether this retained editor owns keyboard focus.
    pub fn is_focused(&self, window: &Window, cx: &App) -> bool {
        self.state.read(cx).focus_handle(cx).is_focused(window)
    }

    /// Subscribe inside a view's context. The handler receives the view, window
    /// and context so it can recompute and notify without naming the engine type.
    pub fn on_change_in<T: 'static>(
        &self,
        window: &Window,
        cx: &mut Context<T>,
        mut handler: impl FnMut(&mut T, &mut Window, &mut Context<T>) + 'static,
    ) -> Subscription {
        cx.subscribe_in(
            &self.state,
            window,
            move |view, _entity, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    handler(view, window, cx);
                }
            },
        )
    }

    pub fn render(&self, readonly: bool, accessibility_id: &'static str) -> impl IntoElement {
        let state = self.state.clone();
        div()
            .w_full()
            .h(DefiniteLength::Fraction(1.0))
            .capture_action(move |_: &Backspace, window, cx| {
                if state.update(cx, |state, cx| delete_grapheme_before(state, window, cx)) {
                    cx.stop_propagation();
                }
            })
            .capture_action({
                let state = self.state.clone();
                move |_: &Delete, window, cx| {
                    if state.update(cx, |state, cx| delete_grapheme_after(state, window, cx)) {
                        cx.stop_propagation();
                    }
                }
            })
            .child(
                Textarea::new(&self.state)
                    .readonly(readonly)
                    .accessibility_id(accessibility_id)
                    .h(DefiniteLength::Fraction(1.0))
                    .w_full(),
            )
    }
}

fn delete_grapheme_before(
    state: &mut TextareaState,
    window: &mut Window,
    cx: &mut Context<TextareaState>,
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
    state: &mut TextareaState,
    window: &mut Window,
    cx: &mut Context<TextareaState>,
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
    state: &mut TextareaState,
    text: &str,
    selection: Range<usize>,
    range: Option<Range<usize>>,
    window: &mut Window,
    cx: &mut Context<TextareaState>,
) -> bool {
    let Some(range) = range else {
        return false;
    };
    if !requires_grapheme_replacement(text, &selection, &range) {
        return false;
    }

    EntityInputHandler::replace_text_in_range(
        state,
        Some(utf16_range(text, range)),
        "",
        window,
        cx,
    );
    true
}

fn requires_grapheme_replacement(
    text: &str,
    selection: &Range<usize>,
    range: &Range<usize>,
) -> bool {
    if selection.is_empty() {
        text[range.clone()].chars().count() > 1
    } else {
        range != selection
    }
}

fn utf16_range(text: &str, range: Range<usize>) -> Range<usize> {
    text[..range.start].encode_utf16().count()..text[..range.end].encode_utf16().count()
}

fn previous_grapheme_range(text: &str, cursor: usize) -> Option<Range<usize>> {
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

fn next_grapheme_range(text: &str, cursor: usize) -> Option<Range<usize>> {
    let boundaries = grapheme_ranges(text);
    boundaries
        .iter()
        .find(|range| range.start <= cursor && cursor < range.end)
        .cloned()
        .or_else(|| boundaries.into_iter().find(|range| range.start >= cursor))
}

fn expand_to_graphemes(text: &str, selection: Range<usize>) -> Option<Range<usize>> {
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

fn grapheme_ranges(text: &str) -> Vec<Range<usize>> {
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
