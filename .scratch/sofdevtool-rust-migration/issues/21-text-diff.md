# 21: Complete Text Diff sessions and History

Type: task
Status: ready-for-agent
Blocked by: 04

Parent: [Rust and GPUI migration specification](../spec.md)

**What to build:** Compare user-entered text in split or unified mode and restore the exact comparison through normal Utility History.

## Acceptance criteria

- [ ] Integrate the proven local web renderer with real Registry metadata, retained session, controls, explicit Clipboard and versioned snapshots.
- [ ] Show loading/ready/error states; accept current-request readiness once, reject stale callbacks and record only a completed current valid comparison.
- [ ] Preserve UTF-8 correctness and disclosed whole-line emoji fallback unless equivalent safer highlighting is demonstrated; retain both source texts on restore.
- [ ] Verify split/unified mode, selection/copy, focus, scroll, resize and offline operation with all current shell surfaces.
- [ ] Test snapshot and callback behavior plus real renderer regressions; include the complete bundled dependency notices and packaged resources.
- [ ] Run the relevant documented Rust gate and focused native scenarios for this slice; record actual commands, host and results without presenting compilation as runtime evidence.

## Completion evidence

Pending implementation. The parent specification supplies shared behavior; this ticket makes no completion or runtime claim.
