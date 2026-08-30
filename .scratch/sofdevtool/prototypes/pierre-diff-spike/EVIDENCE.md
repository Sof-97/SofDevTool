# PierreDiffsSwift validation evidence

Checked 2026-08-28 on Apple Silicon macOS 26.2 with Xcode 26.6 and Swift 6.3.3. This is evidence for the disposable prototype, not implementation verification for SofDevTool.

## Provisional recommendation

Do not let SofDevTool import PierreDiffsSwift directly from its Text Diff workspace. If the owner accepts the remaining interaction tradeoffs, contain it inside one deep `TextDiffRenderer` module whose small interface accepts display-ready old/new text, filename, split/unified mode, intraline mode, overflow mode, and readiness/error callbacks. That module owns the Pierre import, UTF-8 bridge correction, complex-emoji fallback, `WKWebView` behavior, bundled notices, and accessibility accommodations. A later native renderer can replace its implementation without changing the Text Diff workspace or domain logic.

Keep ignore-case and ignore-whitespace semantics in the Text Diff domain module, not the renderer. The prototype makes the consequence visible: preprocessing before Pierre means ignored content is normalized in the rendered diff rather than preserving the original glyphs and merely suppressing their change markers. Owner acceptance of that behavior is still required.

Reject Pierre instead if the manual review finds VoiceOver order, keyboard focus/escape, large-diff scrolling, or normalized ignore-mode presentation unacceptable. Those are product-facing constraints that a shallow wrapper cannot repair reliably.

## Reproducible package and build

- `Vendor/PierreDiffsSwift` is a disposable copy of the published `PierreDiffsSwift` `1.2.4` revision `c2249d7890de957a96480711152d90a06fa1222b` with one documented UTF-8 bridge patch.
- `Package.swift` uses that local copy so the corrected prototype remains reproducible. Production should prefer an upstream-fixed exact release or isolate an equivalent maintained fork inside `TextDiffRenderer`.
- Debug and release builds both complete with the macOS 14 deployment floor and no additional Swift Package dependencies.
- GitHub still lists `1.2.4` as the latest release, while the repository's `main` README recommends unreleased `1.3.0`. The `1.2.4` checkout itself contains changelog material named `1.3.0` and bundles `@pierre/diffs` `1.3.5`. Exact revision pinning is therefore mandatory; ordinary semantic-version ranges are not trustworthy enough for this dependency.

## Rendering and offline behavior

- Split and unified views compile through the public `PierreDiffView` interface.
- Word-level intraline changes render in the small Swift sample.
- The Swift implementation builds an `about:blank` document with JavaScript read from `Bundle.module`; the checked Swift source contains no `URLSession`, remote URL load, or network fetch path.
- A 10,000-line, roughly 208 KB-per-side sample rendered successfully while the process ran under `sandbox-exec` with all network access denied.
- The renderer reports ready twice on initial stress launches. This is not harmful in the spike, but production code must treat readiness as idempotent rather than assuming one callback.

## Unicode and emoji correction

- Root cause 1: Swift correctly base64-encoded UTF-8 JSON, but the wrapper passed JavaScript `atob()` output directly to `JSON.parse()`. Because `atob()` returns a byte/binary string rather than decoded UTF-8 text, accented characters and emoji became mojibake.
- Fix 1: convert the binary string to `Uint8Array`, decode it with `TextDecoder`, then parse the decoded JSON. The regression command `node Measurements/unicode-bridge-check.mjs` now preserves `café`, `Straße`, `👩🏽‍💻`, and `👩🏽‍🚀`.
- Root cause 2: Pierre's word-level intraline highlighter places the changing pieces of a multi-scalar emoji into separate styled spans. WebKit then cannot shape a ZWJ sequence such as `👩🏽‍💻` into one glyph.
- Fix 2: ordinary text and single-scalar emoji retain `.wordAlt` intraline highlighting. When either side contains a multi-scalar emoji grapheme—including ZWJ, skin-tone, flag, or keycap sequences—the renderer discloses and uses whole-line highlighting, preserving the glyph.
- Four Swift regression tests cover ordinary text, ZWJ emoji, flags/keycaps, and single-scalar emoji. The final accessibility-tree check found the intact old/new emoji in the rendered HTML, found the fallback disclosure, and found no mojibake.

## Responsiveness observations

Debug-build render-ready measurements from fresh processes:

| Scenario | First ready | Second ready |
| --- | ---: | ---: |
| 10,000 lines, split | 2.001 s | 2.484 s |
| 10,000 lines, unified | 1.888 s | 2.399 s |
| 10,000 lines, split, network denied | 1.849 s | 2.339 s |

These timings show completion without a hang; they do not establish interactive scrolling quality. That remains a hands-on review item.

## Accessibility and keyboard observations

- The macOS accessibility tree exposes the renderer as HTML content with the filename, line counts, individual line numbers, removed/added text, and intraline text fragments. It is not exposed as an opaque bitmap.
- The native split/unified selector, ignore toggles, sample menu, and both text editors have useful roles and labels.
- The bundled syntax highlighter emits a focusable `pre` element, but the package contains no explicit VoiceOver contract or tests. Only an owner-run VoiceOver pass can establish reading order, verbosity, focus entry/exit, and whether navigation becomes trapped in web content.
- The renderer itself is a `WKWebView`; native control keyboard behavior around it does not prove the web content's keyboard behavior.

## Binary and app-size impact

Release artifacts measured locally:

- Minimal SwiftUI baseline executable: 55,752 bytes.
- Prototype executable linked with PierreDiffsSwift: 1,499,648 bytes.
- Main bundled JavaScript: 10,640,796 bytes.
- Lazy edit JavaScript, unused by SofDevTool's read-only renderer: 160,995 bytes but still copied into the resource bundle.
- Approximate incremental executable-plus-JavaScript cost: 12,245,687 bytes (about 12.25 MB decimal), before normal app metadata and other SofDevTool assets.

## WebView and maintenance findings

- The package creates a `WKWebView`, injects the prebuilt JavaScript into an HTML string, and communicates through a `WKScriptMessageHandler`.
- It enables WebKit developer extras unconditionally. A production adapter should not inherit that behavior without review; the package offers no public switch for it.
- Content and renderer options cross a JavaScript bridge, so updates, lifecycle, errors, and stale render completion must remain isolated inside the renderer module.
- The package is young and its release/tag/changelog numbering is internally inconsistent. Upgrades require an exact-pin review, rebuild, accessibility smoke, offline smoke, performance measurement, and license inventory.

## License findings

- The Swift wrapper ships an MIT license.
- Its checked-in JavaScript lockfile identifies bundled runtime code under Apache-2.0, MIT, BSD-3-Clause, and ISC licenses, including `@pierre/diffs` and Pierre theme packages under Apache-2.0.
- The Swift package exposes only its own `LICENSE`; it does not ship a generated third-party notice inventory beside the compiled JavaScript.
- Adoption therefore requires SofDevTool to generate and check in a complete third-party notices artifact from the exact bundle lockfile, including the wrapper MIT notice and every license required by bundled runtime code. Carrying only MIT and Apache-2.0 headings would be incomplete.

## Owner review status

The owner reported that the prototype otherwise seemed fine and requested correction of the emoji rendering. The corrected app is open on the Unicode sample. The ticket remains claimed only until the owner confirms that the final combined emoji appearance is acceptable.

Automated verification after the correction:

- `node Measurements/unicode-bridge-check.mjs` passes.
- `swift test` passes all four emoji policy tests.
- The Unicode sample's rendered accessibility tree contains `café`, `Straße`, `👩🏽‍💻`, `CAFÉ`, `STRASSE`, and `👩🏽‍🚀` and contains no mojibake.
