# 03: Open Utilities through the macOS Launcher

Type: task
Status: claimed
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

Pending implementation. The parent specification supplies shared behavior; this ticket makes no completion or runtime claim.
