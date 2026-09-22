# Ticket 08 — URL encoding

## Scope

Separate Path Segment and Query Value modes. Path Segment preserves unreserved
characters plus `pchar` (`: @ ! $ & ' ( ) * + , ; =`); Query Value preserves
only unreserved. Literal percent becomes `%25`, spaces become `%20`, and
percent bytes are uppercase UTF-8. Decode is case-insensitive, keeps `+`
literal and rejects incomplete/invalid triplets and non-UTF-8 bytes.

## Verification

- Core unit tests cover reserved delimiters, `pchar`, literal percent, spaces,
  `+` handling, case-insensitive decoding, malformed triplets, non-UTF-8,
  combining marks, non-Latin text, complex emoji and stale-revision rejection.
- Native smoke: the Utility appears in the catalog and renders its input/result
  panes, mode controls, History panel and Clipboard actions.
- Gate: `rust/scripts/verify --full` exit 0.

## Limits

macOS 14/15 runtime unverified. Native interaction was a smoke check; exhaustive
UI interaction evidence is deferred to ticket 22.
