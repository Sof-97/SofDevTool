# SofDevTool Rust-only status

The Rust/GPUI application is the sole maintained SofDevTool product. Its fifteen
Utilities are implemented; `sofui` is the separately versioned component
library. The [Rust-only specification](.scratch/sofdevtool-rust-only/spec.md)
owns the current Utility behavior, limits and privacy rules. In particular,
Regex uses the documented Rust regex dialect and bounded evaluation, not the
retired Swift/ICU behavior.

The [approved ticket index](.scratch/sofdevtool-rust-only/ticket-proposal.md)
links all 22 implementation and acceptance tickets. Tickets 01–11 and 13–21
are resolved. [Ticket 12](.scratch/sofdevtool-rust-only/issues/12-workbench-launcher-settings-redesign.md)
and [ticket 22](.scratch/sofdevtool-rust-only/issues/22-release-acceptance-local-delivery.md)
remain claimed for native Launcher/Spaces/Dock acceptance and final clean local
delivery. A physical global shortcut, another Space/full-screen context and a
literal Dock return still need the owner's manual check; synthetic key events
and application-open tests do not establish those behaviors. The installed
Release and automated/native results are separated in
[ticket 22 evidence](.scratch/sofdevtool-rust-only/evidence/ticket-22.md).

Use the root [README](README.md) for maintained commands and
[AGENTS.md](AGENTS.md) for source ownership. The
[operations register](.scratch/sofdevtool-rust-only/operations.md) records
execution history and the latest evidence boundaries. Historical Swift backlog
material remains in Git history rather than in this active checklist.
