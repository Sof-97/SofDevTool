# Ticket 17 — case conversion

## Scope

Nine styles (camel, Pascal, snake, screaming-snake, kebab, title, sentence,
lower, upper) share one deterministic, locale-independent segmentation. The
evaluation exposes the detected words for display. Acronym/digit boundaries are
pinned: `HTTPServer` → `HTTP`, `Server`; `version2Value` → `version`, `2`,
`Value`.

## Verification

- Core unit tests cover segmentation, all nine styles, acronym/digit cases,
  empty/invalid boundaries, Unicode (combining marks, ZWJ/skin-tone/regional
  emoji) and exact snapshot round-trip.
- Native smoke: `HTTPServer version2Value` in camelCase → `httpServerVersion2Value`
  with Detected Words `HTTP · Server · version · 2 · Value`, and History
  recorded `1/25`.
- Gate: `rust/scripts/verify --full` exit 0.

## Limits

macOS 14/15 runtime unverified. The segmentation is a self-contained UAX #29
subset because `unicode-segmentation` is not a dependency of the core crate; the
orchestrator added it afterwards and the seam can be swapped without changing
the tested behavior. A lone combining-mark/format cluster is treated as a
separator rather than a symbol word (rare divergence, no effect on ordinary
developer text). Exhaustive native interaction evidence is deferred to ticket 22.
