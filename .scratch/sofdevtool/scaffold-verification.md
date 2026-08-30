# Scaffold verification — 2026-08-28

This ledger separates automated build/test evidence, an interactive host inspection, and checks that still require owner confirmation. It does not claim macOS 14 or 15 runtime support from a deployment-target build.

## Environment

- Host: Apple Silicon macOS 26.2 (25C56), arm64
- Xcode: 26.6 (17F113)
- Swift: 6.3.3
- SDK used by Xcode: macOS 26.5
- Deployment target exercised by the gates: macOS 14.0

## Automated evidence

Authoritative command:

```sh
scripts/verify --full
```

Result: passed on the host above. The gate completed strict `swift-format` linting, an arm64 Debug build, the non-UI suite, an arm64 Release build, and the focused UI suite.

- Unit/integration result: `.artifacts/TestResults/Unit-20260828T185639Z-2384.xcresult` — 18 passed, 0 failed, 0 skipped.
- UI result: `.artifacts/TestResults/UI-20260828T185639Z-2384.xcresult` — 2 passed, 0 failed, 0 skipped.
- The UI flows launched the app, searched the Library, opened Base64, switched to Identifier Generator through a pointer click in the Launcher, verified Escape and click-away dismissal, and opened Settings.
- `xcodebuild -project SofDevTool.xcodeproj -list` reports exactly three targets: `SofDevTool`, `SofDevToolTests`, and `SofDevToolUITests`, with the shared `SofDevTool` scheme.
- `file .artifacts/DerivedData/Build/Products/Release/SofDevTool.app/Contents/MacOS/SofDevTool` reports a 64-bit arm64 Mach-O executable.

The Xcode installation logs an out-of-date CoreSimulator warning. The project targets macOS directly; all macOS build and test legs above passed despite that unrelated simulator warning.

## Interactive host inspection

The built app was launched and inspected through the macOS accessibility hierarchy on macOS 26.2. The real dark Workbench rendered with the Library catalog, four representative Utilities, searchable navigation, toolbar destinations, explicit Paste/Clear/Copy controls, workspace content, and History presentation. XCUIAutomation supplied the repeatable interaction evidence listed above.

No settled architecture decision was invalidated during implementation. The scaffold uses Apple frameworks only, so it adds no package dependency or network requirement.

### Launcher regression — 2026-08-28

The owner found that a registered shortcut did not reveal the Launcher from another application. A deterministic host reproduction confirmed that the nonactivating panel was ordered forward while Finder remained active but immediately hidden by `hidesOnDeactivate`. A focused AppKit regression test failed before the correction and passes with the panel allowed to remain visible; click-away remains owned by `windowDidResignKey`.

The original reproduction now passes against the sole fixed app process with Finder still frontmost: the configured shortcut adds the Launcher window and repeating it removes the window. The authoritative full gate above passed after the correction. The owner subsequently confirmed that the physical shortcut works normally and over a full-screen app on macOS 26.2.

### Launcher pointer selection regression — 2026-08-28

The owner found that clicking a Utility in the focused Launcher changed nothing. The Launcher `List` had selection state for keyboard navigation, but pointer activation never invoked the existing command that opens the selected Utility, dismisses the panel, and activates the Workbench. The UI regression clicked Identifier Generator from a Base64 workspace and failed before the correction. It now verifies that the Launcher closes and the Identifier Generator workspace appears. The authoritative full gate above passed after this correction.

### Crash audit and pointer stability — 2026-08-28

The only SofDevTool diagnostic report present after the owner's crash observation is `SofDevTool-2026-08-28-195508.ips`. It belongs to PID 88046 launched from Xcode's DerivedData with debugger instrumentation, and its main thread was idle in the AppKit event loop when it received `EXC_BREAKPOINT`/`SIGTRAP`. That PID is the duplicate debug instance deliberately terminated while isolating the post-fix full-gate run; the report contains no product-code crashing frame. No newer SofDevTool diagnostic report appeared during the checks below.

Computer Use successfully inspected the live Workbench and its accessibility hierarchy, but each attempted pointer action closed its native pipe and produced new `SkyComputerUseService` diagnostic reports while the SofDevTool process remained alive. This is evidence of a Computer Use service failure, not an app crash, so it is not counted as pointer-interaction evidence.

As the actionable fallback, the focused XCUITest was executed for five iterations without retrying failures. All five runs synthesized pointer clicks to open Base64, open the Launcher, choose Identifier Generator, verify that the Launcher dismissed and the Identifier workspace appeared, and open Settings. Result: `.artifacts/TestResults/UI-launcher-stability-5x.xcresult` — 5 passed, 0 failed. No new SofDevTool diagnostic report was produced.

### AFK owner-check automation — 2026-08-28

Computer Use again read the live Workbench and its complete accessibility hierarchy, but both an accessibility-element click and a raw coordinate click crashed `SkyComputerUseService`; SofDevTool remained alive and produced no diagnostic report. The native UI automation fallback produced these results:

- Escape and click-away each dismissed the Launcher in the focused UI flow and are now part of the authoritative full gate.
- Recording an alternate shortcut displayed `⌃⌥L` and wrote that configuration to the app preferences. The original `⌃⌥Space` preference was restored after the probe.
- Isolated controller tests verify that a conflicting registration preserves the previous configuration and does not write the failed candidate, and that a saved shortcut is registered on a new controller instance.
- Closing the Workbench left the process running. Re-activating it from Finder through XCUIApplication did not recreate the window in two probes. Because Computer Use could not perform a real Dock click, Dock reopen remains unresolved rather than passed; this may be either a product lifecycle defect or an XCUIApplication activation limitation.
- Quit-after-reopen could not be reached after that failure. VoiceOver behavior and alternate macOS display settings remain qualitative/manual checks; the accessibility hierarchy inspection alone is not sufficient to pass them.

## Owner release checklist — complete on macOS 26.2

These behaviors are intentionally manual under the accepted verification contract. Complete them on macOS 26 before resolving ticket 09:

- Launcher shortcut and placement: physical activation from another app and over a full-screen Space passed owner verification on macOS 26.2. Synthetic repeated-shortcut dismissal, pointer Utility selection, Escape, and click-away are automated and passed; the owner subsequently confirmed the complete pointer and dismissal flow.
- Shortcut recording, conflict preservation, and saved-configuration restoration have automated coverage. The owner confirmed the visible invalid-shortcut handling and physical post-relaunch shortcut behavior.
- The owner confirmed that closing the main window leaves the process running, the global shortcut still opens the Launcher, the Dock recreates the Workbench with its process-lifetime state, and Command-Q terminates the app.
- The owner confirmed Launcher search, arrow-key selection, Return activation, forward and reverse Tab navigation, button activation, and Escape without VoiceOver. VoiceOver-specific verification is waived because the owner does not use the screen reader; the native accessibility hierarchy remains available as structural evidence.
- The owner inspected the Workbench and Launcher with Increase Contrast, Reduce Transparency, larger text, and a non-default accent color; all passed.

## Supported-version status

- macOS 26.2: automated full gate, functional owner checks, keyboard navigation, shortcut lifecycle, and alternate display-setting checks passed.
- macOS 15: runtime gate and manual checklist pending; no host was available.
- macOS 14: runtime gate and manual checklist pending; the successful macOS 14 deployment-target build is compatibility evidence only.
