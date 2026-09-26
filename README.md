# SofDevTool

SofDevTool 0.2.0 is a local native macOS Developer Toolbox built with Rust and GPUI. Its fifteen Utilities cover JSON, YAML/JSON, Base64, URL encoding, hashes, identifiers, timestamps, JWT inspection, regex, case and whitespace conversion, color, sample data, random strings, and Text Diff. No account or service is required to use the installed application.

The root Cargo workspace is the sole maintained product. [`crates/app`](crates/app) owns the Workbench, Launcher, application lifecycle, Clipboard, preferences, and History policy; [`crates/core`](crates/core) owns Utility behavior and validation. [`sofui`](crates/ui/README.md) is an independently versioned 0.1.0 GPUI component library intended for a later separate release. It owns reusable controls and their interaction logic, while each consumer owns its domain behavior and starts GPUI itself.

## Development

Use an Apple Silicon Mac with macOS 14 or newer, the pinned Rust toolchain in `rust-toolchain.toml`, Xcode Command Line Tools and Python 3. The full renderer asset check also needs Node.js 20 or newer and npm. Run commands from this repository root:

| Command | Action |
| --- | --- |
| `make help` | List maintained commands. |
| `make run` | Build and open `artifacts/SofDevTool Debug.app`. |
| `make gallery` | Open the independent `sofui` component gallery. |
| `make format` | Apply workspace Rust formatting. |
| `make verify` | Check formatting and Clippy, run tests, and build Debug app/gallery. |
| `make verify-full` | Add Release build, Text Diff asset reproduction and both bundles. |
| `make release` | Package `artifacts/SofDevTool.app`. |
| `make install` | Build and install Release in `~/Applications`. |

`make install INSTALL_DESTINATION=/path/to/temporary/Applications` chooses another installation directory. The installer verifies Release identity and replaces only its `SofDevTool.app` bundle. The default `~/Applications/SofDevTool.app` installation, launch and isolated-profile relaunch were exercised on macOS 26.2 arm64 with the corrected Release. The owner reported that physical Launcher activation from another app, another Space/full-screen context and literal Dock reopening after Cmd-W worked. Those three are manual reports, separate from coordinator-observed flows. This host evidence does not establish macOS 14/15 runtime, other display/DPI configurations or full IME composition.

Debug uses `SofDevTool Debug` / `com.gerardocalia.sofdevtool.debug`; Release uses `SofDevTool` / `com.gerardocalia.sofdevtool`. Their bundles, preferences, History and saved controls have separate locations. `SOFDEVTOOL_RUST_SUPPORT_ROOT` overrides the complete Application Support data root for isolated checks; use a distinct temporary root per profile. Existing Swift product data is not read or migrated.

The application version comes from root `Cargo.toml`. `SOFDEVTOOL_BUILD_ID` can select a positive numeric bundle build identifier (default `1`). Each bundle records its exact Git revision, build channel and whether the working source differs from that revision; the application title also displays the same metadata. `CARGO_TARGET_DIR` is honored by Cargo and packaging. The bundle contains its icons and Text Diff resources locally, and running it requires neither Node nor network access.

[`CONTEXT.md`](CONTEXT.md) defines the domain language. The owning Utility modules and tests contain current behavior and privacy policy; [contract vectors](crates/core/tests/retained_contract_vectors.rs) and [JSON fixtures](crates/core/fixtures/cases) retain independent expectations. The [Text Diff source and provenance](crates/app/src/text_diff/assets-source/README.md) and [app notices](platform/macos/APP_NOTICES.md) remain available without a Swift build.

The previous Swift/Xcode sources and build tooling have been retired from the maintained tree. Git history retains that implementation; existing installed apps and personal data are untouched. macOS 14 is a build baseline; actual native behavior on macOS 14/15 requires separate runtime evidence. Automated tests, a temporary installation, or a preview on another macOS version do not establish those runtime claims.
