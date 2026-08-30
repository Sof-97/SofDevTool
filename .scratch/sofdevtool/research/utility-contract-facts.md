# Utility contract facts: UUIDs and Pierre diff

Researched 2026-08-21 from standards, Apple documentation/source, and the projects' own repositories.

## UUID types and sortable UUIDs

RFC 9562 defines UUID versions 1, 3, 4, 5, 6, 7, and 8; version 2 remains reserved for DCE Security rather than specified for general generation. The relevant meanings are: v1 Gregorian-time/node, v3 deterministic namespace-and-name using MD5, v4 random/pseudorandom, v5 deterministic namespace-and-name using SHA-1, v6 reordered v1/Gregorian time, v7 Unix-epoch-millisecond time plus random/counter material, and v8 application-specific custom layout. [RFC 9562 version table](https://www.rfc-editor.org/rfc/rfc9562.html#section-4.2)

The standardized sortable choices are v6 and v7. RFC 9562 says both can sort as opaque bytes and are intended to sort lexicographically in their canonical text form. It recommends v7 instead of v1 or v6 where possible; v6 primarily exists for v1-compatible time sources and layouts. [UUIDv6](https://www.rfc-editor.org/rfc/rfc9562.html#section-5.6), [UUIDv7](https://www.rfc-editor.org/rfc/rfc9562.html#section-5.7), [sorting](https://www.rfc-editor.org/rfc/rfc9562.html#section-6.11)

Timestamp layout alone does not guarantee creation-order sorting for multiple UUIDs generated in the same timestamp tick. Batch/concurrent v6 or v7 generation needs an RFC 9562 section 6.2 monotonic strategy and clock-rollback handling. Random fields should come from a CSPRNG. [monotonicity and counters](https://www.rfc-editor.org/rfc/rfc9562.html#section-6.2), [unguessability](https://www.rfc-editor.org/rfc/rfc9562.html#section-6.9)

Recommended first-release generator contract:

- Generate v1, v3, v4, v5, v6, and v7, singly or in batches; make **v4 the ordinary random default** and **v7 the recommended sortable default**.
- For v3/v5, require namespace UUID plus name, with standard DNS, URL, OID, and X.500 namespace presets and a custom namespace. Prefer v5 over legacy v3. [name-based generation](https://www.rfc-editor.org/rfc/rfc9562.html#section-6.5)
- Label v1 as legacy with a privacy warning because its node field can carry a MAC-derived identifier. Label v6 as sortable/v1-compatible and subject to the same node-data concern if generated that way. [UUIDv1](https://www.rfc-editor.org/rfc/rfc9562.html#section-5.1)
- Do not expose a generic v8 button: v8 is custom/experimental, and RFC 9562 says its uniqueness is implementation-specific and must not be assumed. Only add it with a named, defined profile. [UUIDv8](https://www.rfc-editor.org/rfc/rfc9562.html#section-5.8)
- Treat ULID and KSUID as separate identifier families, not UUID versions.
- Validate/inspect canonical UUIDs, including version and variant, and expose Nil/Max constants separately from generated versions.

On Apple platforms, Foundation's `UUID()` is specifically a version-4 generator. `UUID(uuid:)` accepts 16 bytes and `UUID(uuidString:)` parses text, so Foundation can hold/format RFC-compliant v3/v5/v6/v7 values but does not itself provide those generators in its documented public API. Implement them carefully in app code or adopt a vetted dependency. [Apple Foundation UUID](https://developer.apple.com/documentation/foundation/uuid), [Swift Foundation UUID source](https://github.com/swiftlang/swift-foundation/blob/main/Sources/FoundationEssentials/UUID.swift)

For randomness, Apple documents `SystemRandomNumberGenerator` as automatically seeded, thread-safe, and cryptographically secure whenever possible; Security's randomization services provide an explicitly cryptographically secure Apple-platform source. [SystemRandomNumberGenerator](https://developer.apple.com/documentation/swift/systemrandomnumbergenerator), [Security randomization services](https://developer.apple.com/documentation/security/randomization-services)

## “Pierre diff” on Swift/macOS

The likely package is [PierreDiffsSwift](https://github.com/jamesrochabrun/PierreDiffsSwift), a third-party Swift wrapper around the official JavaScript [`@pierre/diffs`](https://github.com/pierrecomputer/pierre/tree/main/packages/diffs). It is usable in a SwiftUI macOS app through `PierreDiffView`, but it is not a native Swift diff engine: it wraps a `WKWebView` and ships prebuilt JavaScript/Shiki resources. [wrapper README](https://github.com/jamesrochabrun/PierreDiffsSwift#readme)

Fit for the requested text-diff Utility is strong at the rendering layer:

- Accepts old/new strings and a filename.
- Provides split and unified modes, word/character/no intraline highlighting, line numbers, hunk controls, light/dark themes, wrapping/scrolling, and large-file tokenization limits.
- SwiftUI API is macOS-only, requiring macOS 14+, Swift 6, and Xcode 16. Its package manifest declares one `PierreDiffsSwift` library product, a macOS 14 floor, bundled resources, and no SwiftPM package dependencies. [README/API](https://github.com/jamesrochabrun/PierreDiffsSwift#api-reference), [Package.swift](https://github.com/jamesrochabrun/PierreDiffsSwift/blob/main/Package.swift)
- SwiftPM installation is by Git URL. However, repository metadata is inconsistent as of this research date: the README recommends `from: "1.3.0"`, while GitHub marks `1.2.4` as the latest release and the changelog calls the editing work `1.3.0`. Pin and compile a specific resolved tag in a small spike before committing the product contract. [installation](https://github.com/jamesrochabrun/PierreDiffsSwift#installation), [latest release](https://github.com/jamesrochabrun/PierreDiffsSwift/releases/tag/1.2.4), [changelog](https://github.com/jamesrochabrun/PierreDiffsSwift/blob/main/CHANGELOG.md)

Licensing needs two notices: the Swift wrapper is MIT, while the bundled upstream `@pierre/diffs` package declares Apache-2.0. The wrapper also pins and bundles Shiki and related JavaScript, so the app must audit and carry all transitive bundled-resource notices rather than relying only on SwiftPM's dependency list. [wrapper license](https://github.com/jamesrochabrun/PierreDiffsSwift/blob/main/LICENSE), [upstream package metadata](https://github.com/pierrecomputer/pierre/blob/main/packages/diffs/package.json), [wrapper bundle manifest](https://github.com/jamesrochabrun/PierreDiffsSwift/blob/main/scripts/package.json)

Maintenance signals are positive but the wrapper is young and single-maintainer-shaped. The wrapper has 38 commits and activity/releases from January through 9 August 2026; upstream Pierre had frequent commits through 17 August 2026. The wrapper tracks recent upstream versions and has tests, but the versioning inconsistency above and WebView/JavaScript bridge increase integration risk. [wrapper commits](https://github.com/jamesrochabrun/PierreDiffsSwift/commits/main/), [wrapper releases](https://github.com/jamesrochabrun/PierreDiffsSwift/releases), [upstream commits](https://github.com/pierrecomputer/pierre/commits/main/)

Recommendation: allow PierreDiffsSwift as the preferred implementation candidate for unified/side-by-side rendering, but phrase the Utility contract in implementation-neutral terms. Gate adoption on a throwaway spike that proves SwiftPM resolution, offline rendering, plain-text behavior, app size/startup cost, performance on large input, accessibility/keyboard behavior inside WKWebView, and the planned whitespace/case-ignore preprocessing. Keep a native fallback possible because PierreDiffsSwift does not itself define the product's ignore-whitespace or ignore-case semantics.
