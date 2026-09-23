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
- Coordinator native observations are recorded below; uninstrumented installed
  Release acceptance remains pending. No native app was launched by this worker
  for this correction.

## Native result for e3cb13b and corrected action dispatch

On macOS 26.2 arm64, the fully loaded Settings window ignored synthesized Cmd-W and File > Close Window. Custom View > Toggle Full Screen also had no effect; the native red close button and AppKit's automatic Enter/Exit Full Screen menu items worked. GPUI's global Bubble action listener ran inside the active window's update; immediately updating that window again failed due to re-entry. The implementation now defers the target update until the action effect cycle ends and selects the frontmost window from GPUI's window stack.

The coordinator then built a frozen Debug candidate with opt-in temporary
action tracing. Settings Cmd-W closed the Settings window and returned AX to
the Workbench. Ctrl-Cmd-F entered and exited Workbench fullscreen; native
window controls disappeared and reappeared. The trace showed the deferred
Settings target update succeeded, both Workbench fullscreen updates succeeded,
and Workbench Cmd-W scheduled `performClose:`. The temporary tracing was then
removed from maintained source. CUA could still retrieve the retained
Workbench window after its Cmd-W, so this checkpoint does not claim that
Workbench hiding was visibly confirmed. Final uninstrumented installed Release
acceptance remains with the coordinator.
