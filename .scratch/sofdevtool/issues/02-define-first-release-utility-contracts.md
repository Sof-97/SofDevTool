Type: grilling
Status: resolved

## Question

What exact user-visible behavior and boundary defines each first-release Utility: JSON formatting/validation/querying, YAML and JSON conversion, Base64 and URL encoding, hashes, UUIDs, timestamps, JWT inspection, regex testing, text diffing, case and whitespace conversion, color conversion, sample-data generation, and random-string generation? Confirm the representative scaffold slice of JSON, Base64, UUID, and random-string Utilities and identify shared behaviors without prematurely forcing unlike Utilities into one UI.

## Comments

### Shared contract — confirmed 2026-08-20

- Every listed Utility belongs to the first complete release; JSON, Base64, UUID, and random-string Utilities form the representative initial scaffold slice.
- Utilities are text-first. File input is deferred and may be reconsidered later.
- Deterministic lightweight operations run live with debouncing; heavy operations and generators may require explicit execution.
- Applicable Utilities consistently expose labeled input and result regions, inline diagnostics, Copy Result, Clear, optional Swap, and task-appropriate keyboard focus without being forced into one layout.
- Invalid input replaces the result with a precise inline diagnostic; stale results are not presented as current.
- Clipboard reads and writes are always explicit.
- Utility History receives one entry per completed valid Utility Operation; invalid and intermediate edits do not create entries.
- There is no public arbitrary-input-size guarantee. The app remains responsive, may require explicit execution for expensive input, warns rather than silently truncating, and never partially processes input without saying so.

### Individual Utility contracts — confirmed 2026-08-21

- JSON: validate with line-and-column diagnostics; pretty-print with configurable indentation; minify; sort object keys explicitly; and query using JSON Pointer plus simple dot/bracket paths. Preserve array order and numeric values; never silently repair malformed JSON.
- YAML and JSON: convert one YAML 1.2 document to JSON and JSON to YAML, including ordinary mappings, sequences, scalars, anchors, and aliases. Warn that comments, aliases, formatting, and exact key presentation may not round-trip. Multi-document streams are deferred.
- Base64 and URL encoding: encode and decode UTF-8 using standard and URL-safe Base64 alphabets with optional padding. URL encoding distinguishes component and full-query-value modes. Invalid input produces a diagnostic rather than guessed output.
- Hashes: hash UTF-8 text with SHA-256, SHA-384, SHA-512, SHA-1, and MD5; label SHA-1 and MD5 as legacy/non-security choices; output lowercase or uppercase hexadecimal and Base64. Files, HMAC, password hashing, and other checksums are deferred.
- UUIDs: generate multiple UUID types, including sortable UUIDs, singly or in configurable batches; validate and normalize pasted UUIDs; offer uppercase/lowercase and hyphen controls. The exact supported versions remain to be settled.
- Timestamps: convert Unix seconds, Unix milliseconds, ISO 8601, UTC, and local time. Auto-detection displays its inference. Output is locale-stable with an explicit timezone; “now” is an explicit action.
- JWT Decoder: decode readable JWT segments only. Signature verification, claim interpretation, validity judgments, secret entry, and related JWT behavior are out of scope. History is disabled by default.
- Regex: use the platform's visible ICU-compatible dialect; support flags, all matches, capture groups, replacement preview, and zero-length matches; bound execution so pathological expressions cannot freeze the app. Cross-language flavor emulation is deferred.
- Text diff: compare two text inputs with line-level differences, intraline highlighting, unified and side-by-side views. Whitespace-ignore and case-ignore are off by default. Directory comparison, patch application, and Git integration are deferred. Use Pierre if its technical and licensing fit is confirmed.
- Case conversion: support camelCase, PascalCase, snake_case, SCREAMING_SNAKE_CASE, kebab-case, Title Case, sentence case, lowercase, and uppercase; expose surprising word segmentation and preserve Unicode.
- Whitespace conversion: provide separate actions for trimming edges, trimming each line, collapsing horizontal whitespace, collapsing all whitespace, normalizing line endings, tabs-to-spaces, spaces-to-tabs, removing blank lines, and dedenting. Do not hide several destructive changes behind a generic clean action.
- Color conversion: use sRGB; convert HEX, RGB/RGBA, and HSL/HSLA; include the native color picker; preserve alpha; provide CSS-compatible output. Wide-gamut conversion, contrast analysis, and palette generation are deferred.
- Sample data: generate configurable rows of local synthetic JSON or CSV using typed fields such as names, emails, numbers, booleans, dates, UUIDs, and enum choices. Generated identities are explicitly fictional. Schema import, relational datasets, locale packs, and realistic domain simulation are deferred.
- Random strings: use the system cryptographic random-number generator; support length, count, alphanumeric, hexadecimal, URL-safe, and custom character sets; optionally exclude ambiguous characters. Show entropy only when honestly calculable; do not present it as password-management functionality.

### Defaults and restoration — partially confirmed 2026-08-21

- Empty input is a neutral state with no error and no history entry.
- Reopening a Utility should remember all prior state. The interaction between that preference, disabled history, and sensitive JWT data remains to be settled explicitly.
- JSON opens in Format mode with two-space indentation, live validation, and unsorted keys. Querying and key sorting are explicit choices.
- Base64 opens in Encode mode with the standard alphabet, padding enabled, and UTF-8 text. Encode/Decode and Standard/URL-safe are visible controls.
- UUID generation defaults to one v4 UUID, lowercase with hyphens. Version, count, casing, and hyphenation are visible controls.
- Random strings should default to a shorter, password-like result; exact length and alphabet remain to be settled.
- Copy copies only the canonical result and confirms success with a brief toast or similar unobtrusive notification. Multi-result generators provide per-result Copy and newline-separated Copy All.

### Supporting research

- [UUID versions and PierreDiffsSwift feasibility](../research/utility-contract-facts.md): RFC 9562 UUID boundaries and a dependency-fit assessment for Pierre.
- [ULID and KSUID format behavior](../research/sortable-identifier-facts.md): canonical forms, validation, timestamp inspection, sorting, and monotonicity boundaries.

### Final contract refinements — partially confirmed 2026-08-21

- Utility controls are always restored. Inputs and results are restored only through enabled Utility History; disabling History prevents equivalent payload persistence elsewhere. JWT input and output are neither persisted nor restored by default.
- Random String exposes length, selectable character sets, letter case, digits, safe symbols, ambiguous-character exclusion, and multi-string generation. It starts at length 20 with uppercase, lowercase, digits, and safe symbols enabled, ambiguous characters excluded, and one result; the latest choices are restored when reopening it.
- UUID supports v1, v3, v4, v5, v6, and v7; v4 is the ordinary random default and v7 the recommended sortable UUID. Namespace/name inputs are explained for v3/v5, v1/v6 carry privacy warnings, and generic v8 is omitted.
- Sortable identifier support extends beyond UUIDs to formats such as ULID and KSUID. Their organization and exact contract remain to be settled.
- The Text Diff contract remains implementation-independent. PierreDiffsSwift is the preferred candidate subject to a focused spike covering offline operation, accessibility, keyboard use, large inputs, app size, licensing notices, exact ignore semantics, and a pinned compilable release.

### Identifier Generator — confirmed 2026-08-21

- The former UUID Utility is named Identifier Generator. A top-level format selector separates UUID, ULID, and KSUID; format-specific controls appear only after selection, and ULID/KSUID are never presented as UUID versions.
- ULID supports Random and Monotonic modes, single and batch generation, strict validation, canonical uppercase normalization, and timestamp inspection in Unix milliseconds and ISO 8601. Monotonic ordering is explicitly local to one generator, not coordinated across apps or machines.
- KSUID supports ordinary cryptographically random single and batch generation, strict case-sensitive validation, and timestamp inspection. An explicit ordered-sequence batch mode exposes its 65,536-result limit; ordinary KSUID generation is not described as strictly monotonic.
- Generated identifiers are described as collision-resistant, not mathematically unique. Embedded timestamps are non-secret metadata, and no identifier is presented as an authorization secret.
- The representative initial scaffold implements the complete Identifier Generator, including UUID, ULID, and KSUID, rather than a UUID-only or partially enabled shell.

## Answer

The first complete release contains every Utility named in the question. The representative scaffold implements JSON, Base64, Random String, and the complete Identifier Generator (UUID, ULID, and KSUID).

### Shared behavior

Utilities are text-first; file and batch-file workflows are deferred. Lightweight deterministic work runs live with debouncing, while expensive work and generators use explicit execution. Applicable Utilities provide clear input/result regions, inline diagnostics, Copy Result, Clear, optional Swap, and task-appropriate keyboard focus without sharing one forced layout. Empty input is neutral. Invalid input replaces the current result with a precise diagnostic; stale output is never presented as current. No Utility reads or writes the clipboard implicitly, and copying confirms success with an unobtrusive toast.

One settled valid computation or generation request is one Utility Operation and may create one History entry; intermediate and invalid edits do not. Controls are restored when a Utility reopens. Inputs and results are restored only through enabled Utility History, so disabling History cannot be bypassed by equivalent persistence. JWT payloads are neither persisted nor restored by default. Large input is never silently truncated or partially processed; the app may warn or require explicit execution to remain responsive.

### Utility boundaries

- JSON validates with line/column diagnostics, formats with configurable indentation, minifies, explicitly sorts keys, and queries with JSON Pointer plus simple dot/bracket paths. It never silently repairs malformed input.
- YAML/JSON converts one YAML 1.2 document in either direction and warns that comments, aliases, formatting, and exact key presentation may not round-trip. Multi-document streams are deferred.
- Base64 encodes/decodes UTF-8 with standard and URL-safe alphabets and optional padding. URL encoding distinguishes component from full-query-value mode. Invalid decoding is diagnosed rather than guessed.
- Hashes cover SHA-256/384/512 and clearly labelled legacy SHA-1/MD5, with hexadecimal or Base64 output. Files, HMAC, password hashing, and other checksums are deferred.
- Identifier Generator has an explicit UUID/ULID/KSUID format selector. UUID supports v1, v3, v4, v5, v6, and v7, defaulting to one lowercase hyphenated v4 and recommending v7 for sortable UUIDs; v8 is omitted. ULID supports random and local-monotonic modes, strict validation, uppercase normalization, and timestamp inspection. KSUID supports random generation and explicit ordered-sequence batches, strict case-sensitive validation, and timestamp inspection. Identifiers are collision-resistant rather than guaranteed unique or secret.
- Timestamps convert Unix seconds/milliseconds, ISO 8601, UTC, local time, and an explicit Now value; any detected representation is shown and output is locale-stable with an explicit timezone.
- JWT Decoder only decodes readable JWT segments. It does not verify signatures, interpret claims, judge validity, or accept secrets.
- Regex uses the visible platform ICU-compatible dialect with flags, all matches, captures, replacement preview, zero-length-match handling, and bounded execution. Cross-language flavor emulation is deferred.
- Text Diff provides line and intraline changes in unified and side-by-side views, with opt-in whitespace/case ignoring. Directory comparison, patch application, and Git integration are deferred. PierreDiffsSwift is a candidate, not part of the product contract, until separately validated.
- Case Conversion supports camel, Pascal, snake, screaming snake, kebab, title, sentence, lower, and upper case while preserving Unicode and exposing surprising segmentation.
- Whitespace Conversion exposes trim, per-line trim, horizontal/all-whitespace collapse, line-ending normalization, tab/space conversion, blank-line removal, and dedent as distinct actions.
- Color Conversion uses sRGB and covers HEX, RGB(A), HSL(A), alpha, the native picker, and CSS-compatible output. Wide gamut, contrast analysis, and palettes are deferred.
- Sample Data creates local synthetic JSON or CSV rows from typed fields and labels generated identities fictional. Schema import, relations, locale packs, and realistic domain simulation are deferred.
- Random String uses the system cryptographic generator with configurable length, count, case, digits, safe/custom character sets, and ambiguous-character exclusion. It starts at length 20 with upper/lowercase, digits, safe symbols, ambiguous characters excluded, and one result; entropy is shown only when honestly calculable.

Supporting evidence: [UUID and Pierre research](../research/utility-contract-facts.md) and [ULID/KSUID research](../research/sortable-identifier-facts.md).
