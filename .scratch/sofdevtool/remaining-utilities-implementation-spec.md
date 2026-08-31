# Remaining first-release Utilities implementation specification

Status: approved for implementation on 2026-08-31

This addendum is the canonical implementation contract for the ten agreed first-release Utilities that follow the representative scaffold. It refines [Define first-release Utility contracts](issues/02-define-first-release-utility-contracts.md) without changing the product and architecture boundaries in the [implementation handoff](implementation-handoff.md).

The implementing ticket owns evidence and completion. This document owns shared and Utility-specific behavior. When a ticket and this specification appear to conflict, stop and resolve the specification rather than silently choosing in code.

## Shared completion contract

Every Utility is one end-to-end vertical slice: a strongly typed Utility-owned domain engine, workspace session, source-defined Utility Definition, explicit Registry entry, explicit Clipboard actions, versioned History snapshot, preview and restore, diagnostics, and contract-readable tests. Type erasure remains limited to workspace construction in the Registry; there is no universal execution interface.

All work remains local and offline. Empty untouched input is neutral unless a Utility below defines an explicit action that deliberately operates on empty input. Invalid current input removes stale output. Live or asynchronous work is debounced, cancellable where applicable, and revision-gated so only the latest request may publish a result or History snapshot. Large input is never silently truncated or partially presented as a completed operation.

All Utilities support History. History defaults on except for JWT Decoder, which defaults off. A deliberate completed valid operation produces at most one Utility Operation Snapshot. Invalid/intermediate states, preview, and restore produce none. Restore is explicit and applies the captured controls, input, and result without rerunning.

Each Utility adds registry/search tests and domain tests with independent published vectors or hand-reviewed fixtures. Use fixed clocks and timezones, deterministic randomness, controllable asynchronous backends, temporary storage, and predicate-based UI waits where applicable. Add focused UI coverage only where domain tests cannot prove an interaction-heavy or native integration. Completion requires `scripts/verify --full` and evidence naming the actual verified host; a macOS 14 deployment build is not a runtime check.

## YAML/JSON Conversion

- Stable ID: `yaml-json`; category: Format & Convert; History defaults on.
- Convert one YAML 1.2 Core-oriented document to JSON and JSON to YAML. Support ordinary mappings, sequences, scalars, anchors, and aliases. Reject multi-document streams with a located diagnostic.
- Adopt Yams 6.2.2 with an exact package pin. Keep every Yams import and type inside a Utility-owned adapter. The workspace, History, and conversion domain use only app-owned strongly typed values and errors.
- Use Yams composition with its basic resolver for syntax, anchors/aliases, source marks, single-document enforcement, and emission. SofDevTool owns YAML 1.2 Core scalar resolution, JSON representability, deterministic ordering, and conversion warnings.
- YAML mappings must resolve to unique string keys. Accept strings, booleans, null, arrays, ordered objects, integers, and finite decimal values only when the same mathematical value can be emitted to JSON. Reject non-string/duplicate keys, non-finite numbers, unsupported tags, and values that cannot round-trip faithfully rather than coercing them.
- Date-looking YAML scalars remain strings. YAML 1.1 booleans such as `yes`, `no`, `on`, and `off` remain strings; `true` and `false` are booleans.
- Expand aliases to JSON values while visibly disclosing that comments, anchor/alias identity, formatting, mapping order, and exact key presentation do not survive the conversion. Do not claim general-purpose or lossless YAML 1.2 conformance.
- Calibrate and record input-size and nesting policies during implementation, including deeply nested block/flow collections. Reject over-limit input before presenting partial output.
- Record the final Release binary/app size delta and include the Yams/libYAML MIT notices.
- Research and dependency assessment: [YAML parser options](research/yaml-parser-options.md).

## URL Encoding

- Stable ID: `url-encoding`; category: Encode & Inspect; History defaults on.
- Expose Encode/Decode and two precisely named modes: Path Segment and Query Value. Do not call the first mode merely “Component”.
- Path Segment encoding accepts one raw path segment. Preserve RFC 3986 `pchar` characters, encode `/` and `?`, encode a literal `%`, and never allow the result to create another path segment.
- Query Value encoding accepts one raw query parameter value. Preserve only RFC 3986 unreserved characters and encode delimiters including `&`, `=`, `+`, `#`, and `?`.
- Both modes encode UTF-8 bytes, use `%20` for spaces rather than form-style `+`, and emit uppercase hexadecimal percent triplets. This is distinct from URL-safe Base64 and from `application/x-www-form-urlencoded`.
- Decode percent triplets strictly to UTF-8. Reject incomplete/invalid triplets and invalid UTF-8 rather than guessing. Decoding never treats `+` as a space.
- Tests pin exact behavior for reserved delimiters, literal percent and plus, combining marks, non-Latin text, and complex emoji.

## Hashes

- Stable ID: `hashes`; category: Encode & Inspect; History defaults on.
- Support SHA-256, SHA-384, SHA-512, SHA-1, and MD5 over the exact UTF-8 bytes of the text input. Prefer Apple system cryptography.
- Expose lowercase hexadecimal, uppercase hexadecimal, and Base64 output.
- SHA-1 and MD5 are visibly labelled legacy/non-security algorithms and are never presented as password hashing, authentication, or a recommendation for new security designs.
- Use an explicit Hash action rather than live execution. The untouched empty workspace is neutral; pressing Hash with empty input deliberately hashes zero bytes and records a valid operation.
- Files, HMAC, password hashing, and additional checksums remain out of scope.

## Timestamps

- Stable ID: `timestamps`; category: Format & Convert; History defaults on.
- Default to Auto with explicit Unix Seconds, Unix Milliseconds, ISO 8601, and Local Time input modes available.
- In Auto, ignoring an optional leading sign, integer input of at most ten digits is Unix seconds and integer input of exactly thirteen digits is Unix milliseconds. Eleven- or twelve-digit integers are ambiguous and require a manual mode. Fractional Unix seconds are accepted only in the explicit Unix Seconds mode.
- ISO 8601 input in Auto must contain `Z` or an explicit offset. A local date/time requires Local Time mode and an explicit named timezone.
- Reject nonexistent or repeated local wall times at daylight-saving transitions and ask for an explicit offset rather than silently selecting an instant.
- Results represent one instant as locale-stable ISO 8601, UTC, and selected named-timezone values. Always show the inferred input representation and timezone identifier.
- Now is an explicit action backed by an injectable clock. Restore retains the captured instant and never substitutes the current time.

## JWT Decoder

- Stable ID: `jwt-decoder`; category: Encode & Inspect; History is supported but defaults off.
- Decode Base64URL header and payload segments as UTF-8 JSON and present them separately. Treat the signature segment as opaque and unverified.
- Always state prominently that decoding does not verify the signature, interpret claims, judge validity, establish trust, or determine fitness for use. Accept no secret or key and expose no verification action.
- Diagnose segment count, Base64URL, UTF-8, header JSON, and payload JSON failures separately. Never interpret `alg`, expiry, issuer, audience, or any other claim.
- Input and decoded payloads are not persisted or restored by default. If the owner explicitly enables JWT History, valid snapshots may be recorded and restored only through an explicit History action, never automatically on launch.

## Regex

- Stable ID: `regex`; category: Compare & Test; History defaults on.
- Expose the platform ICU-compatible dialect, agreed flags, pattern, test text, all matches, captures, replacement template, and replacement preview. Do not emulate other language flavors.
- Implement a replaceable in-process ICU C backend inside the existing app target. Keep `import ICU`, `URegularExpression`, callbacks, and ICU error codes inside the adapter; the workspace/domain expose only app-owned strongly typed values.
- Every operation sets nonzero ICU engine-step and heap-backtracking budgets, both match and find-progress callbacks, a monotonic deadline, and app-level caps for pattern/input/replacement size, match/capture count, and replacement-preview output.
- Run on a bounded serial worker outside `MainActor`; coalesce or cancel superseded work. Only the current revision may publish. Map engine timeout, backtracking overflow, cancellation, and app-level limits to distinct diagnostics.
- Numeric budgets are selected through implementation-time calibration on representative valid and pathological fixtures, stored as named injectable policy values, and recorded with evidence. Do not equate ICU steps with milliseconds or promise an absolute wall-clock cutoff.
- A resource-limit outcome preserves inputs but publishes no partial match/replacement result and emits no History snapshot. Zero-length matches must advance correctly.
- Do not use an uncancellable `NSRegularExpression` timeout race, unbounded detached tasks, a different regex flavor, or a helper/XPC target under the current three-target architecture.
- Research and verification design: [Regex execution bounds](research/regex-execution-bounds.md).

## Case Conversion

- Stable ID: `case-conversion`; category: Format & Convert; History defaults on.
- Support camelCase, PascalCase, snake_case, SCREAMING_SNAKE_CASE, kebab-case, Title Case, sentence case, lowercase, and uppercase.
- Show an inspectable Detected Words representation before or alongside the result. The same segmentation drives every case output.
- Use a deterministic locale-independent developer-text policy. Split transitions such as `HTTPServer` into `HTTP` and `Server`, and `version2Value` into `version`, `2`, and `Value`; expose acronym, digit, punctuation, and mixed-separator decisions rather than hiding them.
- Preserve valid Unicode and complete grapheme clusters, including accented/combining text, non-Latin scripts, and complex emoji. Identifier conversion must not vary with the Mac's current locale.

## Whitespace Conversion

- Stable ID: `whitespace-conversion`; category: Format & Convert; History defaults on.
- Keep Edge Trim, Per-line Trim, Collapse Horizontal Whitespace, Collapse All Whitespace, Normalize Line Endings, Tabs to Spaces, Spaces to Tabs, Remove Blank Lines, and Dedent as separate actions. There is no generic Clean action.
- Normalize Line Endings exposes LF, CRLF, and CR; LF is the default.
- Tab width defaults to four and is configurable from one through eight. Tabs to Spaces expands using tab stops. Spaces to Tabs affects leading indentation only so ordinary aligned prose is not silently changed.
- Remove Blank Lines removes empty and whitespace-only lines. Preserve whether the input ended with a final newline unless the selected operation explicitly targets that boundary.
- Dedent removes only the common removable indentation of non-blank lines and preserves relative indentation; mixed tabs/spaces and blank-only input have explicit tested behavior.

## Color Conversion

- Stable ID: `color-conversion`; category: Format & Convert; History defaults on.
- Use one sRGB model and accept `#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`, RGB/RGBA with integer or percentage components, and HSL/HSLA with degree hue and percentage saturation/lightness.
- Validate syntax, finite values, and component ranges. Preserve alpha across every representation and synchronize the native color picker without feedback loops or duplicate History operations.
- Emit deterministic modern CSS representations and lowercase HEX. Omit alpha only when the color is fully opaque. Choose and record a round-trip-stable decimal precision during implementation; trim insignificant trailing zeros consistently.
- Wide gamut, contrast analysis, palette generation, named colors, and full CSS Color Level 4 parsing remain out of scope.

## Sample Data

- Stable ID: `sample-data`; category: Generate; History defaults on.
- Default to ten rows of JSON. Allow one through 1,000 rows and at most 50 fields.
- Support named fields of type fictional name, fictional email, number, boolean, date, UUID, and enum. Field names must be unique and valid for both selected output formats.
- Numbers expose integer/decimal and a valid range. Dates use an explicit range and emit ISO 8601 UTC. UUID values are v4. Enum choices are explicit and non-empty.
- Generate names and emails from small bundled local vocabularies and label every generated identity as fictional. Do not imply realistic identities, locale simulation, relational consistency, or domain realism.
- JSON is the default and preserves configured field order. CSV uses an RFC 4180-compatible quoted-field policy, includes a header, uses LF, and correctly escapes commas, quotes, newlines, Unicode, and empty values.
- Generation is explicit. Randomness, clock/date selection, and UUID generation use Utility-owned deterministic test seams; production uses system cryptographic randomness where unpredictability is expected.
- Schema import/paste, relations, locale packs, external datasets, and mass generation beyond the stated bounds remain out of scope.

## Implementation order and blocking

The ten Utilities are independent vertical slices. Each is blocked only by the resolved scaffold ticket and may be implemented in any order. There are no shared prefactor tickets: any adapter or calibration belongs to its owning Utility.

Parallel implementation is logically allowed, but concurrent edits to the Registry, Xcode project, lockfile, and shared tests must be isolated or coordinated. A sequential implementation avoids merge conflicts without introducing false dependency edges.
