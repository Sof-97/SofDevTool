//! Workbench-owned History presentation state. Recording remains in HistoryRecorder.

use std::collections::HashMap;

use gpui::{Context, Pixels};

use crate::preferences::HistoryLayoutPreferences;
use crate::registry::UtilityId;

const INLINE_HISTORY_MIN_WIDTH: f32 = 850.;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryPlacement {
    Hidden,
    Inline,
    Sheet,
}

pub struct WorkspaceLayout {
    active: UtilityId,
    workspace_width: Pixels,
    visible: HashMap<UtilityId, bool>,
    preferences: Option<HistoryLayoutPreferences>,
    save_error: Option<String>,
}

impl WorkspaceLayout {
    pub fn load_from_application_support() -> Self {
        match HistoryLayoutPreferences::application_support() {
            Ok(preferences) => Self::load(Some(preferences)),
            Err(error) => {
                let mut layout = Self::load(None);
                layout.save_error = Some(format!(
                    "History display preference is unavailable for this session: {error}"
                ));
                layout
            }
        }
    }

    pub fn load(preferences: Option<HistoryLayoutPreferences>) -> Self {
        let visible = preferences
            .as_ref()
            .map(HistoryLayoutPreferences::load)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(slug, visible)| UtilityId::from_slug(&slug).map(|id| (id, visible)))
            .collect();
        Self {
            active: UtilityId::Json,
            workspace_width: Pixels::from(1000.),
            visible,
            preferences,
            save_error: None,
        }
    }

    pub fn history_visible(&self, utility: UtilityId) -> bool {
        self.visible.get(&utility).copied().unwrap_or(false)
    }

    pub fn placement(&self, utility: UtilityId) -> HistoryPlacement {
        if utility != self.active || !self.history_visible(utility) {
            HistoryPlacement::Hidden
        } else if f32::from(self.workspace_width) >= INLINE_HISTORY_MIN_WIDTH {
            HistoryPlacement::Inline
        } else {
            HistoryPlacement::Sheet
        }
    }

    pub fn set_history_visible(
        &mut self,
        utility: UtilityId,
        visible: bool,
        cx: &mut Context<Self>,
    ) {
        if self.history_visible(utility) == visible {
            return;
        }
        self.visible.insert(utility, visible);
        if let Some(preferences) = &self.preferences {
            let stored = self
                .visible
                .iter()
                .filter_map(|(id, visible)| visible.then_some((id.slug().to_owned(), true)))
                .collect();
            self.save_error = preferences
                .save(&stored)
                .err()
                .map(|error| format!("History layout preference could not be saved: {error}"));
        }
        cx.notify();
    }

    pub fn set_active(&mut self, utility: UtilityId, cx: &mut Context<Self>) {
        if self.active != utility {
            self.active = utility;
            cx.notify();
        }
    }

    pub fn set_workspace_width(&mut self, width: Pixels, cx: &mut Context<Self>) -> bool {
        let was_inline = f32::from(self.workspace_width) >= INLINE_HISTORY_MIN_WIDTH;
        let is_inline = f32::from(width) >= INLINE_HISTORY_MIN_WIDTH;
        self.workspace_width = width;
        if was_inline != is_inline {
            cx.notify();
        }
        was_inline != is_inline
    }

    pub fn save_error(&self) -> Option<&str> {
        self.save_error.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::AppContext as _;
    use std::fs;

    fn temporary_root() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "sofdevtool-history-layout-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[gpui::test]
    fn history_starts_closed_and_placement_follows_active_width(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            let layout = cx.new(|_| WorkspaceLayout::load(None));
            assert_eq!(
                layout.read(cx).placement(UtilityId::Json),
                HistoryPlacement::Hidden
            );
            layout.update(cx, |layout, cx| {
                layout.set_history_visible(UtilityId::Json, true, cx)
            });
            assert_eq!(
                layout.read(cx).placement(UtilityId::Json),
                HistoryPlacement::Inline
            );
            layout.update(cx, |layout, cx| {
                layout.set_workspace_width(Pixels::from(500.), cx)
            });
            assert_eq!(
                layout.read(cx).placement(UtilityId::Json),
                HistoryPlacement::Sheet
            );
            layout.update(cx, |layout, cx| layout.set_active(UtilityId::TextDiff, cx));
            assert_eq!(
                layout.read(cx).placement(UtilityId::Json),
                HistoryPlacement::Hidden
            );
        });
    }

    #[gpui::test]
    fn malformed_or_unknown_layout_preferences_start_closed(cx: &mut gpui::TestAppContext) {
        let root = temporary_root();
        fs::create_dir_all(&root).unwrap();
        let path = root.join("history-layout.v1.json");
        fs::write(&path, b"not json").unwrap();
        let preferences = HistoryLayoutPreferences::new(root.clone());
        cx.update(|cx| {
            let layout = cx.new(|_| WorkspaceLayout::load(Some(preferences.clone())));
            assert!(!layout.read(cx).history_visible(UtilityId::Json));
            assert_eq!(
                layout.read(cx).placement(UtilityId::Json),
                HistoryPlacement::Hidden
            );
        });
        fs::write(&path, br#"{"version":1,"visible":{"not-a-utility":true}}"#).unwrap();
        cx.update(|cx| {
            let layout = cx.new(|_| WorkspaceLayout::load(Some(preferences.clone())));
            assert!(!layout.read(cx).history_visible(UtilityId::Json));
            layout.update(cx, |layout, cx| {
                layout.set_history_visible(UtilityId::TextDiff, true, cx)
            });
        });
        let restored = WorkspaceLayout::load(Some(preferences));
        assert!(!restored.history_visible(UtilityId::Json));
        assert!(restored.history_visible(UtilityId::TextDiff));
        fs::remove_dir_all(root).unwrap();
    }

    #[gpui::test]
    fn visibility_round_trips_per_utility_without_affecting_recording(
        cx: &mut gpui::TestAppContext,
    ) {
        let root = temporary_root();
        let preferences = HistoryLayoutPreferences::new(root.clone());
        let recorder = crate::history::HistoryRecorder::new(
            crate::history::HistoryStore::new(root.clone()),
            Box::new(crate::history::SystemClock::new()),
        );
        cx.update(|cx| {
            let layout = cx.new(|_| WorkspaceLayout::load(Some(preferences.clone())));
            layout.update(cx, |layout, cx| {
                layout.set_history_visible(UtilityId::Json, true, cx);
                layout.set_history_visible(UtilityId::Json, true, cx);
            });
            assert!(layout.read(cx).history_visible(UtilityId::Json));
            assert!(!layout.read(cx).history_visible(UtilityId::TextDiff));
            assert!(layout.read(cx).save_error().is_none());
        });
        let restored = WorkspaceLayout::load(Some(preferences));
        assert!(restored.history_visible(UtilityId::Json));
        assert!(!restored.history_visible(UtilityId::TextDiff));
        assert!(recorder.is_recording(UtilityId::Json.slug()));
        assert!(recorder.load(UtilityId::Json.slug()).unwrap().is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[gpui::test]
    fn failed_visibility_save_reports_a_non_payload_error(cx: &mut gpui::TestAppContext) {
        let root = temporary_root();
        fs::write(&root, b"not a directory").unwrap();
        cx.update(|cx| {
            let layout = cx
                .new(|_| WorkspaceLayout::load(Some(HistoryLayoutPreferences::new(root.clone()))));
            layout.update(cx, |layout, cx| {
                layout.set_history_visible(UtilityId::Json, true, cx)
            });
            let layout = layout.read(cx);
            assert!(layout.history_visible(UtilityId::Json));
            let error = layout.save_error().unwrap();
            assert!(error.starts_with("History layout preference could not be saved:"));
            assert!(!error.contains(UtilityId::Json.slug()));
        });
        fs::remove_file(root).unwrap();
    }
}
