# 04: Record and restore JSON operations in fresh Rust History

Type: task
Status: ready-for-agent
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

Pending implementation. The parent specification supplies shared behavior; this ticket makes no completion or runtime claim.
