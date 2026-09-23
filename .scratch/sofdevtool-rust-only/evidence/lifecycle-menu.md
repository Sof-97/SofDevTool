# Standard window menu correction after ticket 12

The native shell had only App/Quit and Edit/Copy/Paste menus, so Cmd-W did not
close the separate Settings window and Ctrl-Cmd-F could not toggle fullscreen
from the keyboard. The Settings close button and the Workbench's retained-window
close callback already worked through AppKit.

`main.rs` now exposes File > Close Window (Cmd-W) and View > Toggle Full Screen
(Ctrl-Cmd-F). Close targets the active window through a deferred AppKit
`performClose:` request, which invokes the existing GPUI should-close callback:
the Workbench hides and keeps its Wry child for Dock reopen, Settings closes
normally, and Launcher closes with its dismissal notification. Fullscreen is
offered to active resizable windows, leaving the transient Launcher panel out.
Explicit Quit remains unchanged.

`native_window.rs` keeps the AppKit operation narrow and schedules it for the
next main-run-loop turn to avoid synchronous GPUI window re-entry during menu
action dispatch. The non-macOS fallback uses GPUI's `remove_window`.

## Verification

- Scoped `rustfmt --edition 2021` on `crates/app/src/main.rs` and
  `crates/app/src/native_window.rs` — passed.
- The coordinator's combined `make verify` compiled these files and reached
  application tests; 101 of 102 passed, with a separate ticket 14 Hashes
  empty-Copy test failure. The combined gate stopped there, so this is not a
  full-gate pass.
- **Native Cmd-W/fullscreen observations remain pending the coordinator's
  running-app check.** No native app was launched by this worker for this
  correction.
