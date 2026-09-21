//! Text Diff's application-owned workspace boundary.
//!
//! The workspace exposes GPUI controls and never exposes WebKit/Wry types. The
//! macOS renderer is a child view of the existing GPUI window and receives only
//! an exact UTF-8 snapshot plus a monotonically increasing revision.

mod renderer;

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, Context, FocusHandle, IntoElement, Render, Subscription, Window};
use sofdevtool_ui::{
    diagnostic_banner, panel, Button, DiagnosticSeverity, LabeledField, TextEditor,
};

use crate::clipboard::Clipboard;
use renderer::{RendererStatus, TextDiffRenderer, WebDiffSurface};

pub const TEXT_DIFF_UTILITY_ID: &str = "text-diff";

struct ButtonFocus {
    split: FocusHandle,
    unified: FocusHandle,
    paste_original: FocusHandle,
    paste_updated: FocusHandle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DisplayMode {
    Split,
    Unified,
}

impl DisplayMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Split => "split",
            Self::Unified => "unified",
        }
    }
}

/// An in-memory Text Diff Utility Workspace Session.
pub struct TextDiffWorkspace {
    old: TextEditor,
    new: TextEditor,
    clipboard: Rc<dyn Clipboard>,
    renderer: TextDiffRenderer,
    mode: DisplayMode,
    revision: u64,
    diagnostic: Option<String>,
    renderer_status: RendererStatus,
    renderer_status_changed: Rc<Cell<bool>>,
    focus: ButtonFocus,
    _subscriptions: Vec<Subscription>,
}

impl TextDiffWorkspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, clipboard: Rc<dyn Clipboard>) -> Self {
        let old = TextEditor::new(window, cx);
        let new = TextEditor::new(window, cx);
        old.set_text("let café = \"👨‍👩‍👧‍👦\"\nlet flag = \"🏳️‍🌈\"\n", window, cx);
        new.set_text(
            "let café = \"family 👨‍👩‍👧‍👦\"\nlet flag = \"🏳️‍🌈\"\nlet ready = true\n",
            window,
            cx,
        );
        let renderer_status_changed = Rc::new(Cell::new(false));
        let renderer = TextDiffRenderer::new(window, renderer_status_changed.clone());
        let diagnostic = renderer
            .render(1, old.text(cx), new.text(cx), DisplayMode::Split.as_str())
            .err();
        let renderer_status = renderer.status();
        let subscriptions = vec![
            old.on_change_in(window, cx, |this, window, cx| {
                this.render_snapshot(window, cx)
            }),
            new.on_change_in(window, cx, |this, window, cx| {
                this.render_snapshot(window, cx)
            }),
        ];
        let workspace = Self {
            old,
            new,
            clipboard,
            renderer,
            mode: DisplayMode::Split,
            revision: 1,
            diagnostic,
            renderer_status,
            renderer_status_changed,
            focus: ButtonFocus {
                split: cx.focus_handle().tab_stop(true).tab_index(0),
                unified: cx.focus_handle().tab_stop(true).tab_index(0),
                paste_original: cx.focus_handle().tab_stop(true).tab_index(0),
                paste_updated: cx.focus_handle().tab_stop(true).tab_index(0),
            },
            _subscriptions: subscriptions,
        };
        workspace.observe_renderer_status(cx);
        workspace
    }

    /// The workbench calls this when this workspace is selected or covered.
    /// A native child view is outside GPUI's normal clipping tree, so hiding it
    /// explicitly prevents it from overlaying another workspace.
    pub fn set_active(&self, active: bool) {
        self.renderer.set_active(active);
    }

    /// Return keyboard ownership from the native child to GPUI controls.
    pub fn focus_parent(&self) {
        self.renderer.focus_parent();
    }

    /// Standalone-proof hook: this submits a malformed request through the
    /// same local bridge, so the visible diagnostic proves IPC error handling.
    #[cfg(debug_assertions)]
    pub fn simulate_renderer_failure(&mut self, cx: &mut Context<Self>) {
        self.revision += 1;
        self.diagnostic = self.renderer.render_proof_failure(self.revision).err();
        self.renderer_status = self.renderer.status();
        cx.notify();
    }

    fn render_snapshot(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.revision += 1;
        self.diagnostic = self
            .renderer
            .render(
                self.revision,
                self.old.text(cx),
                self.new.text(cx),
                self.mode.as_str(),
            )
            .err();
        self.renderer_status = self.renderer.status();
        window.refresh();
        cx.notify();
    }

    /// IPC handlers run outside GPUI's entity update path. They set this
    /// signal; this lightweight observer transfers the resulting status into
    /// the workspace and invalidates the visible status immediately.
    fn observe_renderer_status(&self, cx: &mut Context<Self>) {
        let changed = self.renderer_status_changed.clone();
        let executor = cx.background_executor().clone();
        cx.spawn(async move |this, cx| loop {
            executor.timer(Duration::from_millis(25)).await;
            if this
                .update(cx, |this, cx| {
                    if changed.replace(false) {
                        this.renderer_status = this.renderer.status();
                        cx.notify();
                    }
                })
                .is_err()
            {
                break;
            }
        })
        .detach();
    }

    fn set_mode(&mut self, mode: DisplayMode, window: &mut Window, cx: &mut Context<Self>) {
        self.renderer.focus_parent();
        self.mode = mode;
        self.render_snapshot(window, cx);
    }

    fn paste(&self, editor: &TextEditor, window: &mut Window, cx: &mut Context<Self>) {
        self.renderer.focus_parent();
        if let Some(text) = self.clipboard.read_text(cx) {
            editor.replace_all(text, window, cx);
        }
    }
}

impl Drop for TextDiffWorkspace {
    fn drop(&mut self) {
        self.renderer.set_active(false);
    }
}

impl Render for TextDiffWorkspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mode = self.mode;
        let old = self.old.render(false, "text-diff.original");
        let new = self.new.render(false, "text-diff.updated");
        let mut root = div()
            .id("text-diff-workspace")
            .flex()
            .flex_col()
            .gap_3()
            .size_full()
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("Split")
                            .focus_handle(self.focus.split.clone())
                            .on_click(sofdevtool_ui::view_click(cx, |this, window, cx| {
                                this.set_mode(DisplayMode::Split, window, cx)
                            })),
                    )
                    .child(
                        Button::new("Unified")
                            .focus_handle(self.focus.unified.clone())
                            .on_click(sofdevtool_ui::view_click(cx, |this, window, cx| {
                                this.set_mode(DisplayMode::Unified, window, cx)
                            })),
                    )
                    .child(
                        Button::new("Paste original")
                            .focus_handle(self.focus.paste_original.clone())
                            .on_click(sofdevtool_ui::view_click(cx, |this, window, cx| {
                                let old = &this.old;
                                this.paste(old, window, cx)
                            })),
                    )
                    .child(
                        Button::new("Paste updated")
                            .focus_handle(self.focus.paste_updated.clone())
                            .on_click(sofdevtool_ui::view_click(cx, |this, window, cx| {
                                let new = &this.new;
                                this.paste(new, window, cx)
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_3()
                    .h_48()
                    .flex_shrink_0()
                    .child(div().flex().flex_1().min_h_0().child(panel(
                        "Original",
                        "neutral",
                        LabeledField::new("Original", old),
                    )))
                    .child(div().flex().flex_1().min_h_0().child(panel(
                        "Updated",
                        "neutral",
                        LabeledField::new("Updated", new),
                    ))),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(WebDiffSurface::new(self.renderer.clone())),
            );
        root = match &self.renderer_status {
            RendererStatus::Loading => {
                root.child(div().text_xs().child("Loading local diff renderer…"))
            }
            RendererStatus::Ready => root,
            RendererStatus::Failure(message) => {
                root.child(diagnostic_banner(DiagnosticSeverity::Error, message, None))
            }
        };
        if contains_complex_emoji(&self.old.text(cx)) || contains_complex_emoji(&self.new.text(cx))
        {
            root = root.child(div().text_xs().child(
                "Complex emoji uses whole-line highlighting to preserve Unicode correctness.",
            ));
        }
        if let Some(message) = &self.diagnostic {
            root = root.child(diagnostic_banner(DiagnosticSeverity::Error, message, None));
        }
        let _ = mode;
        root
    }
}

fn contains_complex_emoji(text: &str) -> bool {
    text.chars().any(|character| {
        let scalar = character as u32;
        (0x1F000..=0x1FAFF).contains(&scalar) || (0x2600..=0x27BF).contains(&scalar)
    })
}
