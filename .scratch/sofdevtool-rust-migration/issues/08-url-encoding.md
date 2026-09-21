# 08: Encode path segments and query values

Type: task
Status: ready-for-agent
Blocked by: 04

Parent: [Rust and GPUI migration specification](../spec.md)

**What to build:** Convert URL text through separately named Path Segment and Query Value modes, copy the output and restore it from History.

## Acceptance criteria

- [ ] Preserve only the contract-defined characters in each mode; encode delimiters and literal percent as appropriate using uppercase UTF-8 percent bytes.
- [ ] Use %20 for spaces; decode plus literally and reject malformed triplets or invalid UTF-8.
- [ ] Deliver the shared Utility vertical-slice contract including exact snapshots and Registry metadata.
- [ ] Test reserved delimiters, combining marks, non-Latin text, complex emoji and stale results with independent fixtures.
- [ ] Run the relevant documented Rust gate and focused native scenarios for this slice; record actual commands, host and results without presenting compilation as runtime evidence.

## Completion evidence

Pending implementation. The parent specification supplies shared behavior; this ticket makes no completion or runtime claim.
