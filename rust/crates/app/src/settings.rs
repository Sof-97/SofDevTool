//! Separate Settings surface for Launcher shortcut configuration.

use gpui::prelude::*;
use gpui::{
    div, px, size, Context, FocusHandle, IntoElement, KeyDownEvent, Render, Window, WindowBounds,
    WindowKind, WindowOptions,
};
use sofdevtool_ui::{mount, view_click, Button, ThemeTokens};

use crate::preferences::ShortcutPreferences;
use crate::shortcut::{Shortcut, COMMAND, CONTROL, OPTION, SHIFT};
use crate::workbench::Workbench;

/// Opens a focused, separate Settings window. The Workbench keeps ownership of
/// the operating-system registration; this view can only request a change.
pub fn show(
    workbench: gpui::WeakEntity<Workbench>,
    preferences: ShortcutPreferences,
    current_shortcut: Shortcut,
    cx: &mut gpui::App,
) {
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::centered(size(px(460.), px(260.)), cx)),
        focus: true,
        kind: WindowKind::Normal,
        is_resizable: false,
        titlebar: Some(gpui::TitlebarOptions {
            title: Some("Developer Toolbox Settings".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let _ = cx.open_window(options, move |window, cx| {
        let view =
            cx.new(|cx| SettingsView::new(window, cx, workbench, preferences, current_shortcut));
        mount(view, window, cx)
    });
}

pub struct SettingsView {
    workbench: gpui::WeakEntity<Workbench>,
    preferences: ShortcutPreferences,
    active_shortcut: Shortcut,
    capturing: bool,
    diagnostic: Option<String>,
    capture_focus: FocusHandle,
    capture_button_focus: FocusHandle,
}

impl SettingsView {
    fn new(
        _window: &mut Window,
        cx: &mut Context<Self>,
        workbench: gpui::WeakEntity<Workbench>,
        preferences: ShortcutPreferences,
        active_shortcut: Shortcut,
    ) -> Self {
        Self {
            workbench,
            preferences,
            active_shortcut,
            capturing: false,
            diagnostic: None,
            capture_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            capture_button_focus: cx.focus_handle().tab_stop(true).tab_index(1),
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
        let Some(current) = self
            .workbench
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
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tokens = ThemeTokens::graphite();
        let capture_text = if self.capturing {
            "Press a shortcut now (Escape cancels)"
        } else {
            &self.active_shortcut.display_name
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_5()
            .bg(tokens.background())
            .text_color(tokens.text())
            .track_focus(&self.capture_focus)
            .capture_key_down(
                cx.listener(|this, event, window, cx| this.receive_key(event, window, cx)),
            )
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Launcher shortcut"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(tokens.text_muted())
                    .child(capture_text.to_string()),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child("Use Command, Control, or Option with a non-modifier key."),
            )
            .child(
                Button::new("Capture shortcut")
                    .focus_handle(self.capture_button_focus.clone())
                    .on_click(view_click(cx, |this, window, cx| {
                        this.begin_capture(window, cx)
                    })),
            )
            .when_some(self.diagnostic.as_ref(), |this, diagnostic| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(tokens.danger())
                        .child(diagnostic.clone()),
                )
            })
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
