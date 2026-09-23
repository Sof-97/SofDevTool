# Ticket 14 — timestamps

## Scope

Auto/manual Unix seconds and milliseconds, ISO 8601 and named-zone local time.
Digit-count inference is exact: at most ten digits mean seconds, exactly
thirteen mean milliseconds, eleven/twelve require manual selection, and
fractional seconds require explicit seconds mode. ISO Auto requires an offset or
`Z`; local time requires a named zone; repeated and nonexistent DST wall times
are rejected. `Now` uses an injectable clock, and restore never reads a clock.

## Verification

- Core tests cover negatives, fractional and boundary values, digit-count
  inference, DST gaps/folds (`America/New_York`), known zones (`Asia/Kolkata`),
  ISO offset/Z requirements and exact snapshot round-trip. The instant is stored
  as `{seconds, nanoseconds}` so a captured `Now` is self-contained.
- Gate: `rust/scripts/verify --full` exit 0.
- Native smoke: the workspace renders its mode, zone and representation
  controls.

## Limits

macOS 14/15 runtime unverified. A `Now` instant is rendered into its ISO input
at millisecond precision (matching the Swift baseline) while `instant` retains
nanoseconds for typed fractional input. Exhaustive native interaction evidence
is deferred to ticket 22.
