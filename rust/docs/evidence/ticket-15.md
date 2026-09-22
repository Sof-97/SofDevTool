# Ticket 15 — JWT decoder

## Scope

Splits a token on `.`, requires exactly three segments, decodes Base64URL +
UTF-8 JSON for the header and payload with separate segment diagnostics, and
keeps the signature opaque. A prominent notice states that no signature, claim
or trust verification is performed; there is no key input or verification
action.

## Verification

- Core tests cover a published token, malformed segment counts, invalid
  Base64URL, invalid JSON, non-UTF-8, Unicode and exact snapshot round-trip.
  Header/payload are pretty-printed with keys sorted recursively, so the output
  is deterministic even though other workspace crates enable `serde_json`'s
  `preserve_order` feature (a real ordering bug the full-workspace gate caught).
- History is off by default: the Registry declares `jwt-decoder` with
  `history_enabled_by_default: false`, the shared `HistoryRecorder` policy gates
  it, and the workspace exposes an explicit "Record to History" opt-in.
- Gate: `rust/scripts/verify --full` exit 0.

## Limits

macOS 14/15 runtime unverified. Exhaustive native interaction evidence is
deferred to ticket 22.
