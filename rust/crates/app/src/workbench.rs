//! The long-lived application shell and concrete Utility workspace sessions.

use std::rc::Rc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, AnyWindowHandle, App, Context, Entity, FocusHandle, IntoElement, Render, Window};
use sofdevtool_ui::{panel, view_click, Button, ButtonVariant, ThemeTokens};

use crate::clipboard::Clipboard;
use crate::history::HistoryRecorder;
use crate::json_workspace::JsonWorkspace;
use crate::preferences::{ShortcutPreferences, StartupShortcut};
use crate::registry::{OpenUtility, UtilityId, UtilityRegistry};
use crate::shortcut::{Shortcut, ShortcutController};
use crate::text_diff::TextDiffWorkspace;

#[cfg(target_os = "macos")]
use crate::shortcut::macos::CarbonShortcutRegistrar;

/// The GPUI Workbench retains each concrete workspace entity for the whole
/// application run. No workspace input or result is persisted here.
pub struct Workbench {
    registry: UtilityRegistry,
    selected: UtilityId,
    json: Entity<JsonWorkspace>,
    text_diff: Entity<TextDiffWorkspace>,
    launcher_focus: FocusHandle,
    launcher: Option<AnyWindowHandle>,
    main_window: AnyWindowHandle,
    entity: gpui::WeakEntity<Self>,
    shortcut_requested: Arc<AtomicBool>,
    #[cfg(target_os = "macos")]
    shortcut: ShortcutController<CarbonShortcutRegistrar>,
    shortcut_error: Option<String>,
    shortcut_preferences: Option<ShortcutPreferences>,
}

impl Workbench {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
    ) -> Self {
        let json = cx.new(|cx| JsonWorkspace::new(window, cx, clipboard.clone(), history));
        let text_diff = cx.new(|cx| TextDiffWorkspace::new(window, cx, clipboard));
        text_diff.read(cx).set_active(false);
        let shortcut_requested = Arc::new(AtomicBool::new(false));
        #[cfg(target_os = "macos")]
        let mut shortcut = ShortcutController::new(CarbonShortcutRegistrar::new({
            let shortcut_requested = shortcut_requested.clone();
            move || shortcut_requested.store(true, Ordering::Release)
        }));
        #[cfg(target_os = "macos")]
        let shortcut_error = shortcut
            .register(Shortcut::launcher_default())
            .err()
            .map(|error| error.to_string());
        let workbench = Self {
            registry: UtilityRegistry::initial(),
            selected: UtilityId::Json,
            json,
            text_diff,
            launcher_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            launcher: None,
            main_window: window.window_handle(),
            entity: cx.weak_entity(),
            shortcut_requested,
            #[cfg(target_os = "macos")]
            shortcut,
            shortcut_error,
            shortcut_preferences: None,
        };
        workbench.observe_global_shortcut(cx);
        workbench
    }

    pub fn open(&mut self, request: OpenUtility, cx: &mut Context<Self>) -> bool {
        let opened = self.registry.open(request.0, &mut self.selected);
        if opened {
            self.sync_workspace_visibility(cx);
            cx.notify();
        }
        opened
    }

    fn sync_workspace_visibility(&self, cx: &App) {
        self.text_diff
            .read(cx)
            .set_active(self.selected == UtilityId::TextDiff && self.launcher.is_none());
    }

    fn select_json(&mut self, cx: &mut Context<Self>) {
        self.open(OpenUtility(UtilityId::Json), cx);
    }

    fn show_launcher(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(launcher) = self.launcher.take() {
            if launcher
                .update(cx, |_launcher, launcher_window, _cx| {
                    launcher_window.remove_window()
                })
                .is_ok()
            {
                return;
            }
        }
        self.text_diff.read(cx).set_active(false);
        self.launcher = crate::launcher::show(
            self.registry.clone(),
            window.window_handle(),
            cx.weak_entity(),
            cx,
        );
    }

    /// Called from the application-owned shortcut observer rather than a
    /// render pass. A hidden Workbench remains a live GPUI window, so this
    /// works when the user has closed its visible native surface.
    fn consume_global_shortcut(&mut self, cx: &mut Context<Self>) {
        if !self.shortcut_requested.swap(false, Ordering::AcqRel) {
            return;
        }
        if let Some(launcher) = self.launcher.take() {
            if launcher
                .update(cx, |_launcher, launcher_window, _cx| {
                    launcher_window.remove_window()
                })
                .is_ok()
            {
                self.sync_workspace_visibility(cx);
                cx.notify();
                return;
            }
        }
        let main_window = self.main_window;
        self.text_diff.read(cx).set_active(false);
        self.launcher =
            crate::launcher::show(self.registry.clone(), main_window, self.entity.clone(), cx);
    }

    pub fn launcher_did_dismiss(&mut self, cx: &mut Context<Self>) {
        self.launcher = None;
        self.sync_workspace_visibility(cx);
        cx.notify();
    }

    fn observe_global_shortcut(&self, cx: &mut Context<Self>) {
        let requested = self.shortcut_requested.clone();
        let executor = cx.background_executor().clone();
        cx.spawn(async move |this, cx| loop {
            executor.timer(Duration::from_millis(50)).await;
            if this
                .update(cx, |_workbench, cx| {
                    if requested.load(Ordering::Acquire) {
                        _workbench.consume_global_shortcut(cx);
                    }
                })
                .is_err()
            {
                break;
            }
        })
        .detach();
    }

    pub fn configure_shortcut(&mut self, shortcut: Shortcut, cx: &mut Context<Self>) {
        #[cfg(target_os = "macos")]
        {
            if let Err(error) = self.shortcut.register(shortcut) {
                self.shortcut_error = Some(error.to_string());
            } else {
                self.shortcut_error = None;
            }
            cx.notify();
        }
    }

    pub fn restore_shortcut_preferences(
        &mut self,
        preferences: Option<ShortcutPreferences>,
        startup: StartupShortcut,
        cx: &mut Context<Self>,
    ) {
        self.shortcut_preferences = preferences;
        let mut diagnostics = Vec::new();
        if let Some(shortcut) = startup.shortcut {
            #[cfg(target_os = "macos")]
            if let Err(error) = self.shortcut.register(shortcut) {
                diagnostics.push(error.to_string());
            }
        } else if let Some(error) = self.shortcut_error.take() {
            diagnostics.push(error);
        }
        diagnostics.extend(startup.diagnostic);
        // A failed default registration remains visible when no saved shortcut
        // replaces it. A successfully registered saved shortcut clears that
        // stale default error instead of presenting an unreachable state.
        self.shortcut_error = (!diagnostics.is_empty()).then(|| diagnostics.join(" "));
        cx.notify();
    }

    fn show_settings(&mut self, cx: &mut Context<Self>) {
        if let Some(preferences) = self.shortcut_preferences.clone() {
            crate::settings::show(cx.weak_entity(), preferences, self.current_shortcut(), cx);
        } else {
            self.shortcut_error = Some(
                "Launcher settings are unavailable because the Rust Application Support directory could not be resolved."
                    .into(),
            );
            cx.notify();
        }
    }

    /// The committed OS shortcut. Settings persists only this value after the
    /// native registrar accepts a candidate.
    pub fn current_shortcut(&self) -> Shortcut {
        #[cfg(target_os = "macos")]
        {
            self.shortcut.shortcut()
        }
        #[cfg(not(target_os = "macos"))]
        {
            Shortcut::launcher_default()
        }
    }
}

impl Render for Workbench {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tokens = ThemeTokens::graphite();
        let json_selected = self.selected == UtilityId::Json;
        let text_diff_selected = self.selected == UtilityId::TextDiff;
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(tokens.background())
            .text_color(tokens.text())
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(tokens.border())
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Developer Toolbox"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_xs()
                            .text_color(tokens.text_muted())
                            .child("Library · local and offline")
                            .child(
                                Button::new("Settings").variant(ButtonVariant::Secondary).on_click(
                                    view_click(cx, |this, _window, cx| this.show_settings(cx)),
                                ),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .w_56()
                            .p_3()
                            .border_r_1()
                            .border_color(tokens.border())
                            .child(
                                Button::new("JSON")
                                    .variant(if json_selected {
                                        ButtonVariant::Primary
                                    } else {
                                        ButtonVariant::Secondary
                                    })
                                    .on_click(view_click(cx, |this, _window, cx| {
                                        this.select_json(cx);
                                    })),
                            )
                            .child(
                                Button::new("Text Diff")
                                    .variant(if text_diff_selected {
                                        ButtonVariant::Primary
                                    } else {
                                        ButtonVariant::Secondary
                                    })
                                    .on_click(view_click(cx, |this, _window, cx| {
                                        this.open(OpenUtility(UtilityId::TextDiff), cx);
                                    })),
                            )
                            .child(
                                div()
                                    .mt_4()
                                    .text_xs()
                                    .text_color(tokens.text_muted())
                                    .child("More Utilities arrive only when their concrete workspaces are ready."),
                            ),
                    )
                    .child(
                        if json_selected {
                            div().m_3().flex().flex_1().min_w_0().min_h_0().child(panel(
                                "JSON",
                                "persistent session",
                                div().flex().flex_1().min_h_0().child(self.json.clone()),
                            ))
                        } else {
                            div().m_3().flex().flex_1().min_w_0().min_h_0().child(panel(
                                "Text Diff",
                                "persistent session",
                                div().flex().flex_1().min_h_0().child(self.text_diff.clone()),
                            ))
                        },
                    ),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .px_4()
                    .py_3()
                    .border_t_1()
                    .border_color(tokens.border())
                    .child(
                        Button::new("Open Utility Launcher")
                            .focus_handle(self.launcher_focus.clone())
                            .on_click(view_click(cx, |this, window, cx| {
                                this.show_launcher(window, cx);
                            })),
                    ),
            )
            .when_some(self.shortcut_error.as_ref(), |this, error| {
                this.child(
                    div()
                        .px_4()
                        .py_2()
                        .text_xs()
                        .text_color(tokens.danger())
                        .child(format!("Launcher: {error}")),
                )
            })
    }
}
