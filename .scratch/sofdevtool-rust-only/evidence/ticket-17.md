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

## Coordinator integrated and native checks

Frozen `b7e28f0` passed the complete offline default gate: formatting, strict
all-target Clippy,313 tests (89 app,190 core,23 JSON,11 sofui), and Debug
app/gallery/preview builds. Independent `npm run verify` also passed. Logs and
metadata: `.artifacts/parallel-rust-only/wave6/`.

The uninstrumented Debug bundle ran on macOS26.2 arm64 from `/private/tmp`
with isolated support data and the existing `--text-diff-proof` startup option.
The actual WKWebView rendered split and unified comparisons with `caffè`,
`café`, joined skin-tone emoji, flag and keycap sequences intact, with the
disclosed whole-line fallback. Editing both inputs and changing mode produced
three settled entries; a fourth different comparison was then restored to the
prior exact old/new texts with four entries still retained. Switching to JSON
and back preserved the restored comparison. Native double-click selected the
word `caffè` inside the renderer; Cmd-C and paste into the GPUI updated editor
reproduced it exactly, proving selection/copy and focus return. The app quit
successfully. These observations do not replace failure/recovery or final
redesigned-release acceptance, and no OS14/15 runtime claim is made.

## Native failure and recovery — 2026-09-23

A temporary instrumented harness mounted the real `TextDiffWorkspace` from candidate `b7e28f0` and invoked its debug-only `simulate_renderer_failure` hook. The compiled binary SHA-256 was `47c6d9399acd2854b25c0734b53a27c92a97dc01daa46dc5e024d220be970679`; the temporary source SHA-256 was `b57ee89b2e374a93b88be01087c12f43d728ea6d5abd5f6bca4caf5237582533`. It used a unique temporary History directory and in-memory Clipboard, and ran on macOS 26.2 arm64.

Through Computer Use, **Simulate renderer failure** produced the visible actual renderer diagnostic `undefined is not an object (evaluating 'a.lang')` through the WebView IPC error path. Editing Updated to `caffè 👩🏽‍💻 🇮🇹 1️⃣` followed by `recovered after failure` removed the error, rendered the correct split comparison, retained the complex-emoji whole-line disclosure and recorded exactly one settled comparison. Cmd-Q terminated the harness, confirmed by process inspection. Screenshots are in the coordinator conversation.

This establishes the instrumented error/recovery path; it is distinct from the ordinary bundled application's native Unicode, restore, selection/Copy and focus checks above. The temporary harness was not added to the maintained app or installed Release. Final redesigned Release acceptance remains in ticket 22.
