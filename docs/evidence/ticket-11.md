# Ticket 11 — ULID and KSUID

## Scope

ULID and KSUID are added as formats (not UUID versions) to the Identifier
Generator. ULID supports random and process-local monotonic generation,
canonical uppercase Crockford base32 and timestamp inspection. KSUID supports
cryptographic generation, case-sensitive validation, timestamp inspection and
explicit ordered batches capped at 65,536. UUID workflows, per-item Copy,
Copy All and exact History restoration are retained across all formats.

## Verification

- Core tests cover fixed-time boundaries and 48-bit overflow, monotonic carry
  within a batch and across deliberate batches, independent bigint encoder
  oracles, strict validation, the full 65,536 ordered batch, and ULID/KSUID
  snapshot round-trips with deterministic fixtures. Generation and the clock
  use the existing injected `IdentifierSource` seam.
- Gate: `rust/scripts/verify --full` exit 0.
- Native smoke: the format selector and format-relevant controls render.

## Limits

macOS 14/15 runtime unverified. Monotonic state is process-local and resets on
relaunch by design. Exhaustive native interaction evidence is deferred to
ticket 22.
