# YAML parser options for YAML/JSON Conversion

Research date: 2026-08-30

## Recommendation

Use **Yams 6.2.2**, pinned exactly, behind a Utility-owned adapter. Parse with
`compose(yaml:resolver:)` and `Resolver.basic`, then perform YAML 1.2 Core scalar
resolution and JSON-representability checks in SofDevTool's own typed conversion
layer. Use Yams only for YAML syntax, anchors/aliases, source marks, and emission.

Do **not** describe the result as a fully conforming, general-purpose YAML 1.2
processor. The intended contract is narrower: one YAML 1.2 Core-oriented document
containing ordinary mappings, sequences, scalars, anchors, and aliases, converted
to the JSON data model with explicit rejection of values that cannot be represented
faithfully. That wording matches the Utility's real destination and is testable.

Yams is the best balance of current maintenance, Swift/SPM compatibility, parser
and emitter coverage, diagnostics, adoption, and replacement cost. Its default
schema is the important caveat: it recognizes YAML 1.1-style booleans such as
`yes`, `no`, `on`, and `off`, while YAML 1.2 Core does not. Yams exposes
`Resolver.basic` specifically as a resolver with no implicit rules, so the app can
avoid that behavior and own the narrow semantic conversion policy. [Yams resolver
source](https://github.com/jpsim/Yams/blob/6.2.2/Sources/Yams/Resolver.swift),
[YAML 1.2.2 Core schema](https://yaml.org/spec/1.2.2/#103-core-schema)

Do not build a YAML parser in-house. A small app-owned **semantic walker and JSON
emitter** is justified; a custom implementation of YAML scanning, parsing,
anchors, aliases, block scalars, directives, and source diagnostics is not.

## Decision matrix

| Candidate | Fit for this Utility | Main strengths | Main risks / gaps | Decision |
|---|---|---|---|---|
| [Yams 6.2.2](https://github.com/jpsim/Yams/releases/tag/6.2.2) | High, with an app-owned YAML 1.2 Core conversion layer | Mature Swift API; parser and emitter; anchors/aliases; single-document enforcement; line/column diagnostics; no external SPM dependencies; Swift 6 manifest | Default resolver has YAML 1.1 semantics; libYAML-based parser is not evidence of complete YAML 1.2 conformance; no documented parse-depth limit | **Recommend** |
| [PotentCodables 3.5.3](https://github.com/outfoxx/PotentCodables/releases/tag/3.5.3) | Functionally high, operationally oversized | libfyaml-based parser/emitter; explicit Core schema; line/column errors; anchors/aliases retained in its model; Swift 6 support | One package product exposes YAML together with JSON, CBOR, ASN.1 and a substantial dependency graph; public convenience API folds multiple YAML documents into a sequence rather than rejecting them | Reject for first release |
| [swift-yaml 1.0.0](https://github.com/1amageek/swift-yaml/releases/tag/1.0.0) | Promising parser, incomplete codec | Pure Swift; no dependencies; claims YAML 1.2.2 coverage; anchors/aliases; source marks; explicit depth limit | Parser only, no emitter; Swift 6.2 minimum; very new 1.0 package with minimal adoption and limited maintenance history | Revisit after maturity |
| Bounded custom YAML parser | Low | Full local control | YAML syntax and security surface are much larger than this Utility; high test and maintenance cost | Reject |

## Why Yams fits the required surface

### One document and multi-document diagnostics

Yams exposes both all-document and single-document APIs. Its single-root parser
loads one document and then explicitly throws a composer error if another document
is present. This directly supports the Utility's requirement to diagnose streams
instead of silently taking the first document. [Yams parser source, single-root
logic](https://github.com/jpsim/Yams/blob/6.2.2/Sources/Yams/Parser.swift#L224-L241)

The adapter should call `compose(yaml: input, .basic)`, not `load(yaml:)`:

- `compose` returns the representation tree needed to inspect mapping keys, scalar
  styles, tags, and source marks before constructing JSON;
- `.basic` disables Yams's default YAML 1.1-style implicit scalar rules;
- `singleRoot` rejects a second document;
- aliases are resolved through the parser's anchor map, which is suitable because
  JSON has no alias identity and the UI already discloses that aliases will not
  survive the conversion. [Yams parser source, alias handling](https://github.com/jpsim/Yams/blob/6.2.2/Sources/Yams/Parser.swift#L312-L321)

YAML itself treats anchor names as serialization detail and does not require them
to survive composition, so expanding an alias to the referenced JSON value is
consistent with the format model, provided the Utility discloses the loss of alias
identity. [YAML 1.2.2 representation graph](https://yaml.org/spec/1.2.2/#322-anchors-and-aliases)

### Diagnostics

Yams error cases carry reader offsets or one-based line/column marks for scanner,
parser, and composer failures, plus the original YAML needed to render a snippet.
This is enough to translate dependency errors into SofDevTool's shared diagnostic
envelope without exposing Yams types outside the adapter. [Yams error
source](https://github.com/jpsim/Yams/blob/6.2.2/Sources/Yams/YamlError.swift#L18-L79)

The adapter should normalize these cases into a Utility-owned error containing:

- a stable error category;
- a concise user-facing message;
- optional one-based line and column; and
- no raw parser object or dependency type.

Duplicate mapping keys already produce a dedicated Yams error. The conversion
walker must add precise errors for JSON-incompatible keys, tags, and numbers.

### YAML 1.2 semantics and faithful JSON conversion

YAML 1.2 defines schemas separately from syntax and explicitly permits the
application to control tag resolution. The Core schema recognizes nulls, booleans,
decimal/octal/hex integers, finite and non-finite floats, and otherwise strings.
The JSON schema is narrower. [YAML 1.2.2 schema definition and Core
rules](https://yaml.org/spec/1.2.2/#chapter-10-recommended-schemas)

For this converter, use the YAML 1.2 **Core** rules for ordinary user input, then
apply a separate JSON-representability gate:

- accept mappings only when every resolved key is a unique string;
- accept sequences recursively;
- accept strings, booleans, and null;
- accept integers and finite decimal numbers only when the app-owned numeric
  representation can emit the same mathematical value to JSON;
- reject `.inf`, `-.inf`, and `.nan`, because JSON has no representation for them;
- reject explicit application-specific tags and YAML collection types outside the
  supported surface rather than coercing them;
- treat implicit date-looking values as strings under Core rules; and
- diagnose unsupported values at their Yams source mark.

Do not pass the parsed tree through `Any`, `NSDictionary`, or
`JSONSerialization` as the semantic bridge. That makes it too easy to coerce
non-string keys or round large numbers. Use a small Utility-owned recursive value,
for example string / number-lexeme / boolean / null / array / ordered object, and
write deterministic JSON from that value. JSON object keys should be sorted for a
stable canonical result; YAML mappings are not semantically ordered. [YAML 1.2.2
mapping-order rule](https://yaml.org/spec/1.2.2/#3212-nodes)

For JSON-to-YAML, parse JSON into the same Utility-owned value, construct a Yams
node tree containing only JSON-compatible YAML types, and serialize it. Sort object
keys before constructing mappings so output is deterministic. This bounded emitter
path never needs YAML-only tags, aliases, or custom object types.

## Dependency assessment

### Maintenance and Swift/SPM compatibility

Yams 6.2.2 was released on 2026-05-26 and includes a Swift 6.0 package manifest.
The manifest declares a `Yams` library backed by its bundled `CYaml` target and no
other package dependencies. The standard manifest requires Swift 5.7; the
Swift-6-specific manifest selects Swift tools 6.0. [Yams 6.2.2 release](https://github.com/jpsim/Yams/releases/tag/6.2.2),
[Swift 6 package manifest](https://github.com/jpsim/Yams/blob/6.2.2/Package@swift-6.0.swift),
[standard package manifest](https://github.com/jpsim/Yams/blob/6.2.2/Package.swift)

Local compatibility probe on 2026-08-30:

- toolchain: Apple Swift 6.3.3;
- target: arm64 Apple Silicon;
- deployment target: macOS 14.0;
- exact package: Yams 6.2.2;
- result: a release executable using both `compose` and `serialize` built
  successfully, and `otool` reported `minos 14.0`.

This is build compatibility evidence on the current macOS 26 host, not a macOS 14
runtime check.

### License and notices

Yams and its bundled libYAML are MIT licensed. Preserve the Yams/libYAML notice in
the app's third-party notice inventory. [Yams README license statement](https://github.com/jpsim/Yams#license),
[Yams 6.2.2 license](https://github.com/jpsim/Yams/blob/6.2.2/LICENSE)

### Offline and privacy behavior

The runtime library parses caller-provided strings/data and emits strings; its SPM
manifest has no external runtime packages or services. Package retrieval happens
at build time. No account, network connection, telemetry, clipboard access, file
access, or persistence is required by the parser. Clipboard and History therefore
remain exclusively behind SofDevTool's existing explicit adapters. [Yams package
manifest](https://github.com/jpsim/Yams/blob/6.2.2/Package.swift),
[Yams documented APIs](https://github.com/jpsim/Yams#usage)

### Accessibility impact

The dependency has no UI. It has no direct accessibility impact. The Utility still
owns readable diagnostics, focus behavior, explicit Paste/Copy, keyboard access,
and the round-trip-loss disclosure.

### Binary and build impact

Yams adds one Swift target and one bundled C target, with no transitive packages.
The source blobs under `Sources/` in tag 6.2.2 total about 542 KB.

An isolated local release-CLI probe provides an approximate linked-size bound:

| Artifact | Unstripped | `strip -x` |
|---|---:|---:|
| Foundation-only baseline | 52,728 bytes | 51,376 bytes |
| Same executable using Yams parse + emit | 1,260,440 bytes | 795,872 bytes |
| Observed delta | 1,207,712 bytes | 744,496 bytes (about 727 KiB) |

This is not the final `.app` delta: the Xcode linker, dead stripping, architecture
slices, code signing, and overlap with already-linked Foundation code will change
it. Record the final Release `.app` and executable delta when the dependency is
integrated. The current measurement indicates a bounded sub-megabyte stripped
arm64 impact, not a zero-cost dependency.

## Replacement seam

Keep all `import Yams` statements inside one adapter in
`Utilities/YAMLJSONConversion`. The rest of the Utility should depend on narrow,
strongly typed, Utility-owned operations rather than Yams nodes or a universal app
execution interface:

```swift
protocol YAMLDocumentCodec: Sendable {
    func decodeSingleDocument(_ source: String) throws -> YAMLJSONDocument
    func encodeDocument(_ document: YAMLJSONDocument) throws -> String
}
```

`YAMLJSONDocument` should contain only the JSON-compatible recursive value and any
conversion warnings needed by the workspace. Dependency diagnostics are translated
at this boundary. The workspace, History snapshot, and Registry definition never
store or expose Yams types.

Replacement therefore requires rewriting and retesting one adapter, not the
workspace/session/History behavior or the conversion policy. Independent fixtures
should test the adapter contract against published YAML examples and hand-reviewed
edge cases rather than asserting Yams-specific messages.

## Required verification before accepting the dependency

1. Pin `6.2.2` exactly in `Package.resolved` and run the project-list check after
   the Xcode project edit.
2. Prove that `yes`, `no`, `on`, and `off` remain strings while `true` and `false`
   become booleans.
3. Cover decimal, octal, hexadecimal, very large integers, fractions, exponent
   notation, negative zero, infinities, and NaN with explicit fidelity outcomes.
4. Cover anchors and aliases, undefined aliases, duplicate keys, non-string keys,
   explicit/custom tags, Unicode, accented text, and multi-scalar emoji.
5. Prove that a second YAML document is rejected with a located diagnostic.
6. Set and test an application input-size policy. Yams does not document a parser
   depth limit, so also exercise deeply nested block and flow collections and
   record the accepted bound. Do not claim the adapter is safe for arbitrary
   adversarial YAML without stronger resource isolation or a parser-level depth
   limit.
7. Compare the Release executable and `.app` sizes before and after integration,
   and include the MIT notice inventory.
8. Run `scripts/verify --full`; report current-host evidence separately from
   macOS 14/15 runtime evidence.

## Why not the alternatives

### PotentCodables

PotentCodables has the strongest out-of-the-box YAML 1.2 story of the Swift
candidates examined. Its YAML model retains aliases and anchors, its reader applies
an explicit schema, and parser failures expose line and column. Its writer can emit
YAML or JSON and sort keys. [Potent YAML model](https://github.com/outfoxx/PotentCodables/blob/3.5.3/Sources/PotentYAML/YAML.swift),
[reader](https://github.com/outfoxx/PotentCodables/blob/3.5.3/Sources/PotentYAML/YAMLReader.swift),
[serialization API](https://github.com/outfoxx/PotentCodables/blob/3.5.3/Sources/PotentYAML/YAMLSerialization.swift)

It is nevertheless disproportionate here. Its single package product includes
PotentCodables, PotentJSON, PotentCBOR, PotentASN1, PotentYAML, and Cfyaml, with
BigInt, swift-collections, Float16, and Regex dependencies. Its public
`YAMLSerialization.yaml(from:)` also turns multiple documents into one sequence,
so this Utility would need extra stream handling to preserve the single-document
contract. [PotentCodables package manifest](https://github.com/outfoxx/PotentCodables/blob/3.5.3/Package.swift),
[multi-document convenience behavior](https://github.com/outfoxx/PotentCodables/blob/3.5.3/Sources/PotentYAML/YAMLSerialization.swift#L38-L43)

### swift-yaml

swift-yaml 1.0.0 is attractive for a future replacement: its manifest has one
library target and no dependencies; it claims YAML 1.2.2 parsing, source positions,
anchors/aliases, multi-document parsing, and a configurable depth limit. However,
its documented public surface is composition only, so JSON-to-YAML would require a
new emitter. It also requires Swift 6.2 and reached 1.0 only in March 2026. This is
too little maintenance history for the first dependency choice when Yams can be
contained behind a replaceable seam. [swift-yaml README](https://github.com/1amageek/swift-yaml/blob/1.0.0/README.md),
[package manifest](https://github.com/1amageek/swift-yaml/blob/1.0.0/Package.swift),
[1.0 release](https://github.com/1amageek/swift-yaml/releases/tag/1.0.0)

## Decision summary

Adopt **Yams 6.2.2 exact** only after the implementation preserves these boundaries:

- syntax and emission in the Yams adapter;
- YAML 1.2 Core resolution and JSON fidelity in app-owned typed code;
- single-document parsing through `compose(..., .basic)`;
- no Yams types outside the adapter;
- explicit resource bounds and honest conformance wording; and
- final Release size, notice, full-gate, and host evidence recorded with the ticket.
