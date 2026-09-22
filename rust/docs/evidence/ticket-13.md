# Ticket 13 — YAML/JSON conversion

## Scope

One YAML 1.2 Core-oriented document converted to or from JSON, with anchors and
aliases, string-keyed mappings and JSON fidelity checks. Multiple documents,
duplicate mapping keys, non-string mapping keys, unsupported tags and
nonfinite/unrepresentable JSON values are rejected. Date-looking and YAML 1.1
boolean-looking scalars stay strings. The loss of comments, formatting and alias
identity is disclosed.

## Parser decision

The high-level `saphyr` data model silently collapses duplicate mapping keys and
discards tags, which this Utility must reject. The implementation therefore uses
`saphyr-parser` 0.0.3 — the event-level parser from the same maintained project,
already present transitively and now pinned explicitly. Both are MIT OR
Apache-2.0, pure Rust and fully offline. A named `YamlJsonResourcePolicy` bounds
input (1 MiB), nesting (128), alias expansions (10,000) and expanded nodes
(1,000,000); over-limit input yields a diagnostic and no output.

## Verification

- Core tests cover both directions, anchors/aliases, duplicate and non-string
  keys, multiple documents, unsupported tags, nonfinite values, YAML 1.1
  booleans and dates as strings, Unicode, over-limit refusal and exact snapshot
  round-trip.
- Native smoke: a YAML document with `café`, a nested list and a nested mapping
  converted to JSON with the fidelity notice shown and History `1/25`.
- Gate: `rust/scripts/verify --full` exit 0.

## Limits

macOS 14/15 runtime unverified. JSON diagnostics carry a message only; YAML
tag/key/nonfinite errors carry parser line/column. Plain decimals are
canonicalized to a bounded plain form (matching the Swift oracle) and
expansions beyond 4,096 digits are refused. Exhaustive native interaction
evidence is deferred to ticket 22.
