# 04: Record and restore JSON operations in fresh Rust History

Type: task
Status: resolved
Blocked by: 02, 03

Parent: [Rust and GPUI migration specification](../spec.md)

**What to build:** Use JSON in the proven native shell, complete operations, inspect retained entries and explicitly restore exactly what was captured after relaunch.

## Acceptance criteria

- [ ] Complete the JSON controls and contracts in the parent spec and register its real session, metadata, History preview and exact snapshot restore.
- [ ] Create one new versioned JSON History file per Utility under the isolated Rust namespace with atomic replacement, timestamps, IDs and newest-25 retention.
- [ ] Record one settled valid operation, exclude intermediate/invalid/stale results and never record or rerun on preview or restore.
- [ ] Restore only after confirmation when replacing different nonempty content; newly generated Rust History survives relaunch without reading legacy data.
- [ ] Prove request/result/snapshot behavior, session publication ordering and storage round-trips using independent fixtures, fixed time and temporary directories.
- [ ] Recheck editor, WebView and Launcher coexistence; all three earlier technical proofs must remain working before catalog expansion.
- [ ] Run the relevant documented Rust gate and focused native scenarios for this slice; record actual commands, host and results without presenting compilation as runtime evidence.

## Completion evidence

Resolved. JSON records one settled valid operation through the shared
`Session<U>` into a versioned per-Utility History file with atomic replacement,
stable ids, timestamps and newest-25 retention. Preview and restore never rerun;
restore confirms before replacing a different non-empty session and records
nothing. History survives relaunch in the isolated Rust namespace. A native
crash from a duplicated confirmation focus handle was found and fixed. Native
scenarios, limits and the gate are recorded in
[`rust/docs/evidence/ticket-04.md`](../../../rust/docs/evidence/ticket-04.md).
macOS 14/15 runtime remain unverified.
