# Ticket 09 — text hashes

## Scope

SHA-256/384/512, SHA-1 and MD5 over the exact UTF-8 bytes, with lowercase hex,
uppercase hex and Base64 output. SHA-1 and MD5 are labelled legacy. Hashing is
explicit (a `Hash` button): untouched input stays neutral, while an explicit
Hash of empty input hashes zero bytes as one valid operation.

## Verification

- Core unit tests use independent NIST/RFC vectors for all five algorithms
  (including empty-message vectors), Unicode byte fidelity (NFC vs NFD
  unchanged), all three representations with a Base64 oracle, deliberate
  repeats recording separately, neutral/invalid never snapshotting, and exact
  snapshot round-trip.
- Native smoke: `abc` → SHA-256 `ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad`
  (correct NIST vector), legacy labels shown, History recorded `1/25`.
- Gate: `rust/scripts/verify --full` exit 0.

## Limits

macOS 14/15 runtime unverified. `Invalid` exists for contract uniformity but
hashing a Rust `String` cannot fail; it is exercised only by a unit test.
Exhaustive native interaction evidence is deferred to ticket 22.
