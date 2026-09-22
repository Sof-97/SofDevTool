# Ticket 18 — whitespace conversion

## Scope

Nine separate named actions (no generic Clean) with LF default, LF/CRLF/CR
targets, tab width 1–8 default 4, tab-stop expansion and leading-indent-only
spaces-to-tabs. Engine semantics match the Swift baseline, including
visual-column dedent and Unicode-whitespace blank-line removal. A tab width
outside 1–8 is a diagnostic with no partial output.

## Verification

- Core unit tests: one per action plus invalid widths, empty neutrality, each
  line ending, lone CR, widths 1/4/8, Unicode whitespace, graphemes, deliberate
  no-op snapshotting and exact snapshot round-trip.
- Native smoke: the Utility appears in the catalog and renders its controls,
  result pane and History panel.
- Gate: `rust/scripts/verify --full` exit 0.

## Limits

macOS 14/15 runtime unverified. Core column counting iterates Unicode scalars
(no `unicode-segmentation` dependency at implementation time); identical to the
baseline for all fixtures and for space/tab indentation prefixes. Exhaustive
native interaction evidence is deferred to ticket 22.
