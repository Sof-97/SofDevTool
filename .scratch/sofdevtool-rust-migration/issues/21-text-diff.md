# 21: Complete Text Diff sessions and History

Type: task
Status: resolved
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

Resolved. The renderer is integrated with Registry metadata, explicit Clipboard actions, loading/ready/error states and versioned snapshots; one completed current comparison is recorded once and restores the exact old/new texts without rerunning. See [`rust/docs/evidence/ticket-21.md`](../../../rust/docs/evidence/ticket-21.md).
