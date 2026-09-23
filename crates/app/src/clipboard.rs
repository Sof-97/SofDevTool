//! App-owned Clipboard boundary.
//!
//! Utilities never touch GPUI's clipboard directly; the composition root injects
//! this interface and provides the production adapter.

use gpui::{App, ClipboardItem};

pub trait Clipboard: 'static {
    fn read_text(&self, cx: &mut App) -> Option<String>;
    fn write_text(&self, text: &str, cx: &mut App);
}

/// Production adapter over GPUI's explicit clipboard API.
pub struct GpuiClipboard;

impl Clipboard for GpuiClipboard {
    fn read_text(&self, cx: &mut App) -> Option<String> {
        cx.read_from_clipboard().and_then(|item| item.text())
    }

    fn write_text(&self, text: &str, cx: &mut App) {
        cx.write_to_clipboard(ClipboardItem::new_string(text.to_string()));
    }
}
