//! Narrow AppKit operations that GPUI does not expose as public window APIs.
//!
//! GPUI owns the lifetime of the native view. These functions borrow its
//! AppKit view only for the duration of the call and never retain it.

#![allow(unexpected_cfgs)] // objc 0.2's selector macro probes cargo-clippy.

#[cfg(target_os = "macos")]
use gpui::Window;

/// Keeps the GPUI window and any child native views alive while hiding the
/// Workbench. Recreating its root would attach an existing WKWebView to a
/// different NSWindow, which is not a supported Wry lifecycle.
#[cfg(target_os = "macos")]
pub fn hide(window: &Window) {
    with_ns_window(window, |native| unsafe {
        use objc::{msg_send, sel, sel_impl};
        let nil = std::ptr::null_mut::<objc::runtime::Object>();
        let _: () = msg_send![native, orderOut: nil];
    });
}

/// Restores the retained native Workbench window without remounting its root.
#[cfg(target_os = "macos")]
pub fn show(window: &Window) {
    with_ns_window(window, |native| unsafe {
        use objc::{msg_send, sel, sel_impl};
        let nil = std::ptr::null_mut::<objc::runtime::Object>();
        let _: () = msg_send![native, makeKeyAndOrderFront: nil];
    });
}

/// Orders a nonactivating Launcher panel above the current application without
/// bringing the Workbench forward. GPUI's `focus: true` creates the correct
/// nonactivating panel class and makes it key; AppKit still needs this explicit
/// order operation for a popup created while another application owns the key
/// window.
#[cfg(target_os = "macos")]
pub fn show_nonactivating(window: &Window) {
    with_ns_window(window, |native| unsafe {
        use objc::{msg_send, sel, sel_impl};
        let _: () = msg_send![native, orderFrontRegardless];
    });
}

/// Lets the Launcher join every Space and float above a full-screen application.
///
/// `CanJoinAllSpaces | Stationary | FullScreenAuxiliary`. GPUI already assigns
/// the popup window level, so no level change is needed here.
#[cfg(target_os = "macos")]
pub fn configure_launcher_panel(window: &Window) {
    const CAN_JOIN_ALL_SPACES: u64 = 1 << 0;
    const STATIONARY: u64 = 1 << 4;
    const FULL_SCREEN_AUXILIARY: u64 = 1 << 8;
    let behavior = CAN_JOIN_ALL_SPACES | STATIONARY | FULL_SCREEN_AUXILIARY;
    with_ns_window(window, |native| unsafe {
        use objc::{msg_send, sel, sel_impl};
        let _: () = msg_send![native, setCollectionBehavior: behavior];
    });
}

#[cfg(not(target_os = "macos"))]
pub fn configure_launcher_panel(_window: &gpui::Window) {}

/// Makes the Launcher panel the key window for keyboard input.
///
/// A nonactivating `NSPanel` can be key without activating the owning
/// application. GPUI requests key status during window creation, but when the
/// panel opens over an already-active Workbench that request can be lost, so the
/// Launcher reasserts it once its root view is mounted.
#[cfg(target_os = "macos")]
pub fn focus_launcher(window: &Window) {
    with_ns_window(window, |native| unsafe {
        use objc::{msg_send, sel, sel_impl};
        let _: () = msg_send![native, makeKeyWindow];
    });
}

#[cfg(not(target_os = "macos"))]
pub fn focus_launcher(_window: &gpui::Window) {}

#[cfg(not(target_os = "macos"))]
pub fn show_nonactivating(_window: &gpui::Window) {}

#[cfg(target_os = "macos")]
fn with_ns_window(window: &Window, operation: impl FnOnce(*mut objc::runtime::Object)) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return;
    };
    unsafe {
        use objc::{msg_send, sel, sel_impl};
        let view = handle.ns_view.as_ptr().cast::<objc::runtime::Object>();
        let native: *mut objc::runtime::Object = msg_send![view, window];
        if !native.is_null() {
            operation(native);
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn hide(_window: &gpui::Window) {}

#[cfg(not(target_os = "macos"))]
pub fn show(_window: &gpui::Window) {}

/// The current cursor location in AppKit's global display coordinate space.
/// It is sampled only while placing the transient Launcher and is never
/// retained or monitored.
#[cfg(target_os = "macos")]
pub fn mouse_location() -> Option<(f32, f32)> {
    use objc2_app_kit::NSEvent;

    let point = unsafe { NSEvent::mouseLocation() };
    Some((point.x as f32, point.y as f32))
}

#[cfg(not(target_os = "macos"))]
pub fn mouse_location() -> Option<(f32, f32)> {
    None
}
