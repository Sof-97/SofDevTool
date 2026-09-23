# SofDevTool Rust-only status

The Rust/GPUI application is the sole maintained SofDevTool product. Its fifteen
Utilities are implemented; `sofui` is the separately versioned component
library. The [Rust-only specification](.scratch/sofdevtool-rust-only/spec.md)
owns the current Utility behavior, limits and privacy rules. In particular,
Regex uses the documented Rust regex dialect and bounded evaluation, not the
retired Swift/ICU behavior.

The [approved ticket index](.scratch/sofdevtool-rust-only/ticket-proposal.md)
links all 22 implementation and acceptance tickets. All 22 are resolved:
[ticket 12](.scratch/sofdevtool-rust-only/issues/12-workbench-launcher-settings-redesign.md)
records the approved shell and the owner's manual Launcher/Spaces/Dock pass;
[ticket 22](.scratch/sofdevtool-rust-only/issues/22-release-acceptance-local-delivery.md)
records the installed Release and local delivery. The three physical checks
were reported by the owner after a request to test them, not captured by
Computer Use. Automated gates, coordinator-observed native flows, manual
reports and earlier candidates remain distinct in
[ticket 22 evidence](.scratch/sofdevtool-rust-only/evidence/ticket-22.md).
macOS 14/15 runtime, other display/DPI configurations and full IME composition
remain outside the observed acceptance scope.

Use the root [README](README.md) for maintained commands and
[AGENTS.md](AGENTS.md) for source ownership. The
[operations register](.scratch/sofdevtool-rust-only/operations.md) records
execution history and the latest evidence boundaries. Historical Swift backlog
material remains in Git history rather than in this active checklist.
