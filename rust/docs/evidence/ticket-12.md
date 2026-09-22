# Ticket 12 — random strings

## Scope

Length 20 / count 1 by default, upper/lowercase/digits/safe symbols, ambiguous
exclusion, custom alphabets and saved controls. System cryptographic randomness
with unbiased rejection sampling; entropy is shown only when calculable.
Generation is explicit, and deliberate identical requests record separately via
a generation nonce.

## Verification

- Core tests inject a fixed seeded RNG to assert alphabet/exclusion/bounds,
  entropy calculation and validation errors without probabilistic quality
  assertions.
- Native smoke: Generate produced a 20-character string from a 69-character
  alphabet, reported 122.2 bits of entropy, with per-item Copy and History
  `1/25`.
- Gate: `rust/scripts/verify --full` exit 0.

## Limits

macOS 14/15 runtime unverified. Exhaustive native interaction evidence is
deferred to ticket 22.
