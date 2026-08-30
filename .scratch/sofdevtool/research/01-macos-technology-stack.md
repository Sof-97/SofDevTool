# SofDevTool macOS technology stack

Research checked against primary sources on 2026-08-20.

## Decision

Build SofDevTool as a native **Swift 6 macOS application using SwiftUI for the application and Utility UI, with narrow AppKit adapters for macOS capabilities SwiftUI does not expose well**. Use Xcode 26.6, set the deployment target to **macOS 14 Sonoma**, and build only for Apple Silicon. This covers the current and previous two major macOS generations: Tahoe 26, Sequoia 15, and Sonoma 14. Apple lists all three as current Mac operating-system lines, and Xcode 26.6 can deploy as far back as macOS 11. ([Apple macOS versions](https://support.apple.com/en-us/109033), [Xcode system requirements](https://developer.apple.com/xcode/system-requirements))

Keep the first scaffold deliberately small:

- SwiftUI `WindowGroup` for the ordinary main window, `Settings` for preferences, native commands and in-app shortcuts, and `NavigationSplitView`-style navigation.
- Foundation for transformations, encoding, file access, and preferences; `NSPasteboard` behind a small clipboard service; an AppKit window-activation adapter for the launcher.
- [`KeyboardShortcuts`](https://github.com/sindresorhus/KeyboardShortcuts) as the only initial non-Apple runtime package, wrapped behind an app-owned `GlobalShortcutService`. It provides a SwiftUI recorder, conflict feedback, `UserDefaults` persistence, and global hotkeys without an Accessibility permission dialog. Raw `NSEvent` global key monitoring is a worse fit because Apple says key monitoring requires Accessibility trust. ([Apple `NSEvent` global monitor](https://developer.apple.com/documentation/appkit/nsevent/addglobalmonitorforevents(matching:handler:)))
- Swift Testing for unit and integration tests, plus XCTest/XCUIAutomation for a small number of native UI flows. Both are integrated into Xcode; Apple explicitly recommends the combination. ([Apple testing guidance](https://developer.apple.com/documentation/xcode/testing), [Swift Testing](https://developer.apple.com/documentation/testing/))
- `UserDefaults`/`@AppStorage` for preferences. Put bounded Utility History behind an app-owned repository protocol so the later history decision may choose an application-support JSON file or SwiftData without coupling Utilities to storage. Apple documents `Settings` with `@AppStorage`, and SwiftData offers local model storage and migrations if the data shape later warrants it. ([SwiftUI Settings](https://developer.apple.com/documentation/swiftui/settings), [SwiftData ModelContainer](https://developer.apple.com/documentation/swiftdata/modelcontainer))
- Preserve, but do not implement in the scaffold, an optional `MenuBarExtra` scene and a clipboard-classification service. SwiftUI directly supports a persistent menu-bar control, including a window-style presentation. ([SwiftUI MenuBarExtra](https://developer.apple.com/documentation/swiftui/menubarextra))

The global launcher should be a normal app window brought forward by the shortcut, not a second web-style overlay architecture. The shortcut and AppKit activation details stay behind adapters so they can be tested or replaced without touching Utility modules.

## Why this wins

### Native result

SwiftUI supplies macOS window, settings, command, menu, keyboard-navigation, accessibility, appearance, and control behavior without recreating them in HTML and CSS. Apple documents that a `WindowGroup` automatically supplies standard macOS window commands, while app scenes can add commands and shortcuts. Selective AppKit access covers `NSPasteboard`, activation/window behavior, and other Mac-only gaps without turning the UI into an AppKit codebase. ([SwiftUI menu and command behavior](https://developer.apple.com/documentation/swiftui/building-and-customizing-the-menu-bar-with-swiftui), [WindowGroup](https://developer.apple.com/documentation/swiftui/windowgroup))

This is the strongest match for a Mac-only personal toolbox where native polish outranks cross-platform reuse.

### Local-first capability

The required Utilities are pure deterministic transformations and fit ordinary Swift/Foundation code. Preferences, history, clipboard access, shortcut registration, and menu-bar UI all have local macOS implementations; no account, server, paid API, embedded browser engine, or cloud database is needed.

Smart clipboard recommendations remain deferred for a good reason: clipboard reads need a separately designed privacy interaction. The stack supports that work later through `NSPasteboard`, while the scaffold can limit itself to explicit Paste and Copy actions.

### Dependency and maintenance footprint

The UI runtime and platform frameworks ship with macOS. The proposed scaffold therefore has one implementation language and one focused package dependency rather than a frontend runtime, package-manager graph, IPC bridge, and second systems language. The application still needs normal macOS compatibility testing on 14, 15, and 26, but it does not need to track an embedded Chromium release line.

### Coding-agent reliability

This is an inference from the architecture, not a claim made by Apple: agents should work reliably when Utility logic is pure Swift, each Utility conforms to one small source-level interface, platform integrations sit behind protocols, and every change is checked with `xcodebuild` and deterministic tests. Swift's compile-time type checking gives fast, precise feedback. The main friction is Xcode project metadata; minimize target and build-setting churn, use Swift Package Manager for the single dependency, and keep most new work to ordinary `.swift` files.

## Comparison

| Criterion | SwiftUI + AppKit | Tauri 2 + TypeScript/Rust | Electron + TypeScript | Mac Catalyst |
|---|---|---|---|---|
| macOS UI | Native controls and behavior | Web UI in system WebView | Web UI in bundled Chromium | UIKit-derived Mac UI |
| Native integrations | Direct Apple APIs; one hotkey package recommended | Official plugins for shortcut, clipboard, store, and tray | Built-in shortcut, clipboard, tray, and filesystem APIs | Partial Mac APIs layered onto an iPad model |
| Runtime/dependencies | Apple frameworks + one focused SPM package | Rust shell, web frontend, IPC/capabilities, plugins | Chromium, V8, Node.js, frontend and main/preload boundary | Apple frameworks, but UIKit/Catalyst constraints |
| Testing | Swift Testing + XCTest UI | Frontend tests, mocked Tauri APIs, Rust tests, WebDriver | Web tests plus Electron process/integration tests | XCTest |
| Agent change surface | One language for almost all work | Two languages plus bridge/configuration | TypeScript-friendly, but renderer/main/preload boundaries | Native, but wrong abstraction for this product |
| Fit | **Best** | Credible runner-up | Capable but oversized | No reason to adopt |

## Why the alternatives lose

### Tauri 2 is the runner-up

Tauri combines a Rust host with HTML rendered in an operating-system WebView. It avoids shipping its own browser runtime, supports tray interfaces, and provides official plugins for global shortcuts, clipboard access, and a persistent store. ([Tauri architecture](https://v2.tauri.app/concept/architecture/), [global shortcut](https://v2.tauri.app/plugin/global-shortcut/), [clipboard manager](https://v2.tauri.app/reference/javascript/clipboard-manager/), [store](https://v2.tauri.app/reference/javascript/store/))

It would be a sound choice if cross-platform delivery or reuse of an existing web product were important. Neither is true here. For SofDevTool, Tauri adds Rust/TypeScript, WebView-to-host message passing, capability configuration, plugin lifecycle, and hand-built Mac styling while still requiring macOS-specific work. Its DOM-based UI may be familiar to agents, but its total architecture has more seams than SwiftUI. It loses on native UI fidelity and whole-project simplicity, not on basic capability.

### Electron is capable but mismatched

Electron has first-party APIs for global shortcuts and tray icons, and excellent web ecosystem ergonomics. However, Electron explicitly bundles Chromium, V8, and Node.js with each app. Its maintainers also require staying current with Electron because that bundle is part of the application's security surface; only the latest three stable Electron releases are officially supported. ([Why Electron](https://www.electronjs.org/docs/latest/why-electron), [Electron security](https://www.electronjs.org/docs/latest/tutorial/security), [release policy](https://www.electronjs.org/docs/latest/tutorial/electron-timelines))

That runtime, multiprocess model, main/preload/renderer boundaries, and rapid Chromium-linked update cadence are unjustified for a small offline Mac-only toolbox. Electron's own keyboard-shortcut guide also records a longstanding macOS non-QWERTY-layout issue, which is especially relevant for a global launcher. ([Electron keyboard shortcuts](https://www.electronjs.org/docs/latest/tutorial/keyboard-shortcuts))

### Mac Catalyst solves a different problem

Apple positions Mac Catalyst as a way to bring an iPad app to macOS. SofDevTool has no iPad product or UIKit codebase to reuse, so Catalyst adds an iPad-shaped abstraction and restrictions without creating value. ([Apple Mac Catalyst](https://developer.apple.com/documentation/uikit/mac-catalyst))

## Zero-budget packaging boundary

The app can be developed, built, tested, and run locally with Xcode at zero cost. Framework choice does not bypass macOS trust rules for sharing downloaded binaries. Apple recommends Developer ID signing and notarization for direct distribution outside the Mac App Store; those trusted distribution capabilities belong to the paid Apple Developer Program. ([Apple membership comparison](https://developer.apple.com/support/compare-memberships/), [Apple distribution overview](https://developer.apple.com/documentation/technologyoverviews/distribution))

Therefore the initial workflow is local Xcode builds for the owner. Source sharing or manual colleague installation can be revisited after the app proves useful, accepting Gatekeeper friction if no membership is purchased. A paid membership should be reconsidered only when smooth colleague distribution becomes a real need; it is not required for the scaffold.

Tauri's documentation confirms the same underlying boundary for its stack: ad-hoc signing is possible on Apple Silicon, but it does not remove Privacy & Security whitelisting, and a free account cannot notarize the application. ([Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/))

## Consequences for later decisions

- The UI prototype should use native SwiftUI patterns rather than HTML/CSS.
- The native-integration decision should define thin adapters for global shortcut registration, app activation, clipboard operations, and optional menu-bar presence.
- The Utility interface should keep transformation logic independent of SwiftUI and persistence.
- The verification workflow should make command-line `xcodebuild` build and test commands authoritative so agents can validate without relying on Xcode's GUI.
- Menu-bar UI and smart clipboard recommendations stay out of the scaffold, but their service and scene seams must remain possible.

