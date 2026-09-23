use gpui::{AnyElement, App, IntoElement, RenderOnce, SharedString, Window};

use super::{SelectAction, SelectableList, SelectableListFocus, SelectableRow};

/// Compatibility data shape for existing application History consumers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryItem {
    pub id: String,
    pub label: String,
    pub preview: String,
    pub available: bool,
}

/// Compatibility alias. New consumers can use [`SelectAction`].
pub type HistoryAction = SelectAction;

/// Compatibility adapter for workspaces not yet migrated to [`SelectableList`].
/// The quota and unavailable wording remain here only until ticket 19 moves
/// those callers to the generic API.
#[derive(IntoElement)]
pub struct HistoryPanel {
    items: Vec<HistoryItem>,
    selected: Option<String>,
    empty_message: SharedString,
    on_select: Option<HistoryAction>,
    actions: Option<AnyElement>,
}

impl HistoryPanel {
    pub fn new(
        items: Vec<HistoryItem>,
        selected: Option<String>,
        empty_message: impl Into<SharedString>,
    ) -> Self {
        Self {
            items,
            selected,
            empty_message: empty_message.into(),
            on_select: None,
            actions: None,
        }
    }

    pub fn on_select(mut self, handler: HistoryAction) -> Self {
        self.on_select = Some(handler);
        self
    }

    pub fn actions(mut self, actions: impl IntoElement) -> Self {
        self.actions = Some(actions.into_any_element());
        self
    }
}

impl RenderOnce for HistoryPanel {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let count = self.items.len();
        let rows = self
            .items
            .into_iter()
            .map(|item| SelectableRow {
                id: item.id,
                label: item.label,
                preview: item.preview,
                status: (!item.available).then(|| "Unavailable snapshot".to_owned()),
                selectable: true,
            })
            .collect();
        let mut list = SelectableList::new(
            "history.compatibility",
            "History",
            rows,
            self.selected,
            self.empty_message,
            SelectableListFocus::new(),
        )
        .summary(format!("{count}/25"));
        if let Some(action) = self.on_select {
            list = list.on_select(action);
        }
        if let Some(actions) = self.actions {
            list = list.actions(actions);
        }
        list
    }
}
