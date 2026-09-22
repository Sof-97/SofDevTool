# Ticket 20 — sample data

## Scope

Typed reorderable fictional fields (name, email, number, boolean, date, UUID,
enum) with meaningful bounded options; default ten JSON rows; 1–1,000 rows and
at most 50 fields with stable field order; valid JSON and documented RFC 4180
CSV quoting for delimiters, quotes, newlines, Unicode and empty values. UUID
fields reuse the Identifier Generator; randomness and the clock use the shared
injected seam. Names and emails are labelled fictional.

## Verification

- Core tests cover deterministic fixtures, CSV quoting, row/field bounds,
  duplicate/invalid-name validation, the number/date/enum options and exact
  restore without regeneration.
- Gate: `rust/scripts/verify --full` exit 0.
- Native smoke: the field editor, row count, JSON/CSV selector and output
  render.

## Limits

macOS 14/15 runtime unverified. Dates are entered as Unix epoch seconds (no
date-picker component) and number/date draws use modulo reduction like the
Swift baseline, not unbiased rejection; neither is claimed as cryptographic.
Exhaustive native interaction evidence is deferred to ticket 22.
