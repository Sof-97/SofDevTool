//! TODO(ticket 13): YAML/JSON conversion workspace.
//!
//! Replace this stub with the real workspace. Keep the `construct` signature:
//! the Workbench calls it to build this Utility's view.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{div, AnyView, Context, IntoElement, Render, Window};

use crate::clipboard::Clipboard;
use crate::history::HistoryRecorder;
use crate::workbench::Workbench;

pub fn construct(
    window: &mut Window,
    cx: &mut Context<Workbench>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
) -> AnyView {
    cx.new(|cx| YamlJsonWorkspace::new(window, cx, clipboard, history))
        .into()
}

pub struct YamlJsonWorkspace;

impl YamlJsonWorkspace {
    pub fn new(
        _window: &mut Window,
        _cx: &mut Context<Self>,
        _clipboard: Rc<dyn Clipboard>,
        _history: Rc<HistoryRecorder>,
    ) -> Self {
        Self
    }
}

impl Render for YamlJsonWorkspace {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child("YAML/JSON conversion workspace")
    }
}
