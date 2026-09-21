# SofDevTool Rust + GPUI

The Rust rewrite of the Developer Toolbox, built alongside the existing Swift
application in the same repository. This directory is a self-contained Cargo
workspace; it does not read, migrate or write Swift data.

## Crates

| Crate | Path | Role |
| --- | --- | --- |
| `sofdevtool-core` | `crates/core` | GPUI-independent Utility contracts and domain engines. Owns the JSON request/result/diagnostic/snapshot types. |
| `sofdevtool-ui` | `crates/ui` | Owner-maintained GPUI component library: theme tokens, buttons, labelled fields, multiline editor, panels, diagnostics. Must not depend on either SofDevTool crate, on Utility IDs, app persistence, or macOS service policy. |
| `sofdevtool-app` | `crates/app` | The application: identity, composition, and the JSON/Text Diff workspaces. Binary `sofdevtool`. |
| gallery | `crates/ui/examples/gallery.rs` | Executable demonstration of the exported components. Builds without the application crate. |

## Toolchain

Pinned in `rust-toolchain.toml`:

- Rust `1.98.1` (aarch64-apple-darwin), `rustfmt` and `clippy`.

## Dependency provenance and licenses

Only the GPUI family used by `gpui-component` is pinned, as a single compatible
set. Exact versions live in `Cargo.lock`.

| Dependency | Version | License | Provenance / why |
| --- | --- | --- | --- |
| `gpui-pre` (lib `gpui`) | `=0.3.6` | Apache-2.0 | Snapshot of `zed-industries/zed` `gpui` `0.2.2` (`zed-rev bcf6582`). The GPUI version the component ecosystem is built against. |
| `gpui-pre-platform` | `=0.3.6` | Apache-2.0 | The `gpui_platform` entry point (`application()`), same snapshot. |
| `gpui-component` | `=0.6.6` | Apache-2.0 | `longbridge/gpui-kit`. Used **only** for its text editing engine, wrapped by `sofdevtool-ui`'s `TextEditor`/`TextField`. It is not adopted as the application's visual system: theme tokens and all other controls are owner-maintained. |
| `gpui-base` | `0.6.6` | Apache-2.0 | Transitive foundation of `gpui-component`. |
| `unicode-segmentation` | `=1.13.3` | MIT OR Apache-2.0 | Unicode extended-grapheme boundaries for `TextEditor` Backspace/Delete. This exact version was already present in the lockfile through the GPUI dependency graph; the direct pin makes that editor contract explicit. |
| `serde` / `serde_json` | `1.x` | MIT OR Apache-2.0 | Derives for contract types; `serde_json` also serializes the local Text Diff bridge requests. |

The JSON engine does **not** use `serde_json`'s `arbitrary_precision` mode: it
has a small literal-preserving parser, so numbers keep their exact spelling and
a user object keyed `$serde_json::private::Number` is never rewritten into a
number.

`cargo` reports a future-incompatibility warning for the transitive `block
v0.1.6` crate (an `objc` dependency). It is not first-party code and does not
affect the current toolchain.

## Gate

The authoritative Rust gate is `scripts/verify`; formatting is applied with
`scripts/format`. Both run with `set -euo pipefail`, so the first failing command
determines the exit status and no step is hidden by a pipeline.

```sh
rust/scripts/format            # cargo fmt --all
rust/scripts/verify            # fmt check, clippy -D warnings, tests, debug build
rust/scripts/verify --full     # adds a release build
```

## Running

```sh
cargo run -p sofdevtool-app --bin sofdevtool   # JSON and Text Diff technical workbench
cargo run -p sofdevtool-ui --example gallery   # the component gallery
```

Prebuilt Debug binaries (build/test host macOS 26.2, arm64):

- `target/debug/sofdevtool`
- `target/debug/examples/gallery`

To produce a locally identified Debug bundle (it is **not** installed, launched
or notarized):

```sh
rust/scripts/bundle-debug   # -> rust/artifacts/SofDevToolRust.app
```

The bundle carries `CFBundleIdentifier com.gerardo.sofdevtool.rust` and
`LSMinimumSystemVersion 14.0`. `WindowOptions.app_id` is a no-op on the macOS
backend, so the bundle's `Info.plist` — not an in-process constant — is what
identifies the application.

## macOS deployment

`.cargo/config.toml` sets `MACOSX_DEPLOYMENT_TARGET=14.0`, preserving the
migration specification's macOS 14 baseline. The linked binaries report
`minos 14.0` (verified with `otool`). That is compatibility evidence only: the
host that builds and runs the gate is macOS 26.2. macOS 14 and 15 runtime
verification remain pending until the gate and native scenarios run there.

## Embedded Text Diff

The technical workbench opens JSON by default and retains both JSON and Text
Diff sessions when switching with the sidebar. This is the ticket02 proof;
Launcher, History and the remaining catalog are subsequent slices.

Wry **0.53.5** (MIT OR Apache-2.0) hosts a WKWebView child inside the GPUI
window. Native types stay behind `crates/app/src/text_diff/renderer.rs`.
The checked-in Pierre bundle and the complete JavaScript license inventory in
`crates/app/src/text_diff/assets/THIRD_PARTY_NOTICES.md` are embedded in the
local HTML. CSP rejects remote subresources and navigation is restricted.
Readiness and completion callbacks are revision-gated; renderer errors appear
in the workspace. Complex emoji use a visibly disclosed whole-line fallback.

Native verification and remaining platform limits are recorded in
[the ticket02 evidence](docs/evidence/ticket-02.md). Compilation is not evidence
of native interaction or macOS14/15 runtime compatibility.
