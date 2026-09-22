# 17: Convert text case with inspectable words

Type: task
Status: resolved
Blocked by: 04

Parent: [Rust and GPUI migration specification](../spec.md)

**What to build:** Inspect detected words and convert developer text through the existing nine case styles while preserving Unicode.

## Acceptance criteria

- [ ] Use one deterministic locale-independent segmentation for camel/Pascal/snake/screaming-snake/kebab/title/sentence/lower/upper modes.
- [ ] Show Detected Words; pin acronym, digit, punctuation and mixed-separator behavior including HTTPServer and version2Value.
- [ ] Preserve complete graphemes and valid Unicode, including combining marks and complex emoji.
- [ ] Deliver complete Utility integration and fixtures for segmentation, empty/invalid boundaries, modes and exact restore.
- [ ] Run the relevant documented Rust gate and focused native scenarios for this slice; record actual commands, host and results without presenting compilation as runtime evidence.

## Completion evidence

Resolved. Nine styles share one deterministic, locale-independent segmentation
that preserves complete graphemes and Unicode; Detected Words are shown; acronym
and digit behavior is pinned (`HTTPServer`, `version2Value`). Full
Registry/session/Clipboard/snapshot/History flow with exact restore. Evidence:
[`rust/docs/evidence/ticket-17.md`](../../../rust/docs/evidence/ticket-17.md).
