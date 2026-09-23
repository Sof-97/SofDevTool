# SofDevTool

SofDevTool 0.2.0 is a local native macOS Developer Toolbox built with Rust and GPUI. The application owns Utilities, History, preferences, Clipboard policy and lifecycle; `sofui` 0.1.0 supplies reusable GPUI components. No account or service is required to use the installed application.

## Development

Use an Apple Silicon Mac with macOS 14 or newer, the pinned Rust toolchain in `rust-toolchain.toml`, Xcode Command Line Tools and Python 3. The full renderer asset check also needs Node.js 18 or newer and npm. Run commands from this repository root:

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

`make install INSTALL_DESTINATION=/path/to/temporary/Applications` chooses another installation directory. The installer verifies Release identity and replaces only its `SofDevTool.app` bundle. The actual default home Applications installation and native relaunch acceptance are tracked separately; a build or temporary install alone does not prove them.

Debug uses `SofDevTool Debug` / `com.gerardocalia.sofdevtool.debug`; Release uses `SofDevTool` / `com.gerardocalia.sofdevtool`. Their bundles, preferences, History and saved controls have separate locations. `SOFDEVTOOL_RUST_SUPPORT_ROOT` overrides the complete Application Support data root for isolated checks; use a distinct temporary root per profile. Existing Swift product data is not read or migrated.

The application version comes from root `Cargo.toml`. `SOFDEVTOOL_BUILD_ID` can select a positive numeric bundle build identifier (default `1`). Each bundle records its exact Git revision, build channel and whether the working source differs from that revision; the application title also displays the same metadata. `CARGO_TARGET_DIR` is honored by Cargo and packaging. The bundle contains its icons and Text Diff resources locally, and running it requires neither Node nor network access.

The older Swift/Xcode tree remains in the checkout until the separate retirement step. It is not used by these root commands. macOS 14 is a build baseline; actual native behavior on macOS 14/15 requires separate runtime evidence.
