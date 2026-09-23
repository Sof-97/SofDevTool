# Ticket 16 — Rust regex

## Scope

The pinned `regex` crate dialect only, with a visible engine/dialect label.
Look-around and backreferences are rejected with explicit "no ICU emulation"
diagnostics. Supported flags and replacement syntax are documented in the UI.
Named limits bound pattern size (16 KiB), compiled program (1 MiB), text
(1 MiB), template (256 KiB), matches (10,000), capture slots (50,000) and
replacement output (2 MiB); over-limit input is refused whole.

## Verification

- Core tests cover supported syntax, unsupported look-around/backreference
  diagnostics, UTF-8 byte offsets, absent captures, zero-length matches,
  replacement preview with capture references, the documented limits, and exact
  snapshot round-trip. Obsolete debounced revisions are discarded by the shared
  session; the `regex` crate exposes no cooperative cancellation, so no hard
  interruption is claimed.
- Native smoke: the workspace renders the engine label, flags, pattern,
  replacement template, test text and replacement preview without error.
- Gate: `rust/scripts/verify --full` exit 0.

## Limits

macOS 14/15 runtime unverified. Exhaustive native interaction evidence is
deferred to ticket 22.
