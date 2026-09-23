# Ticket 17 evidence: Text Diff asset pipeline

## Implementation

- Kept the editable main bridge entry, exact dependency versions and integrity
  hashes, esbuild recipe, upstream provenance, and retained-source MIT notice
  under `rust/crates/app/src/text_diff/assets-source/`.
- The checked-in lockfile's dependency graph is unchanged from the PierreDiffsSwift
  snapshot; only its root project-name metadata was changed. The original
  upstream lockfile SHA-256 was
  `19193f716a5db005ea31881bf228a266618a4ed147e6b25b647f0544393a51bc`; the
  Rust-owned lockfile SHA-256 is
  `84dfc103a798b4b150aee6cd913dc869847049b2cfe82b30220272126108e99a`.
- The release bundle contains the main read-only renderer only. Its generated
  inventory contains 30 packages and 511 module paths that contribute bytes to
  esbuild outputs; optional-editor-only packages and unrelated Swift/Yams
  inventory claims are excluded. Required entry-source attribution remains in
  the notice output.
- Bundle SHA-256:
  `86bd5b4699d852c32dbc51fb6bdbdde6dc6904459471809f57416edd67da1efb`.
- The renderer embeds HTML, notices, and JavaScript at compile time. CSP denies
  network connections and remote resources; Wry permits only the initial
  `about:blank` navigation. Runtime does not invoke Node/npm or fetch assets.

## Verification

Commands were run from `rust/` unless a working directory is stated.

- `npm ci --offline` in
  `rust/crates/app/src/text_diff/assets-source/` — passed; 56 packages
  installed, audit reported 0 vulnerabilities.
- `npm run build` — passed.
- `npm run verify` — passed; regenerated bundle, dependency inventory, and
  notices match checked-in assets.
- `CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target cargo test -p sofdevtool-app --all-targets`
  — passed; 89 library tests and the binary target (0 tests).
- `CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target cargo clippy -p sofdevtool-app --all-targets -- -D warnings`
  — passed.
- `rustfmt --edition 2021 --check crates/app/src/text_diff/renderer.rs`,
  `node --check crates/app/src/text_diff/assets-source/bundle.mjs`, and
  `node --check crates/app/src/text_diff/assets-source/src/diff-entry.js` —
  passed.

Renderer boundary tests cover exact UTF-8 JSON payloads including accented
text, flag and joined emoji sequences; the disclosed whole-line fallback for
complex emoji; idempotent/early readiness and pending latest-request behavior;
stale callback rejection; error recovery; inline-only resources/CSP; and local
navigation policy.

## Limits

No native app launch or WKWebView observation was performed. Verification is
offline build and automated boundary-test evidence; it does not claim native
visual, focus, or runtime behavior.
