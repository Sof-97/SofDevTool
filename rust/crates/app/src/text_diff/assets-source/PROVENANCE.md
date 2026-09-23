# Text Diff asset provenance

`src/diff-entry.js` is the editable main entry retained from the maintained
PierreDiffsSwift source snapshot at revision
`c2249d7890de957a96480711152d90a06fa1222b`. The upstream entry was
`ThirdParty/PierreDiffsSwift/scripts/src/diff-entry.js`; its wrapper snapshot
and provenance were documented in that tree before its later retirement.

The entry imports the exact npm release `@pierre/diffs` 1.3.5 and uses the
lockfile's Shiki 4.4.1 family. Package versions and integrity hashes are
retained from the upstream npm lockfile. The upstream lockfile SHA-256 before
changing its project-name metadata is
`19193f716a5db005ea31881bf228a266618a4ed147e6b25b647f0544393a51bc`. The build
recipe preserves the upstream main-bundle settings: esbuild IIFE, minified
production output, Safari 16 target and `process.env.NODE_ENV` set to
`production`. It builds only the read-only main bundle used by SofDevTool.

The owned entry changes only the host bridge contract needed by Rust/Wry:

- `postToSwift` is renamed `postToHost`; it sends JSON through Wry's
  `window.ipc.postMessage`.
- `renderDiff` requires the positive integer revision supplied by Rust and
  includes it in ready/error callbacks. UTF-8 text stays in JSON strings; it is
  not converted through `atob` or a Latin-1 string.

The generated `bundle-dependencies.json` lists the exact packages and source
modules contributing bytes to esbuild outputs. `../assets/THIRD_PARTY_NOTICES.md`
is generated from those packages' actual license files and includes the MIT
attribution for the retained JavaScript entry source. Packages used only by an
optional editor bundle or the old Swift application are excluded.

The HTML template and complete bundle are inline local resources. Its Content
Security Policy blocks network loads, and Wry rejects navigation after the
initial `about:blank` document. The runtime has no Node/npm or network
dependency.
