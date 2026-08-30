Type: grilling
Status: resolved
Assignee: Codex
Blocked by: 01, 03

## Question

Which macOS integrations belong in the initial SofDevTool scaffold, and what are their precise behaviors? The configurable global shortcut and native system appearance are required candidates; menu-bar actions are optional. Smart clipboard recommendations are explicitly deferred, but the decision must preserve an appropriate seam without implementing speculative infrastructure.

## Comments

### Native integration decisions — round 1, confirmed 2026-08-21

- SofDevTool remains running after its main window closes so the global Utility Launcher continues to work. Only an explicit Quit terminates it. It does not launch automatically at login in the initial scaffold.
- The Utility Launcher appears centered on the active display. Escape, clicking away, or repeating the configured global shortcut dismisses it. Choosing a Utility dismisses the Launcher, activates the main window, and opens that Utility; cancellation leaves the previously active app untouched.
- The global shortcut defaults to Control-Option-Space and is editable in Settings through native shortcut recording. Incomplete shortcuts are rejected. A failed or conflicting registration retains the last working shortcut and produces a clear inline error rather than silently leaving the Launcher unreachable.
- SofDevTool uses a dark-only appearance rather than following the system or offering an appearance selector. Its precise accessibility relationship remains to be confirmed.
- The initial scaffold has normal Dock presence and standard macOS application menus, but no persistent menu-bar status item.
- Login items, Services, Share extensions, URL schemes, notifications, and Touch Bar integration are excluded from the initial scaffold.

### Native integration decisions — round 2, confirmed 2026-08-21

- Clipboard access remains explicitly user-triggered through Copy and Paste. The initial scaffold has no clipboard monitoring, classification, permission prompts, or recommendation UI. Native clipboard access sits behind a narrow adapter that a later, separately designed opt-in recommendation source could reuse.
- The application forces Dark appearance and offers no appearance selector, while continuing to respect macOS accessibility settings including Increase Contrast, Reduce Transparency, accent color, and text scaling.
- Clicking the Dock icon after closing the main window restores its previous size and position and reopens the last selected Utility. Utility inputs and results are restored only where the separately defined Utility History policy permits it.
- The Utility Launcher appears on the currently active Space and display, including over a full-screen application, without first switching to SofDevTool's existing window. Selecting a Utility then uses normal macOS activation behavior to reveal the main window.

## Answer

The initial scaffold is a normal Dock-based macOS application that remains running after its last window closes. It quits only through an explicit Quit and does not launch at login. Dock activation restores the main window's previous size and position and its last selected Utility; restoration of Utility payloads remains governed by Utility History.

The Utility Launcher is available while the application is running through a configurable global shortcut, initially Control-Option-Space. It appears centered on the active display and current Space, including over full-screen applications. Escape, clicking away, or repeating the shortcut dismisses it without disturbing the current application. Choosing a Utility dismisses the Launcher, activates the main window using normal macOS behavior, and opens the selection. Shortcut editing uses native recording in Settings; invalid input or registration conflicts retain the last working shortcut and show an inline error.

SofDevTool always uses Dark appearance and has no appearance selector, but it honors macOS accessibility settings such as increased contrast, reduced transparency, accent color, and text scaling. It retains standard macOS application menus and Dock presence but adds no persistent menu-bar status item.

Clipboard interaction is explicit: only user-triggered Copy and Paste access the pasteboard. A narrow native clipboard adapter preserves a future integration point, but the scaffold contains no background monitoring, smart classification, permission prompt, or recommendation UI.

Launch-at-login, Services, Share extensions, URL schemes, notifications, Touch Bar support, and a menu-bar status item are outside the initial scaffold. Smart clipboard recommendations remain deferred as already recorded on the map.
