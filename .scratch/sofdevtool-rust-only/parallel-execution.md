# Parallel implementation and final review

## Current authorization

The owner's latest instruction requires **GPT-6 Sol for every implementation and
correction**. The root coordinator orchestrates, validates, performs authorized
native acceptance and GitButler integration; GPT-6 Astra performs the independent
final review. Earlier GPT-6 Luna contributions to tickets 07 and 17, and the
segmented control foundation, are historical work already integrated and are
not rewritten as Sol work. The earlier pi/Kimi trial limit and its sequential
worker policy were superseded on 2026-09-23. No remote publication is authorized.

## Worker contract

Read AGENTS.md, CONTEXT.md, the current Rust-only specification and your assigned
ticket. The specification owns the preserved product contracts; retired Swift
source and its earlier handoff are available only in Git history. Your dispatch message owns the exact
file allocation and any temporary coordination boundary.

Work only on the assigned files. All agents share this checkout. Preserve other
workers' edits; request a handoff before touching a file owned by another worker.
Do not run workspace-wide formatting while other workers edit; format owned
files only. Use installed dependency sources and keep pinned versions.

The coordinator approves ticket status changes and alone uses GitButler for
commits and integration. Workers leave their candidate uncommitted and report changed files,
commands/results, acceptance coverage and remaining checks in the assigned
evidence file. Raw Git status has a synthetic index: deletion/untracked pairs
can describe unchanged files. Compare actual bytes with `git show HEAD:path`
when useful; never repair the index.

Use temporary synthetic profiles. Keep personal application data, credentials
and History payloads out of inspection/logs. Native launch and install are
coordinator-owned. The existing Cargo cache is
`CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target` with
`CARGO_NET_OFFLINE=true`. The Cargo workspace is now at the repository root after ticket 20. Coordinate
shared manifest changes. Compile the shared application only at a source-ready
boundary agreed by all active application writers.

## Dependencies and acceptance

The coordinator checks contracts, builds and regression evidence at each
integration boundary so dependent workers can proceed. Ticket status is resolved
only for clauses covered by recorded evidence; unverified native requirements
remain claimed. Ticket 22 separately owns final installed Release acceptance.

The owner explicitly approved ticket 11 candidate `b7e28f0` on 2026-09-23.
Ticket 12 and subsequent Utility rollout use that accepted compact native
direction. Keep later native and final installed Release checks distinct from
that visual approval.

The comprehensive Astra Standards review reported zero findings. Its Specification
review reported two Text Diff findings; GPT-6 Sol corrected both in `df95bd3`,
and Astra's read-only recheck found no remaining regression. The corrected source
passed the full offline gate. The installed Release and remaining native limits
are tracked in [ticket 22 evidence](evidence/ticket-22.md); final branch
cleanliness and local delivery remain coordinator-owned.
