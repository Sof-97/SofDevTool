# 03: Open Utilities through the macOS Launcher

Type: task
Status: resolved
Blocked by: 01

Parent: [Rust and GPUI migration specification](../spec.md)

**What to build:** Invoke a configurable global shortcut from another app, find JSON or another registered Utility and open it in the Workbench without disturbing the prior app when dismissing.

## Acceptance criteria

- [ ] Register Control-Option-Space by default and provide native shortcut capture/configuration; invalid or conflicting changes preserve the last working shortcut with an inline error.
- [ ] Use shared Registry search with keyboard selection; Escape, click-away and repeated shortcut dismiss without activating the main window.
- [ ] Verify placement on the active display/current Space and over a full-screen application; selection activates the main window and opens the Utility.
- [ ] Keep the process alive after the last window closes; Dock activation restores window placement and selection; Quit exits normally.
- [ ] Test registration failure via an adapter and record real macOS shortcut/focus/Space/lifecycle evidence. Persist settings only in the Rust namespace.
- [ ] Run the relevant documented Rust gate and focused native scenarios for this slice; record actual commands, host and results without presenting compilation as runtime evidence.

## Completion evidence

Resolved. Control-Option-Space registers by default through Carbon behind the
app-owned `ShortcutRegistrar`; Settings captures and persists a replacement
under the fresh Rust namespace and keeps the working chord on invalid input with
an inline error. The nonactivating Launcher searches the shared Registry with
keyboard selection; Escape, click-away and a repeated shortcut dismiss it
without activating the main window, and it appears over a full-screen app.
Selection activates the Workbench and opens the Utility. Closing the last
window keeps the process alive and reopening restores the same window and
selection. Native scenarios, limits and the gate are recorded in
[`rust/docs/evidence/ticket-03.md`](../../../rust/docs/evidence/ticket-03.md).
macOS 14/15 runtime remain unverified; the 14.0 baseline is unchanged.
