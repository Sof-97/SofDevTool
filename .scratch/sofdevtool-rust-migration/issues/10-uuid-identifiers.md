# 10: Generate and inspect UUID identifiers

Type: task
Status: ready-for-agent
Blocked by: 04

Parent: [Rust and GPUI migration specification](../spec.md)

**What to build:** Open Identifier Generator, generate or inspect UUIDs with version-specific controls and restore complete batches without regeneration.

## Acceptance criteria

- [ ] Support UUID v1/v3/v4/v5/v6/v7, strict validation, normalization, case/hyphen controls and required version-specific inputs; do not add v8.
- [ ] Default to one lowercase hyphenated v4, expose batch generation, per-item Copy and Copy All, and describe collision resistance accurately.
- [ ] Use injected clocks/randomness for deterministic tests and system cryptographic randomness where required; preserve numeric/batch limits from the existing contract.
- [ ] Deliver the Identifier Utility Registry/session/snapshot flow, with tests for version bits, published namespace vectors, options and exact restore.
- [ ] Do not show inactive ULID/KSUID placeholders; the following ticket adds those functioning formats.
- [ ] Run the relevant documented Rust gate and focused native scenarios for this slice; record actual commands, host and results without presenting compilation as runtime evidence.

## Completion evidence

Pending implementation. The parent specification supplies shared behavior; this ticket makes no completion or runtime claim.
