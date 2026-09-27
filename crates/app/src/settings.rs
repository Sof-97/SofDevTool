//! Separate Settings surface for Launcher shortcut and History management.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, px, size, App, Context, FocusHandle, IntoElement, KeyDownEvent, Render, Subscription,
    Window, WindowBounds, WindowKind, WindowOptions,
};
use gpui_kit::component::{
    button::Button,
    switch::Switch,
    tab::{Tab, TabBar},
    ActiveTheme as _,
};

use crate::appearance::AppearanceMode;
use crate::history::{HistoryRecorder, HistorySubscription};
use crate::preferences::ShortcutPreferences;
use crate::shortcut::{Shortcut, COMMAND, CONTROL, OPTION, SHIFT};
use crate::ui;
use crate::workbench::Workbench;

/// A Utility row shown in the History section.
#[derive(Clone)]
pub struct UtilityRow {
    pub id: String,
    pub name: String,
    pub default_enabled: bool,
}

/// The scope of a destructive History clear. The scope is owned by the view;
/// the confirmation dialog only reports that the owner confirmed it.
#[derive(Clone, PartialEq, Eq)]
enum ClearTarget {
    ClearUtility(String),
    ClearAll,
}

impl ClearTarget {
    fn description(&self) -> &'static str {
        match self {
            ClearTarget::ClearUtility(_) => {
                "Retained History for this Utility will be removed. This cannot be undone."
            }
            ClearTarget::ClearAll => {
                "All retained History for every Utility will be removed. This cannot be undone."
            }
        }
    }
}

fn clear_history(history: &HistoryRecorder, target: &ClearTarget, cx: &mut App) -> String {
    match target {
        ClearTarget::ClearUtility(id) => match history.clear_utility(id, cx) {
            Ok(()) => "Retained entries cleared.".into(),
            Err(error) => format!("History could not be cleared: {error}"),
        },
        ClearTarget::ClearAll => match history.clear_all(cx) {
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
    appearance_mode: AppearanceMode,
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
                cx,
                Some(workbench),
                preferences,
                history,
                utilities,
                current_shortcut,
                appearance_mode,
            )
        });
        cx.new(|cx| gpui_kit::component::Root::new(view, window, cx))
    });
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
    capture_focus: FocusHandle,
    appearance_mode: AppearanceMode,
    _workbench_subscription: Option<Subscription>,
    _history_subscription: HistorySubscription,
}

impl SettingsView {
    fn new(
        cx: &mut Context<Self>,
        workbench: Option<gpui::WeakEntity<Workbench>>,
        preferences: ShortcutPreferences,
        history: Rc<HistoryRecorder>,
        utilities: Vec<UtilityRow>,
        active_shortcut: Shortcut,
        appearance_mode: AppearanceMode,
    ) -> Self {
        let workbench_subscription = workbench
            .as_ref()
            .and_then(|workbench| workbench.upgrade())
            .map(|workbench| {
                let weak_view = cx.weak_entity();
                cx.observe(&workbench, move |_, workbench, cx| {
                    let weak_view = weak_view.clone();
                    let workbench = workbench.clone();
                    cx.defer(move |cx| {
                        let mode = workbench.read(cx).appearance_mode();
                        weak_view
                            .update(cx, |view, cx| {
                                if view.appearance_mode != mode {
                                    view.appearance_mode = mode;
                                    cx.notify();
                                }
                            })
                            .ok();
                    });
                })
            });
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe_status(move |cx| {
            let weak = weak.clone();
            cx.defer(move |cx| {
                weak.update(cx, |_, cx| cx.notify()).ok();
            });
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
            capture_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            appearance_mode,
            _workbench_subscription: workbench_subscription,
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
        target: ClearTarget,
        cx: &mut Context<Self>,
    ) -> Button {
        let weak = cx.weak_entity();
        let confirm_target = target.clone();
        let description = target.description();
        Button::new(id)
            .label(label)
            .on_click(move |_event, window, cx| {
                let weak = weak.clone();
                let confirm_target = confirm_target.clone();
                // The kit dialog owns focus, keyboard handling and dismissal.
                // Cancel has no callback effect, so it leaves every retained
                // entry untouched; confirming performs the captured scope.
                ui::confirm_dialog(
                    window,
                    cx,
                    "Clear History",
                    description,
                    "Clear",
                    "Cancel",
                    move |_window, cx| {
                        weak.update(cx, |this, cx| {
                            this.perform(confirm_target.clone(), cx);
                        })
                        .ok();
                    },
                    |_window, _cx| {},
                );
            })
    }

    fn perform(&mut self, target: ClearTarget, cx: &mut Context<Self>) {
        self.notice = Some(clear_history(&self.history, &target, cx));
        cx.notify();
    }

    fn retry_recording(&mut self, utility_id: &str, cx: &mut Context<Self>) {
        self.notice = Some(retry_history(&self.history, utility_id, cx));
        cx.notify();
    }

    fn set_appearance(
        &mut self,
        mode: AppearanceMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(workbench) = self.workbench.as_ref() else {
            self.notice = Some("The Workbench is no longer available.".into());
            cx.notify();
            return;
        };
        if workbench
            .update(cx, |workbench, cx| {
                workbench.set_appearance(mode, window, cx)
            })
            .is_ok()
        {
            self.appearance_mode = mode;
            cx.notify();
        }
    }

    fn stored_ids(&self) -> Vec<String> {
        self.history
            .store()
            .stored_utility_ids()
            .unwrap_or_default()
    }
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let capture_text = if self.capturing {
            "Press a shortcut now (Escape cancels)"
        } else {
            &self.active_shortcut.display_name
        };

        let policy = self.history.policy();
        let mut utilities = div().flex().flex_col().w_full();
        for row in self.utilities.clone().iter() {
            let recording = policy
                .per_utility
                .get(&row.id)
                .copied()
                .unwrap_or(row.default_enabled);
            let paused = self.history.store().is_paused(&row.id);
            let effective = self.history.recording_state(&row.id);
            let id = row.id.clone();
            utilities = utilities.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .w_full()
                    .min_h(px(40.))
                    .py_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(div().flex_1().min_w_0().text_sm().child(row.name.clone()))
                    .child(
                        div()
                            .text_xs()
                            .text_color(if paused && policy.global_enabled && recording {
                                theme.warning
                            } else {
                                theme.muted_foreground
                            })
                            .child(effective.label()),
                    )
                    .child(
                        Switch::new(format!("history-record-{}", row.id))
                            .checked(recording)
                            .accessibility_label(format!("Record History for {}", row.name))
                            .on_change(cx.listener(move |this, next, _window, cx| {
                                this.history.set_utility_enabled(&id, *next);
                                this.history.notify_status(cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .debug_selector({
                                let selector_id = row.id.clone();
                                move || format!("history-clear-{selector_id}")
                            })
                            .child(self.clear_button(
                                format!("history-clear-{}", row.id),
                                "Clear",
                                ClearTarget::ClearUtility(row.id.clone()),
                                cx,
                            )),
                    )
                    .when(paused, |this| {
                        let retry_id = row.id.clone();
                        this.child(
                            Button::new(format!("history-retry-{}", row.id))
                                .label("Retry")
                                .on_click(cx.listener(move |this, _event, _window, cx| {
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
        let mut unknown_list = div().flex().flex_col().w_full();
        for id in &unknown {
            unknown_list = unknown_list.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .min_h(px(40.))
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(format!("Unknown Utility file: {id}")),
                    )
                    .child(
                        div()
                            .debug_selector({
                                let selector_id = id.clone();
                                move || format!("history-clear-unknown-{selector_id}")
                            })
                            .child(self.clear_button(
                                format!("history-clear-unknown-{id}"),
                                "Clear",
                                ClearTarget::ClearUtility(id.clone()),
                                cx,
                            )),
                    ),
            );
        }

        // The kit dialog layer is owned by the window's `Root`. This view opens
        // the confirmation dialogs, so it mounts that layer alongside its own
        // content; it renders nothing until a dialog is opened.
        let dialog_layer = gpui_kit::component::Root::render_dialog_layer(window, cx);

        div()
            .id("settings-scroll")
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .bg(theme.background)
            .text_color(theme.foreground)
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
                            .text_color(theme.muted_foreground)
                            .child("LOCAL · THIS MAC"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .py_3()
                    .border_t_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Appearance"),
                    )
                    .child(
                        TabBar::new("settings.appearance")
                            .segmented()
                            .selected_index(match self.appearance_mode {
                                AppearanceMode::System => 0,
                                AppearanceMode::Light => 1,
                                AppearanceMode::Dark => 2,
                            })
                            .child(Tab::new().label("System"))
                            .child(Tab::new().label("Light"))
                            .child(Tab::new().label("Dark"))
                            .on_click(cx.listener(|this, index, window, cx| {
                                let mode = match index {
                                    0 => AppearanceMode::System,
                                    1 => AppearanceMode::Light,
                                    _ => AppearanceMode::Dark,
                                };
                                this.set_appearance(mode, window, cx);
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .py_3()
                    .border_t_1()
                    .border_color(theme.border)
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
                                    .min_w_0()
                                    .text_xs()
                                    .text_color(theme.primary)
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
                                    .min_w_0()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child("Command, Control, or Option with a key"),
                            )
                            .child(
                                Button::new("settings.capture-shortcut")
                                    .label("Capture shortcut")
                                    .on_click(cx.listener(|this, _event, window, cx| {
                                        this.begin_capture(window, cx)
                                    })),
                            ),
                    )
                    .when_some(self.diagnostic.as_ref(), |this, diagnostic| {
                        this.child(ui::diagnostic_banner(
                            cx,
                            ui::DiagnosticSeverity::Error,
                            diagnostic,
                            None,
                        ))
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .py_3()
                    .border_t_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div().flex().flex_col().gap_1().child(
                                    div()
                                        .text_sm()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child("History"),
                                ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(div().text_xs().child("Record operations"))
                                    .child(
                                        Switch::new("history-global-record")
                                            .checked(policy.global_enabled)
                                            .accessibility_label("Record History globally")
                                            .on_change(cx.listener(|this, next, _window, cx| {
                                                this.history.set_global_enabled(*next);
                                                this.history.notify_status(cx);
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        div().debug_selector(|| "settings-clear-all".into()).child(
                                            self.clear_button(
                                                "history-clear-all".to_owned(),
                                                "Clear All",
                                                ClearTarget::ClearAll,
                                                cx,
                                            ),
                                        ),
                                    ),
                            ),
                    )
                    .when_some(self.notice.as_ref(), |this, notice| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(theme.primary)
                                .child(notice.clone()),
                        )
                    })
                    .child(div().h(px(1.)).bg(theme.border))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.muted_foreground)
                            .child(format!("PER UTILITY · {}", self.utilities.len())),
                    )
                    .child(utilities)
                    .when(!unknown.is_empty(), |this| {
                        this.child(div().h(px(1.)).bg(theme.border))
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.muted_foreground)
                                    .child("STORED FILES WITHOUT A REGISTERED UTILITY"),
                            )
                            .child(unknown_list)
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .py_3()
                    .border_t_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("About"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(crate::identity::build_description()),
                    ),
            )
            .children(dialog_layer)
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
    use gpui::{Modifiers, VisualTestContext};
    use gpui_kit::component::WindowExt as _;
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

    /// Clicks a control through the debug selector wrapped around it and
    /// redraws, so a confirmation dialog it opened is mounted.
    fn click_control(cx: &mut VisualTestContext, selector: &'static str) {
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let hit = cx
            .debug_bounds(selector)
            .expect("control is rendered")
            .center();
        cx.simulate_click(hit, Modifiers::none());
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    #[gpui::test]
    fn settings_confirmation_dialog_scopes_clear_and_cancel_changes_nothing(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
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
                    AppearanceMode::System,
                )
            });
            captured = Some(view.clone());
            gpui_kit::component::Root::new(view, window, cx)
        });
        let settings = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);

        // Activating "Clear" opens the standard kit confirmation dialog.
        click_control(&mut cx, "history-clear-json");
        assert_eq!(history.load("json").unwrap().len(), 1);
        assert_eq!(history.load("legacy-unknown").unwrap().len(), 1);
        assert!(cx.update(|window, cx| window.has_active_dialog(cx)));

        // Cancelling with the keyboard changes nothing: entries stay.
        cx.simulate_keystrokes("escape");
        assert!(!cx.update(|window, cx| window.has_active_dialog(cx)));
        assert_eq!(history.load("json").unwrap().len(), 1);
        assert_eq!(history.load("legacy-unknown").unwrap().len(), 1);

        // Confirming with the keyboard clears exactly this Utility's entries.
        click_control(&mut cx, "history-clear-json");
        assert!(cx.update(|window, cx| window.has_active_dialog(cx)));
        cx.simulate_keystrokes("enter");
        assert!(!cx.update(|window, cx| window.has_active_dialog(cx)));
        assert!(history.load("json").unwrap().is_empty());
        assert_eq!(history.load("legacy-unknown").unwrap().len(), 1);
        assert_eq!(
            settings.read_with(&cx, |view, _| view.notice.clone()),
            Some("Retained entries cleared.".into())
        );

        // Confirming Clear All clears every remaining stored file.
        click_control(&mut cx, "settings-clear-all");
        assert!(cx.update(|window, cx| window.has_active_dialog(cx)));
        cx.simulate_keystrokes("enter");
        assert!(history.store().stored_utility_ids().unwrap().is_empty());
        assert_eq!(
            settings.read_with(&cx, |view, _| view.notice.clone()),
            Some("All Rust History cleared.".into())
        );
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
                clear_history(&history, &ClearTarget::ClearUtility("json".into()), cx),
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
                clear_history(&history, &ClearTarget::ClearAll, cx),
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
                clear_history(&history, &ClearTarget::ClearAll, cx),
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
}
