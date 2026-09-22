# Parallel implementation and final review

## Current authorization

On 2026-09-23 the owner authorized the coordinator to implement every remaining
ticket using parallel GPT-6 Sol workers, with GPT-6 Luna for simpler assignments,
and to launch GPT-6 Astra for the comprehensive review only after implementation
of all tickets. This supersedes the earlier pi/Kimi trial limit and its sequential
worker policy. Ticket 08 is already integrated at `5748a16`; starting branch head
is `54be475` on `feat/rust-gpui-migration`. No remote publication is authorized.

## Worker contract

Read AGENTS.md, CONTEXT.md, the retained product contracts in the old handoff,
then the new spec and your assigned ticket. The new spec owns Rust-only scope;
Swift-specific instructions are historical. Your dispatch message owns the exact
file allocation and any temporary coordination boundary.

Work only on the assigned files. All agents share this checkout. Preserve other
workers' edits; request a handoff before touching a file owned by another worker.
Do not run workspace-wide formatting while other workers edit; format owned
files only. Use installed dependency sources and keep pinned versions.

The coordinator alone changes ticket status and uses GitButler for commits and
integration. Workers leave their candidate uncommitted and report changed files,
commands/results, acceptance coverage and remaining checks in the assigned
evidence file. Raw Git status has a synthetic index: deletion/untracked pairs
can describe unchanged files. Compare actual bytes with `git show HEAD:path`
when useful; never repair the index.

Use temporary synthetic profiles. Keep personal application data, credentials
and History payloads out of inspection/logs. Native launch and install are
coordinator-owned. The existing Cargo cache is
`CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target` with
`CARGO_NET_OFFLINE=true`. Cargo workspace is `rust/` until the coordinator
serializes relocation in ticket 20. Coordinate shared manifest changes.

## Dependencies and acceptance

The coordinator checks contracts, builds and regression evidence at each
integration boundary so dependent workers can proceed. This is implementation
integration, not the requested final independent reviewer. Keep each implemented
ticket claimed with an implementation/evidence note until final Astra review
and any required native acceptance are complete.

Ticket 11 still requires explicit owner approval of the concrete native preview
before ticket 12 and its visual rollout. Parallel independent work may continue
while that approval is pending. Do not interpret this execution instruction as
approval of an unseen preview.

After every implementation and required native gate, dispatch GPT-6 Astra to
review the complete branch against the current specification and standards.
Address findings with the implementation workers; re-review affected changes.
Resolve tickets honestly and leave a clean committed local branch.
