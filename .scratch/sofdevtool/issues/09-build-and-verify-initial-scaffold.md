Type: task
Status: resolved
Assignee: Codex
Blocked by: 08

## Question

Create and verify the agreed SofDevTool macOS scaffold: a compiling app shell with the validated navigation and Utility Launcher, Settings and history controls, required shortcuts and native integrations, tests, and working JSON, Base64, complete Identifier Generator (UUID, ULID, and KSUID), and Random String Utilities. Follow the [product and architecture handoff](../implementation-handoff.md). Record build and test evidence and surface any architectural decision invalidated by implementation rather than silently improvising around it.

## Comments

### Implementation checkpoint — 2026-08-28

The native scaffold is implemented and the authoritative `scripts/verify --full` gate passes on Apple Silicon macOS 26.2: strict format lint, Debug and Release builds, 16 unit/integration tests, and one focused end-to-end UI flow. The built app was also launched and inspected through the macOS accessibility hierarchy. No resolved architectural decision was invalidated.

Detailed commands, result-bundle paths, environment versions, and the remaining owner-only release checks are recorded in [the scaffold verification ledger](../scaffold-verification.md). This ticket remains claimed until the macOS 26 global-shortcut/Spaces, lifecycle, VoiceOver, and display-setting checklist is completed. macOS 14 and 15 runtime checks remain explicitly pending as agreed.

### Launcher fix checkpoint — 2026-08-28

The owner's manual check exposed a real background-activation defect: the nonactivating Launcher panel used `hidesOnDeactivate`, so a registered global shortcut ordered it forward and macOS immediately hid it while another application remained active. A new focused regression test went red before the AppKit configuration change and green afterward. The original host reproduction now opens and dismisses the Launcher with Finder still frontmost, and the post-fix `scripts/verify --full` gate passes with 16 unit/integration tests and one UI test. The owner then confirmed physical shortcut activation both normally and over a full-screen app on macOS 26.2; the remaining manual items stay listed in the verification ledger.

### Launcher pointer-selection fix checkpoint — 2026-08-28

The owner's next manual check found that clicking a Utility only changed `List` selection and never invoked the Launcher's existing open command. The focused UI flow now clicks Identifier Generator from a Base64 workspace and requires both panel dismissal and the Identifier workspace. It failed before pointer activation was wired to the shared command path and passes afterward. A clean `scripts/verify --full` run passes with 16 unit/integration tests and one UI test; owner confirmation of the fixed pointer behavior remains pending.

### Crash-audit checkpoint — 2026-08-28

The only SofDevTool diagnostic report after the owner's crash observation belongs to the duplicate Xcode-debug PID deliberately terminated while isolating the clean post-fix test run. It shows an idle AppKit event loop, debugger instrumentation, and no product-code crashing frame; no newer SofDevTool report appeared. Computer Use could inspect the live app, but its pointer helper crashed independently while SofDevTool remained alive. The actual pointer flow was therefore exercised through five unretried XCUITest iterations instead: all five opened the Launcher, clicked Identifier Generator, verified panel dismissal and workspace switching, and passed without producing an app crash report. The result bundle is `.artifacts/TestResults/UI-launcher-stability-5x.xcresult`; owner confirmation and the other manual checklist items remain pending.

### AFK checklist checkpoint — 2026-08-28

Computer Use continued to inspect the live accessibility hierarchy but its native service crashed on both element and coordinate clicks while SofDevTool remained alive. Native UI automation now verifies pointer selection plus Escape and click-away dismissal. Isolated tests verify failed shortcut registration preserves the last working configuration and saved configuration is restored; a live Settings probe also recorded `⌃⌥L`, after which the owner's `⌃⌥Space` preference was restored. Closing the Workbench left the process running, but reactivation from Finder through XCUIApplication did not recreate it in two probes. A real Dock click could not be attempted through the failing Computer Use service, so lifecycle reopen is unresolved rather than passed. The updated authoritative full gate passes with 18 unit/integration tests and two UI tests. VoiceOver, alternate display settings, visible conflict-error confirmation, physical post-relaunch shortcut activation, Dock reopen, and Quit remain pending.

### Owner functional confirmation — 2026-08-30

The owner confirmed the complete Launcher pointer, Escape, and click-away flow; closing the Workbench while the process remains active; global-shortcut activation without a main window; Dock recreation of the Workbench with process-lifetime state; and Command-Q termination. A separate keyboard-only pass confirmed Launcher search, arrow selection, Return activation, forward and reverse Tab navigation, button activation, and Escape. VoiceOver-specific verification is waived because the owner does not use VoiceOver. The remaining macOS 26 release checks are visible shortcut-conflict presentation with physical post-relaunch activation and the alternate display-setting inspection recorded in the verification ledger.

### Resolution — 2026-08-30

The owner completed the remaining shortcut/error, persistence, Increase Contrast, Reduce Transparency, larger-text, and non-default-accent checks successfully. Together with the authoritative full gate (18 unit/integration tests and two UI tests), repeated Launcher stability run, crash audit, and prior functional confirmations, this completes the agreed macOS 26 scaffold verification. VoiceOver-specific use is explicitly waived for this private owner-only app; ordinary keyboard navigation passed. macOS 14 and 15 remain honestly recorded as deployment-build evidence without runtime-host verification and do not block this scaffold ticket.
