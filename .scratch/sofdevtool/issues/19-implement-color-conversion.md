Type: task
Status: resolved
Blocked by: 09

## What to build

Implement Color Conversion as a synchronized local sRGB Utility. The owner can enter HEX, RGB/RGBA, or HSL/HSLA, use the native color picker, preserve alpha, and copy deterministic CSS-compatible representations.

Follow the [remaining Utilities implementation specification](../remaining-utilities-implementation-spec.md), [Define first-release Utility contracts](02-define-first-release-utility-contracts.md), and [Decide native integration boundaries](04-decide-native-integration-boundaries.md).

## Acceptance criteria

- [x] The stable Utility Definition appears under Format & Convert with History enabled by default and color/hex/rgb/hsl/css aliases.
- [x] The approved bounded CSS syntax for HEX, RGB/RGBA, and HSL/HSLA is documented; parsed values are validated against explicit finite ranges and converted through one sRGB model.
- [x] Alpha is preserved across every representation. Opaque colors do not acquire accidental transparency, and conversions use a documented deterministic rounding/formatting policy.
- [x] The native macOS color picker and textual inputs remain synchronized without feedback loops or duplicate History entries, and the picker does not introduce wide-gamut claims.
- [x] CSS-compatible outputs are available per supported representation with explicit Copy actions. Wide gamut, contrast analysis, and palette generation remain absent.
- [x] Empty input is neutral; malformed syntax, non-finite values, and out-of-range components show precise diagnostics and clear stale output.
- [x] Preview and explicit restore reproduce the source representation, normalized sRGB value, alpha, picker state, and outputs without rerunning or rerecording.
- [x] Tests use known conversion vectors and tolerances for hue boundaries, achromatic colors, alpha, shorthand/full HEX if supported, rounding, malformed/range input, picker synchronization, Unicode-independent parsing, snapshots, and registry behavior.
- [x] Focused UI evidence covers the native picker synchronization where a domain test cannot prove the integration. `scripts/verify --full` passes and evidence names the verified host.

## Answer

Implemented one quantized 8-bit sRGB model, bounded HEX/RGB(A)/HSL(A) parsing, deterministic modern CSS output, native picker synchronization, per-representation Copy actions, and exact snapshot restore. The formatting policy uses at most four decimal places, trims insignificant zeroes, and was verified to parse each emitted HEX, RGB, and HSL form back to the same normalized color. Opaque output omits alpha; non-opaque output preserves it.

Focused evidence on macOS 26.2: the Color Conversion suite passed 5 tests through the Xcode test target on 2026-08-31. A focused XCUITest proved that text conversion synchronizes the native color well. Shared Registry and Utility-owned History preview integration are complete. The authoritative `scripts/verify --full` gate passed 84 unit/contract tests and 3 UI tests on Apple Silicon macOS 26.2 (25C56), Xcode 26.6 (17F113), Apple Swift 6.3.3. macOS 14 and 15 runtime verification remains pending.
