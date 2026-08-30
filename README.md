# SofDevTool

A private, local-only native macOS Developer Toolbox. The representative scaffold contains JSON, Base64, Identifier Generator (UUID, ULID, and KSUID), and Random String Utilities in a dark Workbench UI.

## Requirements

- Apple Silicon Mac
- macOS 14 or newer
- Xcode 26 or a compatible Swift 6 Xcode toolchain

Open `SofDevTool.xcodeproj`, select the shared `SofDevTool` scheme, and run. No account, network service, paid API, or Apple Developer Program membership is required for local development.

Run `scripts/verify` for formatting, Debug build, and unit/integration tests. Run `scripts/verify --full` for the Release build and UI suite as well.
