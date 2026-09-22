# Worker assignment: ticket 04

Implement only ticket 04: Propagate History changes through generators, Regex and Text Diff.

You are the implementation worker running through pi with opencode-go/kimi-k3. The coordinating agent owns scheduling, independent review, tracker state and GitButler commits. Complete this assignment, report evidence, and stop; do not start another ticket or delegate more workers.

## Authorization and source precedence

The owner has now authorized a pi/Kimi implementation trial, superseding the earlier documentation-only stopping point for explicitly dispatched tickets. A stored prompt alone is not a dispatch: the coordinator must select this ticket and verify its blockers before running it. The initial trial order is 08, 05, 01, with review between tickets. Later work requires the coordinator's dispatch and respects every blocker. Native preview approval remains mandatory before the visual rollout.

Read, in order:
1. AGENTS.md and CONTEXT.md.
2. .scratch/sofdevtool/implementation-handoff.md for retained product contracts; its Swift-specific stack/source commands are historical.
3. .scratch/sofdevtool-rust-only/spec.md, which supersedes old coexistence, layout, identity and visual decisions and supplies exact limits/testing/ownership rules.
4. .scratch/sofdevtool-rust-only/issues/04-history-generator-comparison-workspaces.md and any blocking ticket evidence.

Recorded blockers: [02: Coordinate JSON and Base64 History with enforced recording recovery](../issues/02-history-coordination-recovery.md).

## Working agreement

- This checkout has one implementation writer. Preserve the coordinator's prompts, tracker and operational notes, plus any pre-existing changes. Change only files needed by this ticket and its evidence.
- Keep Utility requests, results, snapshots and validation strongly typed and Utility-owned. sofui contains reusable UI and component interaction only. Preserve existing working behavior outside the ticket.
- Use installed dependency sources and pinned APIs for GPUI details. Keep the complex editor engine; do not invent APIs or upgrade dependencies to avoid understanding the existing implementation.
- Do not run Git or GitButler mutation commands, commits, pushes, merges, resets, checkout changes or destructive cleanup. The coordinator reviews and commits the candidate. Read-only Git inspection is allowed. This is a GitButler synthetic workspace: raw Git status/diff can show index deletion/untracked pairs for files that are present and already committed. Do not repair the index; inspect actual files or compare their contents with HEAD when necessary.
- Do not change ticket statuses, checked acceptance boxes, the parent specification or other tickets. Report any genuine contract conflict rather than weakening acceptance.
- Use synthetic data and temporary application data roots. Never read personal Application Support data, inspect credentials, dump environment variables or log/export real History. No installation or app launch against real data unless this specific dispatch explicitly requests it.
- Run regression checks at the existing highest useful seam. Demonstrate an old behavior failing where practical, then the corrected behavior passing; avoid tests that merely mirror private implementation.
- Verify the current workspace location before commands. Until ticket 20 relocates it, use rust/scripts/verify and Cargo from rust/. A pre-existing build cache is at /private/tmp/sofdevtool-terra-target; the coordinator supplies CARGO_TARGET_DIR and CARGO_NET_OFFLINE. Keep commands deterministic and preserve full failure evidence. A successful compile does not prove native behavior.
- On a real environment or service blocker, stop and report the exact non-secret error, completed changes and remaining checks. Do not claim success or silently switch model/provider.

## Delivery

Write concise evidence to .scratch/sofdevtool-rust-only/evidence/ticket-04.md: behavior changed, regression/test commands and outcomes, evidence limits, and remaining native checks. Do not claim a source commit for uncommitted work.

Finish with: changed files; acceptance criteria covered or outstanding; exact tests and outcomes; known limitations. Leave a reviewable working tree and stop. The coordinator independently accepts or requests revisions.
