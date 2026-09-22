# Ticket 06 — Base64 encode/decode

## Scope

The first catalog Utility after JSON, built on the shared `Session<U>` and
`HistoryStore`. Encode/Decode, Standard/URL-safe and padding controls with the
existing defaults (Encode, Standard, padded). Encoding is over the exact UTF-8
bytes; decoding validates the selected alphabet, rejects whitespace and
misplaced padding, restores omitted padding and requires valid UTF-8.

## Reproducible native checks

Host: macOS 26.2 (25C56), Apple Silicon arm64; Rust 1.98.1. Debug bundle,
isolated `SOFDEVTOOL_RUST_SUPPORT_ROOT`, synthetic Clipboard input.

1. The sidebar lists JSON, Text Diff and Base64; selecting Base64 shows the
   workspace with Encode/Decode, Standard/URL-safe and Padding controls.
2. Paste `café` in Encode/Standard/Padding-on → `Y2Fmw6k=`; History records
   `1/25`.
3. Toggle Padding off → `Y2Fmw6k`; History records a second entry (`2/25`).
4. Switch to Decode with `café` still in the input → the diagnostic
   `Input contains characters outside the selected Base64 alphabet.` appears
   with no output.

## Verification boundaries

- macOS 14/15 runtime remain unverified.
- URL-safe alphabet, omitted padding, non-UTF-8 decoded bytes, misplaced padding
  and length errors are covered by core unit tests with published vectors.

## Automated verification

`CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target rust/scripts/verify --full`
exit 0: rustfmt, Clippy `-D warnings`, 74 tests (30 core, 16 app, 23 JSON
contract, 5 UI), Debug and Release builds.
