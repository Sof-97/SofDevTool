# SofDevTool utility backlog

The following agreed first-release Utilities are not yet implemented. Their product contracts are defined in [ticket 02](.scratch/sofdevtool/issues/02-define-first-release-utility-contracts.md).

- [ ] **YAML/JSON Conversion** — Convert one YAML 1.2 document to JSON and JSON to YAML. Support ordinary mappings, sequences, scalars, anchors, and aliases; disclose lossy round-trip behavior. Defer multi-document streams.
- [ ] **URL Encoding** — Percent-encode and decode UTF-8 in explicit component and full-query-value modes. Keep this distinct from the existing URL-safe Base64 alphabet and diagnose invalid input rather than guessing.
- [ ] **Hashes** — Hash UTF-8 text with SHA-256, SHA-384, SHA-512, SHA-1, and MD5. Support lowercase/uppercase hexadecimal and Base64 output; label SHA-1 and MD5 as legacy. Defer files, HMAC, and password hashing.
- [ ] **Timestamps** — Convert Unix seconds, Unix milliseconds, ISO 8601, UTC, and local time. Show auto-detection explicitly, produce locale-stable output with a named timezone, and make “Now” an explicit action.
- [ ] **JWT Decoder** — Decode readable JWT header and payload segments only. Do not verify signatures, interpret claims, judge validity, or accept secrets. Keep History disabled by default and do not restore payloads automatically.
- [ ] **Regex** — Provide the visible platform ICU-compatible dialect, flags, all matches, capture groups, replacement preview, and zero-length-match handling. Bound execution so pathological expressions cannot freeze the app.
- [ ] **Case Conversion** — Support camelCase, PascalCase, snake_case, SCREAMING_SNAKE_CASE, kebab-case, Title Case, sentence case, lowercase, and uppercase. Preserve Unicode and expose surprising word segmentation.
- [ ] **Whitespace Conversion** — Provide separate actions for edge trimming, per-line trimming, horizontal/all-whitespace collapse, line-ending normalization, tabs-to-spaces, spaces-to-tabs, blank-line removal, and dedenting.
- [ ] **Color Conversion** — Convert sRGB HEX, RGB/RGBA, and HSL/HSLA; preserve alpha, include the native color picker, and provide CSS-compatible output. Defer wide gamut, contrast analysis, and palette generation.
- [ ] **Sample Data** — Generate local synthetic JSON or CSV rows from typed fields including names, emails, numbers, booleans, dates, UUIDs, and enums. Mark identities as fictional; defer schema import, relations, locale packs, and realistic domain simulation.

All implementations must preserve the existing Utility boundaries: strongly typed independent modules, explicit Paste/Copy, local-only operation, and History entries only for completed valid operations.
