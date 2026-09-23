# Ticket 19 — color conversion

## Scope

Bounded sRGB HEX/RGB(A)/HSL(A) input normalized to 8-bit channels, with
deterministic rounding, synchronized HEX/RGB/HSL output and an interactive
channel picker. Unsupported color spaces and syntax are rejected.

## Verification

- Core tests cover shorthand/full hex, integer and percentage RGB, alpha forms,
  HSL round-trips and hue wrap, invalid syntax, and exact snapshot round-trip.
- Native smoke: `#ff0000` produced a red swatch, channel picker R=255/G=0/B=0/A=255,
  synchronized `#ff0000` / `rgb(255 0 0)` / `hsl(0deg 100% 50%)` outputs and
  History `1/25`.
- A duplicate a11y node id crash (repeated "−"/"+" channel-button labels) was
  found natively and fixed by giving `Button` an explicit element-id override.
- Gate: `rust/scripts/verify --full` exit 0.

## Limits

macOS 14/15 runtime unverified. The reusable color controls are not yet added to
the component gallery; ticket 07 owns gallery extension. Exhaustive native
interaction evidence is deferred to ticket 22.
