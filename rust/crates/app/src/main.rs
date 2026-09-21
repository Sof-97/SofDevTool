//! Entry point for the Rust Developer Toolbox.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{App, TitlebarOptions, WindowOptions};
use sofdevtool_app::clipboard::{Clipboard, GpuiClipboard};
use sofdevtool_app::identity;
use sofdevtool_app::json_workspace::JsonWorkspace;
use sofdevtool_ui::{init, mount, run, set_dark_theme};

fn window_options() -> WindowOptions {
    WindowOptions {
        titlebar: Some(TitlebarOptions {
            title: Some(identity::APP_DISPLAY_NAME.into()),
            ..Default::default()
        }),
        app_id: Some(identity::BUNDLE_IDENTIFIER.to_string()),
        ..Default::default()
    }
}

fn main() {
    run(|cx: &mut App| {
        init(cx);
        set_dark_theme(None, cx);
        let clipboard: Rc<dyn Clipboard> = Rc::new(GpuiClipboard);
        cx.open_window(window_options(), move |window, cx| {
            let view = cx.new(|cx| JsonWorkspace::new(window, cx, clipboard.clone()));
            mount(view, window, cx)
        })
        .expect("open the workbench window");
    });
}
