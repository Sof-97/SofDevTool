# SofDevTool Rust + GPUI

The Rust rewrite of the Developer Toolbox, built alongside the existing Swift
application in the same repository. This directory is a self-contained Cargo
workspace; it never reads, migrates or writes Swift data.

## Crates and module ownership

| Crate | Path | Role |
| --- | --- | --- |
| `sofdevtool-core` | `crates/core` | GPUI-independent Utility contracts and domain engines. `utility.rs`/`session.rs` hold the shared `Utility` trait and revision-gated `Session<U>`; `diagnostic.rs` holds the shared diagnostic vocabulary; `utilities/<name>.rs` owns each Utility's request/result/snapshot and tests. |
| `sofdevtool-ui` | `crates/ui` | Owner-maintained GPUI component library: semantic theme tokens (`theme.rs`), `Button`, `HoldButton`, `LabeledField`, `TextEditor`, `TextField`, `panel`, `HistoryPanel`, diagnostics, copy feedback. Must not depend on either SofDevTool crate, on Utility IDs, app persistence, or macOS service policy. |
| `sofdevtool-app` | `crates/app` | The application: identity, Registry, Workbench, Launcher, Settings, preferences, fresh Rust History, and one workspace per Utility (`utilities/<name>.rs`, plus `json_workspace.rs` and `text_diff/`). Binary `sofdevtool`. |
| gallery | `crates/ui/examples/gallery.rs` | Executable demonstration of the exported components in their normal, focused, disabled, invalid, empty, list, hold, confirmation and theme states. Builds without the application crate. |

All fifteen Utilities are implemented and registered: JSON, YAML/JSON, Base64,
URL Encoding, Hashes, Identifier Generator (UUID/ULID/KSUID), Timestamps, JWT
Decoder, Regex, Case Conversion, Whitespace, Color Conversion, Sample Data,
Random String and Text Diff.

### Component consumption and extraction

The application consumes components only through the `sofdevtool_ui` public
surface; it never names `gpui-component` types directly. `sofdevtool-ui` wraps
`gpui-component`'s text-editing engine behind `TextEditor`/`TextField` and owns
everything else. Extraction to a separate repository is deferred until a second
consuming application makes the reusable boundary concrete; the crate already
has no SofDevTool dependencies, so extraction is a repository move rather than a
refactor.

## Toolchain and gate

Pinned in `rust-toolchain.toml`: Rust `1.98.1` (aarch64-apple-darwin) with
`rustfmt` and `clippy`.

```sh
rust/scripts/format            # cargo fmt --all
rust/scripts/verify            # fmt check, clippy -D warnings, tests, debug build
rust/scripts/verify --full     # adds a release build (the authoritative gate)
```

The gate runs each command with `set -euo pipefail`; the first failure is the
exit status and no step is hidden by a pipeline.

## Dependency provenance and licenses

Exact versions live in `Cargo.lock`. First-party crates depend only on:

| Dependency | Version | License | Why |
| --- | --- | --- | --- |
| `gpui-pre` (lib `gpui`) | `=0.3.6` | Apache-2.0 | GPUI snapshot of `zed-industries/zed`; the version the component ecosystem targets. |
| `gpui-pre-platform` | `=0.3.6` | Apache-2.0 | The `gpui_platform` entry point. |
| `gpui-component` | `=0.6.6` | Apache-2.0 | Used **only** for its text editing engine, wrapped by `sofdevtool-ui`. Not adopted as the visual system. |
| `unicode-segmentation` | `=1.13.3` | MIT OR Apache-2.0 | Grapheme boundaries for editor deletion and case segmentation. |
| `serde` / `serde_json` | `1.x` | MIT OR Apache-2.0 | Contract derives and the local Text Diff bridge. |
| `digest`, `sha1`, `sha2`, `md-5`, `hex` | pinned | MIT OR Apache-2.0 | Hash Utility. |
| `rand`, `uuid` | pinned | MIT OR Apache-2.0 | Identifier, random-string and sample-data generation. |
| `regex` | `=1.11.3` | MIT OR Apache-2.0 | The Rust regex dialect. |
| `chrono`, `chrono-tz` | pinned | MIT OR Apache-2.0 | Timestamps. |
| `saphyr`, `saphyr-parser` | `=0.0.3` | MIT OR Apache-2.0 | YAML 1.2 parsing; the event-level parser is required so duplicate keys and tags are rejected rather than silently accepted. |
| `wry` | `=0.53.5` | MIT OR Apache-2.0 | Embedded WKWebView for the Text Diff renderer. |

The JSON engine deliberately avoids `serde_json`'s `arbitrary_precision` mode.
`cargo` reports a future-incompatibility warning for the transitive `block
v0.1.6` crate (an `objc` dependency); it is not first-party code.

## Running

```sh
cargo run -p sofdevtool-app --bin sofdevtool   # the full Developer Toolbox
cargo run -p sofdevtool-ui --example gallery   # the component gallery
```

### Bundles

```sh
rust/scripts/bundle-debug     # -> rust/artifacts/SofDevToolRust.app (Debug)
rust/scripts/bundle-release   # -> rust/artifacts/SofDevToolRust.app (Release)
```

Both carry `CFBundleIdentifier com.gerardo.sofdevtool.rust` and
`LSMinimumSystemVersion 14.0`. The Release bundle additionally embeds a
generated `AppIcon.icns`, version `0.1.0` (build `1`) and
`THIRD_PARTY_NOTICES.md`. Neither installs, launches or notarizes anything.

### Owner installation and launch

1. `rust/scripts/bundle-release`
2. Move `rust/artifacts/SofDevToolRust.app` to `/Applications` (or anywhere) if
   you want it outside the checkout.
3. Launch it from Finder or `open /Applications/SofDevToolRust.app`.

The Rust app writes only under `~/Library/Application Support/SofDevToolRust`
(History and preferences) and the `com.gerardo.sofdevtool.rust` preference
domain. The Swift app's `~/Library/Application Support/SofDevTool` data and
`dev.gerardo.SofDevTool` preferences are never read or written. Installing or
replacing the Swift app is a separate owner action and is not performed here.

WebView assets and third-party notices are compiled into the binary, so the
packaged app resolves everything locally with no network access.

## macOS deployment

`.cargo/config.toml` sets `MACOSX_DEPLOYMENT_TARGET=14.0`, preserving the macOS
14 baseline; the linked binaries report `minos 14.0`. That is compatibility
evidence only. All native verification to date ran on macOS 26.2 (arm64);
**macOS 14 and 15 runtime remain unverified**.

## Verification evidence

Per-ticket native scenarios, commands, host details and honest limits live in
[`docs/evidence/`](docs/evidence/). Compilation and unit tests are never
presented as runtime evidence. The IME composition check remains a recorded
nonblocking evidence exception.
