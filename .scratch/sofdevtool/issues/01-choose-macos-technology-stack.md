Type: research
Status: resolved

## Question

Which zero-cost application stack should SofDevTool use to deliver a polished, local-only macOS experience on Apple Silicon across the current and previous two macOS generations? Compare credible options using primary sources, including SwiftUI with selective AppKit integration and relevant web-based shells. Evaluate native UI quality, global shortcuts, optional menu-bar support, clipboard integration, local persistence, testing, packaging without a paid developer membership, dependency footprint, long-term maintenance, and how reliably coding agents can navigate and change the project. Recommend one stack and explain why the alternatives lose.

## Comments

## Answer

Use Swift 6 with SwiftUI as the primary macOS UI framework and narrow AppKit adapters for platform gaps. Target Apple Silicon with a macOS 14 deployment target, covering Sonoma 14, Sequoia 15, and Tahoe 26. Use Swift Testing plus XCTest UI tests, native local persistence behind an app-owned repository, and one focused Swift Package Manager dependency (`KeyboardShortcuts`) behind a global-shortcut protocol. Keep `MenuBarExtra` and smart clipboard classification as later seams, not scaffold features.

Tauri 2 is the credible fallback but loses because its TypeScript/Rust/WebView/IPC stack adds seams while requiring web-built macOS UI. Electron adds a bundled Chromium/Node runtime and faster security-update cadence that are disproportionate for this small offline app. Mac Catalyst targets reuse of an iPad app, which SofDevTool does not have.

Local development and owner installation remain zero-cost. Smooth downloaded-binary distribution to colleagues would require revisiting paid Developer ID signing and notarization; no framework removes that Apple trust boundary.

Full evidence and comparison: [SofDevTool macOS technology stack](../research/01-macos-technology-stack.md).
