//! Separate Settings surface for Launcher shortcut and History management.

use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    div, px, size, App, Context, FocusHandle, IntoElement, KeyDownEvent, Render, Window,
    WindowBounds, WindowKind, WindowOptions,
};
use sofdevtool_ui::{
    diagnostic_banner, mount, view_click, Button, ConfirmationBar, DiagnosticSeverity, HoldButton,
    HoldController, ThemeTokens,
};

use crate::history::{HistoryRecorder, HistorySubscription};
use crate::preferences::ShortcutPreferences;
use crate::shortcut::{Shortcut, COMMAND, CONTROL, OPTION, SHIFT};
use crate::workbench::Workbench;

/// A Utility row shown in the History section.
#[derive(Clone)]
pub struct UtilityRow {
    pub id: String,
    pub name: String,
    pub default_enabled: bool,
}

/// A destructive History action that requires a deliberate pointer hold.
#[derive(Clone, PartialEq, Eq)]
enum HoldTarget {
    ClearUtility(String),
    ClearAll,
}

impl HoldTarget {
    fn duration(&self) -> Duration {
        match self {
            HoldTarget::ClearUtility(_) => Duration::from_secs(1),
            HoldTarget::ClearAll => Duration::from_secs(2),
        }
    }
}

fn clear_history(history: &HistoryRecorder, target: &HoldTarget, cx: &mut App) -> String {
    match target {
        HoldTarget::ClearUtility(id) => match history.clear_utility(id, cx) {
            Ok(()) => "Retained entries cleared.".into(),
            Err(error) => format!("History could not be cleared: {error}"),
        },
        HoldTarget::ClearAll => match history.clear_all(cx) {
            Ok(report) if report.failed_ids.is_empty() => "All Rust History cleared.".into(),
            Ok(report) => format!(
                "Cleared {} History file(s); {} could not be cleared.",
                report.cleared_ids.len(),
                report.failed_ids.len()
            ),
            Err(error) => format!("History could not be cleared: {error}"),
        },
    }
}

fn retry_history(history: &HistoryRecorder, utility_id: &str, cx: &mut App) -> String {
    match history.retry_recording(utility_id, cx) {
        Ok(_) => "History recording resumed without adding an entry.".into(),
        Err(error) => format!("History recording is still paused: {error}"),
    }
}

/// Opens a focused, separate Settings window. The Workbench keeps ownership of
/// the operating-system registration and the History repository.
pub fn show(
    workbench: gpui::WeakEntity<Workbench>,
    preferences: ShortcutPreferences,
    history: Rc<HistoryRecorder>,
    utilities: Vec<UtilityRow>,
    current_shortcut: Shortcut,
    cx: &mut gpui::App,
) {
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::centered(size(px(700.), px(720.)), cx)),
        focus: true,
        kind: WindowKind::Normal,
        is_resizable: true,
        titlebar: Some(gpui::TitlebarOptions {
            title: Some("Developer Toolbox Settings".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let _ = cx.open_window(options, move |window, cx| {
        let view = cx.new(|cx| {
            SettingsView::new(
                window,
                cx,
                Some(workbench),
                preferences,
                history,
                utilities,
                current_shortcut,
            )
        });
        mount(view, window, cx)
    });
}

struct UtilityFocus {
    record: FocusHandle,
    clear: FocusHandle,
    retry: FocusHandle,
}

pub struct SettingsView {
    workbench: Option<gpui::WeakEntity<Workbench>>,
    preferences: ShortcutPreferences,
    history: Rc<HistoryRecorder>,
    utilities: Vec<UtilityRow>,
    active_shortcut: Shortcut,
    capturing: bool,
    diagnostic: Option<String>,
    notice: Option<String>,
    utility_holds: Vec<HoldController>,
    unknown_holds: BTreeMap<String, HoldController>,
    unknown_focus: BTreeMap<String, FocusHandle>,
    clear_all_hold: HoldController,
    pending_confirm: Option<HoldTarget>,
    capture_focus: FocusHandle,
    capture_button_focus: FocusHandle,
    global_record_focus: FocusHandle,
    clear_all_focus: FocusHandle,
    confirm_focus: FocusHandle,
    cancel_confirm_focus: FocusHandle,
    utility_focus: Vec<UtilityFocus>,
    _history_subscription: HistorySubscription,
}

impl SettingsView {
    fn new(
        _window: &mut Window,
        cx: &mut Context<Self>,
        workbench: Option<gpui::WeakEntity<Workbench>>,
        preferences: ShortcutPreferences,
        history: Rc<HistoryRecorder>,
        utilities: Vec<UtilityRow>,
        active_shortcut: Shortcut,
    ) -> Self {
        let utility_focus: Vec<UtilityFocus> = utilities
            .iter()
            .map(|_| UtilityFocus {
                record: cx.focus_handle().tab_stop(true).tab_index(0),
                clear: cx.focus_handle().tab_stop(true).tab_index(0),
                retry: cx.focus_handle().tab_stop(true).tab_index(0),
            })
            .collect();
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe_status(move |cx| {
            weak.update(cx, |_, cx| cx.notify()).ok();
        });
        Self {
            workbench,
            preferences,
            history,
            utilities,
            active_shortcut,
            capturing: false,
            diagnostic: None,
            notice: None,
            utility_holds: (0..utility_focus.len())
                .map(|_| HoldController::new())
                .collect(),
            unknown_holds: BTreeMap::new(),
            unknown_focus: BTreeMap::new(),
            clear_all_hold: HoldController::new(),
            pending_confirm: None,
            capture_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            capture_button_focus: cx.focus_handle().tab_stop(true).tab_index(1),
            global_record_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            clear_all_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            confirm_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            cancel_confirm_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            utility_focus,
            _history_subscription: history_subscription,
        }
    }

    fn begin_capture(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.capturing = true;
        self.diagnostic = None;
        self.capture_focus.focus(window, cx);
        cx.notify();
    }

    fn receive_key(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.capturing {
            return;
        }
        match ShortcutCapture::record(
            native_key_code(),
            &event.keystroke.key,
            carbon_modifiers(&event.keystroke.modifiers),
        ) {
            CaptureResult::IgnoredModifier => {}
            CaptureResult::Cancelled => {
                self.capturing = false;
                self.diagnostic = None;
                cx.notify();
            }
            CaptureResult::Invalid(diagnostic) => {
                self.diagnostic = Some(diagnostic);
                cx.notify();
            }
            CaptureResult::Captured(candidate) => self.apply(candidate, cx),
        }
        cx.stop_propagation();
    }

    fn apply(&mut self, candidate: Shortcut, cx: &mut Context<Self>) {
        if candidate.is_same_chord(&self.active_shortcut) {
            self.capturing = false;
            self.diagnostic = None;
            cx.notify();
            return;
        }
        let Some(workbench) = self.workbench.as_ref() else {
            self.capturing = false;
            self.diagnostic =
                Some("The Workbench is no longer available; the shortcut was not changed.".into());
            cx.notify();
            return;
        };
        let Some(current) = workbench
            .update(cx, |workbench, cx| {
                workbench.configure_shortcut(candidate.clone(), cx);
                workbench.current_shortcut()
            })
            .ok()
        else {
            self.capturing = false;
            self.diagnostic =
                Some("The Workbench is no longer available; the shortcut was not changed.".into());
            cx.notify();
            return;
        };
        if !current.is_same_chord(&candidate) {
            self.capturing = false;
            self.diagnostic = Some(
                "That shortcut could not be registered. The previous shortcut remains active."
                    .into(),
            );
            cx.notify();
            return;
        }
        match self.preferences.save(&current) {
            Ok(()) => {
                self.active_shortcut = current;
                self.diagnostic = None;
            }
            Err(error) => {
                self.active_shortcut = current;
                self.diagnostic = Some(format!(
                    "Shortcut is active for this launch but could not be saved: {error}"
                ));
            }
        }
        self.capturing = false;
        cx.notify();
    }

    fn clear_button(
        &self,
        id: String,
        label: &'static str,
        target: HoldTarget,
        controller: HoldController,
        focus: Option<FocusHandle>,
        cx: &mut Context<Self>,
    ) -> HoldButton {
        let weak = cx.weak_entity();
        let complete_target = target.clone();
        let mut button = HoldButton::new(gpui::ElementId::Name(id.into()), label, controller)
            .duration(target.duration())
            .on_complete(move |cx| {
                weak.update(cx, |this, cx| this.perform(complete_target.clone(), cx))
                    .ok();
            })
            .on_keyboard(view_click(cx, move |this, _window, cx| {
                this.request_confirm(target.clone(), cx);
            }));
        if let Some(focus) = focus {
            button = button.focus_handle(focus);
        }
        button
    }

    fn request_confirm(&mut self, target: HoldTarget, cx: &mut Context<Self>) {
        self.pending_confirm = Some(target);
        cx.notify();
    }

    fn confirm(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.pending_confirm.take() {
            self.perform(target, cx);
        }
    }

    fn cancel_confirm(&mut self, cx: &mut Context<Self>) {
        self.pending_confirm = None;
        cx.notify();
    }

    fn perform(&mut self, target: HoldTarget, cx: &mut Context<Self>) {
        self.notice = Some(clear_history(&self.history, &target, cx));
        cx.notify();
    }

    fn retry_recording(&mut self, utility_id: &str, cx: &mut Context<Self>) {
        self.notice = Some(retry_history(&self.history, utility_id, cx));
        cx.notify();
    }

    fn stored_ids(&self) -> Vec<String> {
        self.history
            .store()
            .stored_utility_ids()
            .unwrap_or_default()
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tokens = ThemeTokens::active();
        let capture_text = if self.capturing {
            "Press a shortcut now (Escape cancels)"
        } else {
            &self.active_shortcut.display_name
        };

        let policy = self.history.policy();
        let mut utilities = div().flex().flex_col().gap_2().w_full();
        for (index, row) in self.utilities.clone().iter().enumerate() {
            let recording = policy
                .per_utility
                .get(&row.id)
                .copied()
                .unwrap_or(row.default_enabled);
            let paused = self.history.store().is_paused(&row.id);
            let id = row.id.clone();
            let focus = &self.utility_focus[index];
            utilities = utilities.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .w_full()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .border_1()
                    .border_color(tokens.border())
                    .child(div().flex_1().text_sm().child(row.name.clone()))
                    .child(
                        Button::new(if recording {
                            "Recording: on"
                        } else {
                            "Recording: off"
                        })
                        .id(gpui::ElementId::Name(
                            format!("history-record-{}", row.id).into(),
                        ))
                        .variant(if recording {
                            sofdevtool_ui::ButtonVariant::Primary
                        } else {
                            sofdevtool_ui::ButtonVariant::Secondary
                        })
                        .focus_handle(focus.record.clone())
                        .on_click(view_click(
                            cx,
                            move |this, _window, cx| {
                                this.history.set_utility_enabled(&id, !recording);
                                cx.notify();
                            },
                        )),
                    )
                    .child(self.clear_button(
                        format!("history-clear-{}", row.id),
                        "Clear",
                        HoldTarget::ClearUtility(row.id.clone()),
                        self.utility_holds[index].clone(),
                        Some(focus.clear.clone()),
                        cx,
                    ))
                    .when(paused, |this| {
                        let retry_id = row.id.clone();
                        this.child(div().text_xs().text_color(tokens.warning()).child("paused"))
                            .child(
                                Button::new("Retry")
                                    .id(gpui::ElementId::Name(
                                        format!("history-retry-{}", row.id).into(),
                                    ))
                                    .focus_handle(focus.retry.clone())
                                    .on_click(view_click(cx, move |this, _window, cx| {
                                        this.retry_recording(&retry_id, cx);
                                    })),
                            )
                    }),
            );
        }

        let unknown: Vec<String> = self
            .stored_ids()
            .into_iter()
            .filter(|id| !self.utilities.iter().any(|row| &row.id == id))
            .collect();
        let mut unknown_list = div().flex().flex_col().gap_2().w_full();
        for id in &unknown {
            let controller = self.unknown_holds.entry(id.clone()).or_default().clone();
            let focus = self
                .unknown_focus
                .entry(id.clone())
                .or_insert_with(|| cx.focus_handle().tab_stop(true).tab_index(0))
                .clone();
            unknown_list = unknown_list.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(tokens.text_muted())
                            .child(format!("Unknown Utility file: {id}")),
                    )
                    .child(self.clear_button(
                        format!("history-clear-unknown-{id}"),
                        "Clear",
                        HoldTarget::ClearUtility(id.clone()),
                        controller,
                        Some(focus),
                        cx,
                    )),
            );
        }

        div()
            .id("settings-scroll")
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .bg(tokens.background())
            .text_color(tokens.text())
            .text_size(px(13.))
            .overflow_y_scroll()
            .track_focus(&self.capture_focus)
            .capture_key_down(
                cx.listener(|this, event, window, cx| this.receive_key(event, window, cx)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Settings"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(tokens.text_muted())
                            .child("LOCAL · THIS MAC"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(tokens.border())
                    .bg(tokens.surface())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("Launcher shortcut"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(tokens.accent())
                                    .child(capture_text.to_string()),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(tokens.text_muted())
                                    .child("Press Command, Control, or Option with a key. Escape cancels capture."),
                            )
                            .child(
                                Button::with_id("settings.capture-shortcut", "Capture shortcut")
                                    .focus_handle(self.capture_button_focus.clone())
                                    .on_click(view_click(cx, |this, window, cx| {
                                        this.begin_capture(window, cx)
                                    })),
                            ),
                    )
                    .when_some(self.diagnostic.as_ref(), |this, diagnostic| {
                        this.child(diagnostic_banner(
                            DiagnosticSeverity::Error,
                            diagnostic,
                            None,
                        ))
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(tokens.border())
                    .bg(tokens.surface())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child("History"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(tokens.text_muted())
                                            .child("Recording choices do not erase retained entries."),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        Button::with_id(
                                            "history-global-record",
                                            if policy.global_enabled {
                                                "Recording: on"
                                            } else {
                                                "Recording: off"
                                            },
                                        )
                                        .variant(if policy.global_enabled {
                                            sofdevtool_ui::ButtonVariant::Primary
                                        } else {
                                            sofdevtool_ui::ButtonVariant::Secondary
                                        })
                                        .focus_handle(self.global_record_focus.clone())
                                        .on_click(view_click(cx, |this, _window, cx| {
                                            let next = !this.history.policy().global_enabled;
                                            this.history.set_global_enabled(next);
                                            cx.notify();
                                        })),
                                    )
                                    .child(
                                        div().debug_selector(|| "settings-clear-all".into()).child(
                                            self.clear_button(
                                                "history-clear-all".to_owned(),
                                                "Clear All · hold 2s",
                                                HoldTarget::ClearAll,
                                                self.clear_all_hold.clone(),
                                                Some(self.clear_all_focus.clone()),
                                                cx,
                                            ),
                                        ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(tokens.text_muted())
                            .child("Hold Clear for one second, or Clear All for two seconds. Keyboard activation asks for confirmation."),
                    )
                    .when_some(self.pending_confirm.clone(), |this, target| {
                        let label = match target {
                            HoldTarget::ClearUtility(_) => "Clear this Utility's retained entries?",
                            HoldTarget::ClearAll => "Clear all Rust History?",
                        };
                        this.child(
                            ConfirmationBar::new(
                                "settings.history.clear",
                                label,
                                "Confirm clear",
                                "Cancel",
                            )
                            .focus_handles(
                                self.confirm_focus.clone(),
                                self.cancel_confirm_focus.clone(),
                            )
                            .on_confirm(view_click(cx, |this, _window, cx| this.confirm(cx)))
                            .on_cancel(view_click(cx, |this, _window, cx| this.cancel_confirm(cx))),
                        )
                    })
                    .when_some(self.notice.as_ref(), |this, notice| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(tokens.accent())
                                .child(notice.clone()),
                        )
                    })
                    .child(div().h(px(1.)).bg(tokens.border()))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(tokens.text_muted())
                            .child(format!("PER UTILITY · {}", self.utilities.len())),
                    )
                    .child(utilities)
                    .when(!unknown.is_empty(), |this| {
                        this.child(div().h(px(1.)).bg(tokens.border()))
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(tokens.text_muted())
                                    .child("STORED FILES WITHOUT A REGISTERED UTILITY"),
                            )
                            .child(unknown_list)
                    }),
            )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum CaptureResult {
    IgnoredModifier,
    Cancelled,
    Invalid(String),
    Captured(Shortcut),
}

struct ShortcutCapture;

impl ShortcutCapture {
    /// Converts the focused native GPUI key event to the Carbon virtual-key
    /// code the global registrar requires. Modifier-only events have no key
    /// code and therefore cannot accidentally replace the active chord.
    fn record(key_code: Option<u32>, displayed_key: &str, modifiers: u32) -> CaptureResult {
        let Some(key_code) = key_code else {
            return CaptureResult::Invalid(
                "macOS did not provide a physical key code. Try capturing the shortcut again."
                    .into(),
            );
        };
        if key_code == 53 {
            return CaptureResult::Cancelled;
        }
        if matches!(key_code, 54..=63) {
            return CaptureResult::IgnoredModifier;
        }
        let label = display_name(displayed_key, modifiers);
        let shortcut = Shortcut::new(key_code, modifiers, label);
        match shortcut.validate() {
            Ok(()) => CaptureResult::Captured(shortcut),
            Err(error) => CaptureResult::Invalid(error.to_string()),
        }
    }
}

#[cfg(target_os = "macos")]
fn native_key_code() -> Option<u32> {
    use objc2_app_kit::NSApplication;
    use objc2_foundation::MainThreadMarker;

    let marker = MainThreadMarker::new()?;
    let event = NSApplication::sharedApplication(marker).currentEvent()?;
    Some(unsafe { event.keyCode() as u32 })
}

#[cfg(not(target_os = "macos"))]
fn native_key_code() -> Option<u32> {
    None
}

fn carbon_modifiers(modifiers: &gpui::Modifiers) -> u32 {
    let mut carbon = 0;
    if modifiers.shift {
        carbon |= SHIFT;
    }
    if modifiers.control {
        carbon |= CONTROL;
    }
    if modifiers.alt {
        carbon |= OPTION;
    }
    if modifiers.platform {
        carbon |= COMMAND;
    }
    carbon
}

fn display_name(key: &str, modifiers: u32) -> String {
    let mut label = String::new();
    if modifiers & CONTROL != 0 {
        label.push('⌃');
    }
    if modifiers & OPTION != 0 {
        label.push('⌥');
    }
    if modifiers & SHIFT != 0 {
        label.push('⇧');
    }
    if modifiers & COMMAND != 0 {
        label.push('⌘');
    }
    label.push_str(match key.to_ascii_lowercase().as_str() {
        "space" => "Space",
        "return" | "enter" => "Return",
        "delete" | "backspace" => "Delete",
        key => key,
    });
    label
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::{HistoryEntry, HistoryStore, HistoryViewState, SystemClock};
    use gpui::{Entity, Modifiers, MouseButton, VisualTestContext};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_ROOT: AtomicU64 = AtomicU64::new(0);

    struct HistoryProbe {
        history: Rc<HistoryRecorder>,
        utility_id: &'static str,
        view: HistoryViewState,
        active_content: String,
        visible: bool,
        notifications: usize,
        _subscription: HistorySubscription,
    }

    fn probe(
        cx: &mut App,
        history: Rc<HistoryRecorder>,
        utility_id: &'static str,
        visible: bool,
    ) -> gpui::Entity<HistoryProbe> {
        cx.new(|cx| {
            let weak: gpui::WeakEntity<HistoryProbe> = cx.weak_entity();
            let subscription = history.subscribe(utility_id, move |cx| {
                weak.update(cx, |this, cx| {
                    this.view.reconcile(&this.history, this.utility_id);
                    this.notifications += 1;
                    cx.notify();
                })
                .ok();
            });
            HistoryProbe {
                view: HistoryViewState::load(&history, utility_id),
                history,
                utility_id,
                active_content: format!("current {utility_id} input and result"),
                visible,
                notifications: 0,
                _subscription: subscription,
            }
        })
    }

    fn test_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "sofdevtool-settings-history-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_TEST_ROOT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn test_entry(utility_id: &str, id: &str) -> HistoryEntry {
        HistoryEntry {
            id: id.into(),
            captured_at: "2026-01-01T00:00:00Z".into(),
            utility_id: utility_id.into(),
            snapshot_version: 1,
            payload: serde_json::json!({"synthetic": id}),
        }
    }

    struct TestRoot(Entity<SettingsView>);

    impl Render for TestRoot {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(self.0.clone())
        }
    }

    #[gpui::test]
    fn settings_keyboard_confirmation_and_two_second_hold_clear_isolated_history(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(sofdevtool_ui::init);
        let root = test_root();
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.clone()),
            Box::new(SystemClock::new()),
        ));
        history
            .store()
            .record(test_entry("json", "json-one"))
            .unwrap();
        history
            .store()
            .record(test_entry("legacy-unknown", "unknown-one"))
            .unwrap();
        let mut captured = None;
        let window = cx.add_window(|window, cx| {
            let view = cx.new(|cx| {
                SettingsView::new(
                    window,
                    cx,
                    None,
                    ShortcutPreferences::new(root.clone()),
                    Rc::clone(&history),
                    vec![UtilityRow {
                        id: "json".into(),
                        name: "JSON".into(),
                        default_enabled: true,
                    }],
                    Shortcut::new(49, CONTROL | OPTION, "⌃⌥Space"),
                )
            });
            captured = Some(view.clone());
            TestRoot(view)
        });
        let settings = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = settings.read(cx).utility_focus[0].clear.clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(history.load("json").unwrap().len(), 1);
        assert!(settings.read_with(&cx, |view, _| view.pending_confirm.is_some()));
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = settings.read(cx).cancel_confirm_focus.clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert!(settings.read_with(&cx, |view, _| view.pending_confirm.is_none()));
        assert_eq!(history.load("json").unwrap().len(), 1);

        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = settings.read(cx).utility_focus[0].clear.clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = settings.read(cx).confirm_focus.clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert!(history.load("json").unwrap().is_empty());
        assert_eq!(history.load("legacy-unknown").unwrap().len(), 1);
        assert_eq!(
            settings.read_with(&cx, |view, _| view.notice.clone()),
            Some("Retained entries cleared.".into())
        );

        cx.update(|window, cx| window.draw(cx).clear(cx));
        let hit = cx.debug_bounds("settings-clear-all").unwrap().center();
        cx.simulate_mouse_move(hit, None, Modifiers::none());
        cx.simulate_mouse_down(hit, MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
        for _ in 0..19 {
            cx.executor().advance_clock(Duration::from_millis(100));
            cx.run_until_parked();
        }
        assert_eq!(history.load("legacy-unknown").unwrap().len(), 1);
        assert!(settings.read_with(&cx, |view, _| view.clear_all_hold.is_active()));
        cx.executor().advance_clock(Duration::from_millis(100));
        cx.run_until_parked();
        assert!(history.store().stored_utility_ids().unwrap().is_empty());
        assert_eq!(
            settings.read_with(&cx, |view, _| view.notice.clone()),
            Some("All Rust History cleared.".into())
        );
        cx.simulate_mouse_up(hit, MouseButton::Left, Modifiers::none());
        fs::remove_dir_all(root).unwrap();
    }

    #[gpui::test]
    fn settings_clear_reconciles_visible_and_hidden_history_entities(
        cx: &mut gpui::TestAppContext,
    ) {
        let root = test_root();
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.clone()),
            Box::new(SystemClock::new()),
        ));
        let json_entry = test_entry("json", "json-one");
        let base64_entry = test_entry("base64", "base64-one");
        history.store().record(json_entry.clone()).unwrap();
        history.store().record(base64_entry.clone()).unwrap();
        history
            .store()
            .record(test_entry("legacy-unknown", "unknown-one"))
            .unwrap();

        cx.update(|cx| {
            let json = probe(cx, Rc::clone(&history), "json", true);
            let base64 = probe(cx, Rc::clone(&history), "base64", false);
            json.update(cx, |this, _| {
                assert!(this.view.select("json-one"));
                this.view.pending_restore = Some(json_entry.clone());
            });
            base64.update(cx, |this, _| {
                assert!(this.view.select("base64-one"));
                this.view.pending_restore = Some(base64_entry.clone());
            });

            assert_eq!(
                clear_history(&history, &HoldTarget::ClearUtility("json".into()), cx),
                "Retained entries cleared."
            );
            let json_state = json.read(cx);
            assert_eq!(json_state.notifications, 1);
            assert!(json_state.visible);
            assert!(json_state.view.entries.is_empty());
            assert!(json_state.view.selected.is_none());
            assert!(json_state.view.pending_restore.is_none());
            assert_eq!(json_state.active_content, "current json input and result");
            assert_eq!(base64.read(cx).notifications, 0);
            assert!(base64.read(cx).view.selected.is_some());

            assert_eq!(
                clear_history(&history, &HoldTarget::ClearAll, cx),
                "All Rust History cleared."
            );
            let base64_state = base64.read(cx);
            assert_eq!(base64_state.notifications, 1);
            assert!(!base64_state.visible);
            assert!(base64_state.view.entries.is_empty());
            assert!(base64_state.view.selected.is_none());
            assert!(base64_state.view.pending_restore.is_none());
            assert_eq!(
                base64_state.active_content,
                "current base64 input and result"
            );
            assert!(history.store().stored_utility_ids().unwrap().is_empty());
        });
        fs::remove_dir_all(root).unwrap();
    }

    #[gpui::test]
    fn settings_partial_clear_reports_failure_and_reconciles_actual_storage(
        cx: &mut gpui::TestAppContext,
    ) {
        let root = test_root();
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.clone()),
            Box::new(SystemClock::new()),
        ));
        history
            .store()
            .record(test_entry("json", "json-one"))
            .unwrap();
        fs::create_dir(root.join("legacy-unknown.history.v1.json")).unwrap();
        cx.update(|cx| {
            let json = probe(cx, Rc::clone(&history), "json", false);
            assert_eq!(
                clear_history(&history, &HoldTarget::ClearAll, cx),
                "Cleared 1 History file(s); 1 could not be cleared."
            );
            assert_eq!(json.read(cx).notifications, 1);
            assert!(json.read(cx).view.entries.is_empty());
            assert_eq!(
                history.store().stored_utility_ids().unwrap(),
                vec!["legacy-unknown"]
            );
        });
        fs::remove_dir_all(root).unwrap();
    }

    #[gpui::test]
    fn settings_retry_reports_failure_then_resumes_without_duplicate_entry(
        cx: &mut gpui::TestAppContext,
    ) {
        let root = test_root();
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.clone()),
            Box::new(SystemClock::new()),
        ));
        history
            .store()
            .record(test_entry("json", "retained"))
            .unwrap();
        history.store().fail_next_writes(2);
        assert!(history
            .store()
            .record(test_entry("json", "failed"))
            .is_err());
        assert!(history.store().is_paused("json"));

        cx.update(|cx| {
            let json = probe(cx, Rc::clone(&history), "json", false);
            assert!(json.read(cx).view.error.is_some());
            let failed_notice = retry_history(&history, "json", cx);
            assert!(failed_notice.contains("still paused"));
            assert!(history.store().is_paused("json"));
            assert!(json.read(cx).view.error.is_some());

            assert_eq!(
                retry_history(&history, "json", cx),
                "History recording resumed without adding an entry."
            );
            assert!(!history.store().is_paused("json"));
            assert!(json.read(cx).view.error.is_none());
            assert_eq!(json.read(cx).view.entries.len(), 1);
            assert_eq!(history.load("json").unwrap()[0].id, "retained");
        });
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recorder_maps_a_real_chord_to_its_carbon_key_code() {
        assert_eq!(
            ShortcutCapture::record(Some(49), "space", CONTROL | OPTION),
            CaptureResult::Captured(Shortcut::new(49, CONTROL | OPTION, "⌃⌥Space"))
        );
    }

    #[test]
    fn recorder_ignores_pure_modifiers_and_cancels_with_escape() {
        assert_eq!(
            ShortcutCapture::record(Some(56), "shift", SHIFT),
            CaptureResult::IgnoredModifier
        );
        assert_eq!(
            ShortcutCapture::record(Some(53), "escape", CONTROL),
            CaptureResult::Cancelled
        );
    }

    #[test]
    fn recorder_requires_a_primary_modifier() {
        assert_eq!(
            ShortcutCapture::record(Some(0), "a", SHIFT),
            CaptureResult::Invalid("Use Command, Control, or Option with another key.".into())
        );
    }

    #[test]
    fn recorder_preserves_the_physical_forward_delete_code() {
        assert_eq!(
            ShortcutCapture::record(Some(117), "delete", CONTROL),
            CaptureResult::Captured(Shortcut::new(117, CONTROL, "⌃Delete"))
        );
    }

    #[test]
    fn hold_durations_match_the_contract() {
        assert_eq!(
            HoldTarget::ClearUtility("json".into()).duration(),
            Duration::from_secs(1)
        );
        assert_eq!(HoldTarget::ClearAll.duration(), Duration::from_secs(2));
    }
}
