Type: task
Status: resolved
Blocked by: 09

## What to build

Implement Sample Data as a fully local synthetic-data generator. The owner defines typed fields, chooses JSON or CSV and a row count, generates fictional rows explicitly, and can copy or restore the exact generated result.

Follow the [remaining Utilities implementation specification](../remaining-utilities-implementation-spec.md), [Define first-release Utility contracts](02-define-first-release-utility-contracts.md), and the [implementation handoff](../implementation-handoff.md).

## Acceptance criteria

- [x] The stable Utility Definition appears under Generate with History enabled by default and sample/mock/fake/JSON/CSV search aliases.
- [x] The field editor supports stable field names and the agreed name, email, number, boolean, date, UUID, and enum types, exposing only meaningful bounded options for the selected type.
- [x] Generation is explicit, defaults to ten JSON rows, and enforces one through 1,000 rows and at most 50 fields. Invalid/duplicate names, ranges, enum choices, or schemas are diagnosed before generation.
- [x] JSON output is valid and deterministic in field order. CSV follows a documented dialect, includes a header, and correctly quotes commas, quotes, newlines, Unicode, and empty values.
- [x] Generated names/emails are visibly labelled fictional. The Utility stays offline and does not imply realistic identity, locale, relational, schema-import, or domain-simulation capabilities.
- [x] Randomness, clock/date generation, and UUID creation are injected behind Utility-owned deterministic test seams; production uses system cryptographic randomness where unpredictability is expected.
- [x] Paste/import of schemas, relations between rows or tables, locale packs, and external datasets are absent from this ticket.
- [x] Copy Result, Clear, output format, field editing/reordering, and Generate are keyboard accessible. Each explicit successful generation records exactly one versioned snapshot, including deliberate repeats.
- [x] History preview and explicit restore reproduce field definitions, options, generated rows, and serialized output exactly without regeneration or rerecording.
- [x] Tests cover every field type and option boundary, deterministic generation, JSON validity, CSV escaping, Unicode and complex emoji enum values, invalid schemas, row-count limits, repeated generations, snapshots, stale generation rejection, and registry/search behavior.
- [x] `scripts/verify --full` passes and the implementation evidence names the verified host.

## Answer

Implemented the explicit local generator, typed and reorderable field editor, deterministic entropy seam, UUID v4/date selection, ordered JSON writer, always-quoted RFC 4180-compatible CSV writer, fictional-identity disclosure, schema validation, and exact snapshot restore. Field names use the documented portable ASCII identifier policy. JSON preserves configured field order; CSV uses LF, a header, doubled quotes, and quoted Unicode/newline values.

Focused evidence on macOS 26.2: the Sample Data suite passed 5 tests through the Xcode test target on 2026-08-31, covering all field types and upper/invalid schema bounds. Shared Registry and Utility-owned History preview integration are complete. The authoritative `scripts/verify --full` gate passed 84 unit/contract tests and 3 UI tests on Apple Silicon macOS 26.2 (25C56), Xcode 26.6 (17F113), Apple Swift 6.3.3. macOS 14 and 15 runtime verification remains pending.
