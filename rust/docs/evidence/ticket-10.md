# Ticket 10 — Identifier Generator (UUID)

## Scope

UUID v1/v3/v4/v5/v6/v7 with strict validation and normalization, case/hyphen
options and required version-specific inputs (v3/v5 namespace + name). Default
is one lowercase hyphenated v4. No v8. Format is modelled as a separate
dimension so ticket 11 can add ULID/KSUID without changing the UUID paths.

## Verification

- Core tests assert version/variant bits for all six versions, injected-instant
  timestamps for v1/v6/v7 against an independent `uuid`-crate oracle, published
  v3/v5 DNS namespace vectors, case/hyphen options, strict validation failures,
  batch limits, and exact snapshot round-trip without regeneration. Randomness
  and the clock are injected through an `IdentifierSource` seam.
- Native smoke: selecting v4 and pressing Generate produced a valid v4 UUID,
  with per-item Copy and History `1/25`.
- Gate: `rust/scripts/verify --full` exit 0.

## Limits

macOS 14/15 runtime unverified. Ticket 11 owns the ULID/KSUID formats; the
current app intentionally shows no format selector and no inactive placeholders.
Exhaustive native interaction evidence is deferred to ticket 22.
