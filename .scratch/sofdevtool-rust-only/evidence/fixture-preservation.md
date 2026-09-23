# Swift contract-vector preservation audit

Read-only comparison of the legacy `SofDevToolTests/*.swift` cases with
`crates/core/src/utilities/*`, `crates/core/tests/json_contract.rs`, and the
new independent integration vectors in
`crates/core/tests/retained_contract_vectors.rs`. This is fixture-preservation
evidence only; it does not claim Swift retirement or native verification.

## Additional vectors retained

The existing Rust unit tests exercise most of the same rules, but did not pin
these exact, cross-feature legacy inputs or boundary values. They are now
asserted through the public core evaluation APIs against literal goldens:

| Legacy domain | Added independent expectation |
| --- | --- |
| Case Conversion | CJK-to-ASCII transition `東京Value` and the exact skin-tone ZWJ token in `go👩🏽‍💻Now`. |
| Color | Achromatic `hsla(240deg, 0%, 50%, 0.5)` normalized outputs; combined percentage RGB and slash alpha teal output; legacy rejection of RGB range/NaN, out-of-range HSL, and `display-p3`. |
| Base64 | Exact decode of `Y2Fmw6kg8J+RqfCfj73igI3wn5K7` to `café 👩🏽‍💻`. |
| JWT Decoder | The old `exp: 0` Unicode payload remains opaque decoded JSON, with the signature returned unchanged. |
| Hashes | The exact composite `café 👩🏽‍💻` digest goldens for SHA-256/384/512, SHA-1, and MD5, plus the legacy SHA-256 uppercase-hex and Base64 forms. |
| Timestamps | The pre-epoch `-0.5` half-second representation and the former Europe/Rome 2026 spring gap, fall fold, and ordinary wall-time literals. |
| URL Encoding | The combined decomposed-accent, literal-plus, and CJK decode vector `e%CC%81+%E6%BC%A2%E5%AD%97` → `é+漢字`. |
| Regex | The supported Unicode case-insensitive capture/replacement expectation from the old test, expressed with Rust's `(?P<word>...)` group syntax. |

## Coverage already present; no duplicate vectors added

- `CaseConversionTests.swift`: current `case_conversion.rs` tests pin acronym
  runs and trailing capitals, both digit transitions, separators/symbols,
  combining marks, emoji/graphemes, all nine outputs, snapshots, and the UTF-16
  limit. `東京Value` and the old skin-tone ZWJ token were the gaps retained
  above. The Turkish `İSTANBUL` legacy assertions only checked that the result
  contained `café` and ended in the emoji; no exact output literal was lost.
- `ColorConversionTests.swift`: current `color.rs` tests cover red/HSL/RGB
  equivalence, shorthand-alpha expansion, RGB alpha, hue wrapping, strict
  errors, and round trips. The old quantized achromatic-HSL and combined teal
  outputs, plus explicit `display-p3` rejection, are retained above.
- `HashesUtilityTests.swift`: `hashes.rs` has independent published `abc` and
  empty-input vectors, UTF-8/precomposed/decomposed/emoji cases, all output
  representations, explicit repeats, labels, and snapshot behavior. The old
  combined accent-plus-skin-tone string across every algorithm was missing and
  is retained above.
- `JWTDecoderTests.swift`: `jwt.rs` pins a published literal token's decoded
  header, payload, and opaque signature and covers UTF-8 payloads, segment and
  decoding failures, and snapshots. Its Unicode payload content was already
  covered, but no core golden pinned the expired `exp: 0` claim remaining
  uninterpreted. That literal token and exact decoded payload are retained
  above.
- `RegexTests.swift`: `regex.rs` separately covers supported flags, captures,
  replacements, Unicode byte offsets, zero-width matches, limits, cancellation,
  and invalid syntax. The Unicode case-insensitive capture/replacement
  combination is retained above in the current supported dialect.
- `SampleDataTests.swift`: `sample_data.rs::csv_quotes_delimiters_quotes_newlines_unicode_and_empty_values`
  asserts the same RFC 4180 quote/comma/newline/emoji CSV bytes literally.
  Field ordering, deterministic generation, invalid schemas, row/field limits,
  and snapshot restoration are already covered in that module.
- `TimestampsUtilityTests.swift`: `timestamps.rs` covers digit-count
  inference, offsets, negative fractions (including `-1.5`), named zones,
  DST gaps/folds, injected clock, and restore. The exact `-0.5` zero-boundary
  and Europe/Rome 2026 wall-time inputs were missing and are retained above.
- `URLEncodingUtilityTests.swift`: `url_encoding.rs` covers path/query safe
  sets, delimiter escaping, uppercase UTF-8 triplets, decomposed accents,
  CJK, complex emoji, plus preservation, malformed bytes, and round trips.
  The combined decomposed-accent / plus / CJK literal was missing and is
  retained above.
- `WhitespaceConversionTests.swift`: `whitespace.rs` already asserts the
  legacy edge/per-line/horizontal/all-whitespace behavior, mixed CRLF/CR/LF,
  tabs, visual-column dedent, and Unicode graphemes with literal input/output
  expectations. No extra copy was warranted.
- `SofDevToolContractTests.swift` JSON cases map to
  `crates/core/tests/json_contract.rs`: exact formatting/indentation/minify,
  recursive sort and array order, numeric preservation, Unicode escape/surrogate
  handling, duplicate keys, diagnostics, JSON Pointer and dot/bracket queries,
  query-container ordering, nesting bounds, and fixture-driven comparisons.
  YAML cases map to `yaml_json.rs` scalar, number, alias, malformed/tag/duplicate,
  deterministic round-trip, resource-policy, and snapshot tests. Base64's
  standard and URL-safe vectors map to `base64.rs`; its exact complex Unicode
  decode is retained above. Identifier UUID/ULID/KSUID cases map to
  `identifiers.rs` published namespace, bit, encoding, timestamp, sort, and
  inspection tests. Text Diff line structure and emoji policy are app/renderer
  boundaries and remain in the app renderer tests, not this core fixture file.

## Deliberate omissions

- The old ICU look-behind expression from `RegexTests.swift` is not retained as
  an accepted case. The Rust core explicitly uses the Rust `regex` dialect and
  rejects look-around; copying the ICU expectation would encode unsupported
  behavior. The Rust look-behind diagnostic has its own current contract test.
- The old Swift palette literals from `SofDevToolContractTests.swift` are not
  copied. The Rust theme intentionally uses a different approved palette, and
  palette attribution is handled by the app-owned notice work, not core test
  fixtures.
- `HistoryDefaultsTests.swift`, identity/catalog/search, shortcut registration,
  panels, and UI-rendering assertions are application or interaction contracts,
  not core utility vectors. They remain with their Rust app/shell ticket tests
  rather than being mirrored in a core integration test.
- No algorithm is duplicated to derive an expected value. Each added test
  submits one literal input through the public contract and compares to an
  independent literal expectation.

## Verification

`CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/private/tmp/sofdevtool-retained-fixtures-target cargo test -p sofdevtool-core --test retained_contract_vectors`
passed: 9 tests, 0 failures. The scoped strict Clippy command
`CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/private/tmp/sofdevtool-retained-fixtures-target cargo clippy -p sofdevtool-core --test retained_contract_vectors -- -D warnings`
passed. `rustfmt --edition 2021 --check crates/core/tests/retained_contract_vectors.rs`
and `git diff --check -- crates/core/tests/retained_contract_vectors.rs .scratch/sofdevtool-rust-only/evidence/fixture-preservation.md`
passed. No other file was formatted or modified for this assignment.
